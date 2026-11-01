//! Exportação (spec `exportacao`): um JPEG por lâmina ou por página, gravado com segurança.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use std::borrow::Cow;

use lamina_core::{ColorSpace, FrameContent, Project};
use lamina_render::PhotoSource;

use crate::clean::remove_leftovers;
use crate::cover::{self, CoverExport};
use crate::cover_preflight;
use crate::in_order::in_order;
use crate::jobs::{SpreadJob, plan};
use crate::preflight::{Warning, preflight};
use crate::write::{self, Ctx};
use crate::{ExportError, ExportFormat, pdf};

/// Máximo de lâminas de impressão processadas ao mesmo tempo (~100 MB cada para a imagem, mais os
/// originais abertos; regra 7 e spec LV, regra 13). Codificar o JPEG é o passo mais lento e usa um
/// núcleo por lâmina.
const AT_ONCE: usize = 3;

/// Lâminas ao mesmo tempo: até `AT_ONCE`, nunca mais que os núcleos da máquina.
pub(crate) fn at_once() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(AT_ONCE))
}

/// Qualidade padrão do JPEG: 95, com cor em resolução total (4:4:4) (AGENTS.md, regra 14;
/// decisão do dono em 2026-10-15). Imperceptível em fotos (~48 dB numa foto real) com ~35% menos
/// espaço que a 100.
pub const DEFAULT_QUALITY: u8 = 95;

/// Opções da exportação; os padrões vêm da spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    /// Qualidade do JPEG, de 1 a 100.
    pub quality: u8,
    /// Intervalo de lâminas (ex.: "1, 3-4"); `None` = todas.
    pub range: Option<String>,
    /// Um arquivo por página em vez de um por lâmina.
    pub pages: bool,
    /// Substituir arquivos que já existem e apagar os que sobraram de uma exportação anterior
    /// (spec EI, regra 4).
    pub overwrite: bool,
    /// JPEG (um arquivo por lâmina ou página) ou PDF (um arquivo do álbum; spec PD).
    pub format: ExportFormat,
    /// Marcas de corte, só no PDF (spec PD, regra 6).
    pub crop_marks: bool,
    /// Espaço de cor da saída (spec CT). A fonte de fotos passada a [`export`] tem que entregar as
    /// fotos já nele (`lamina_media::original_source_in`).
    pub space: ColorSpace,
    /// A capa, num álbum que tem capa: junto, só ela ou sem ela (spec CA, regra 13).
    pub cover: CoverExport,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            quality: DEFAULT_QUALITY,
            range: None,
            pages: false,
            overwrite: false,
            format: ExportFormat::Jpeg,
            crop_marks: false,
            space: ColorSpace::Srgb,
            cover: CoverExport::default(),
        }
    }
}

/// O que foi gravado e os avisos da pré-impressão.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportReport {
    /// Arquivos gravados, na ordem do álbum.
    pub files: Vec<PathBuf>,
    pub warnings: Vec<Warning>,
    /// Arquivos de uma exportação anterior apagados ao substituir (spec EI, regra 4).
    pub removed: Vec<PathBuf>,
    pub elapsed: Duration,
}

/// Exporta o projeto para `folder`. `progress` recebe (feitos, total) a cada arquivo gravado.
pub fn export(
    project: &Project,
    folder: &Path,
    photos: &dyn PhotoSource,
    options: &ExportOptions,
    progress: &dyn Fn(usize, usize),
) -> Result<ExportReport, ExportError> {
    let started = Instant::now();
    let in_space = colors_in(project, options.space);
    let project = in_space.as_ref();
    let canvas = cover::canvas_of(project, options.cover)?;
    let only = cover_alone(project, options, canvas.as_ref());
    let jobs = if only {
        check_options(options)?
    } else {
        plan(project, options)?
    };
    let names = all_names(project, canvas.as_ref(), options, &jobs);
    if !options.overwrite {
        refuse_existing(&names, folder)?;
    }
    std::fs::create_dir_all(folder).map_err(|e| ExportError::Folder {
        folder: folder.display().to_string(),
        reason: e.to_string(),
    })?;
    let ctx = Ctx {
        project,
        folder,
        photos,
        quality: options.quality,
        space: options.space,
        format: options.format,
    };
    // A contagem do andamento vale para tudo: as lâminas e depois a capa.
    let pages: usize = jobs.iter().map(|j| j.names().len()).sum();
    let total = pages + usize::from(canvas.is_some());
    let album_progress = |done: usize, _: usize| progress(done, total);
    let mut files = match (options.format, only) {
        (_, true) => Vec::new(),
        (ExportFormat::Pdf, false) => vec![pdf::write(&ctx, &jobs, options, &album_progress)?],
        (_, false) => write_all(&ctx, &jobs, &album_progress)?,
    };
    if let Some(canvas) = &canvas {
        let cover_progress = |done: usize, _: usize| progress(pages + done, total);
        files.extend(write_cover(canvas, &ctx, options, &cover_progress)?);
    }
    // A limpeza só vale para os JPEGs e TIFFs (spec PD, regra 7; spec CT, regra 6).
    // "Só a capa" não apaga as lâminas da exportação anterior: elas continuam valendo.
    let removed = if options.overwrite && options.format != ExportFormat::Pdf && !only {
        let ext = if options.format == ExportFormat::Tiff {
            ".tif"
        } else {
            ".jpg"
        };
        remove_leftovers(folder, &names, ext)?
    } else {
        Vec::new()
    };
    let mut removed = removed;
    removed.extend(remove_old_cover_pdf(
        project,
        options,
        canvas.is_some(),
        folder,
    )?);
    let spreads: Vec<usize> = jobs.iter().map(|j| j.spread).collect();
    Ok(ExportReport {
        files,
        warnings: all_warnings(project, options, &spreads),
        removed,
        elapsed: started.elapsed(),
    })
}

/// O projeto com fundo e borda no espaço de cor da saída (spec CT, regra 3). Em sRGB, o próprio
/// projeto, sem cópia (nada muda: regra 7).
fn colors_in(project: &Project, space: ColorSpace) -> Cow<'_, Project> {
    if space == ColorSpace::Srgb {
        return Cow::Borrowed(project);
    }
    let mut copy = project.clone();
    let cover = copy.cover.as_mut().map(|c| &mut c.content);
    for spread in copy.spreads.iter_mut().chain(cover) {
        spread.background = space.from_srgb(spread.background);
        spread.background_right = spread.background_right.map(|c| space.from_srgb(c));
        for frame in &mut spread.frames {
            frame.border.color = space.from_srgb(frame.border.color);
            if let FrameContent::Text(text) = &mut frame.content {
                text.color = space.from_srgb(text.color);
            }
        }
    }
    Cow::Owned(copy)
}

/// Nomes dos arquivos que a exportação vai gravar, na ordem do álbum, sem gravar nada (para
/// quem precisa conferir a pasta antes, como o `lamina build`).
pub fn output_names(
    project: &Project,
    options: &ExportOptions,
) -> Result<Vec<String>, ExportError> {
    let canvas = cover::canvas_of(project, options.cover)?;
    let only = cover_alone(project, options, canvas.as_ref());
    let jobs = if only {
        check_options(options)?
    } else {
        plan(project, options)?
    };
    Ok(all_names(project, canvas.as_ref(), options, &jobs))
}

/// Sai só a capa: pedido, ou porque o álbum ainda não tem lâminas.
fn cover_alone(project: &Project, options: &ExportOptions, canvas: Option<&Project>) -> bool {
    canvas.is_some() && (options.cover == CoverExport::Only || project.spreads.is_empty())
}

/// "Só a capa": confere as opções, sem lâminas (spec CA, regra 13).
fn check_options(options: &ExportOptions) -> Result<Vec<SpreadJob>, ExportError> {
    if !(1..=100).contains(&options.quality) {
        return Err(ExportError::Quality(options.quality));
    }
    Ok(Vec::new())
}

/// Os nomes das lâminas e os da capa.
fn all_names(
    project: &Project,
    canvas: Option<&Project>,
    options: &ExportOptions,
    jobs: &[SpreadJob],
) -> Vec<String> {
    let only = cover_alone(project, options, canvas);
    let mut names = if only {
        Vec::new()
    } else {
        names_of(project, options, jobs)
    };
    if let Some(canvas) = canvas {
        names.extend(names_of(canvas, options, &[cover::job(options.format)]));
    }
    names
}

/// Grava a capa pela projeção, com as mesmas opções das lâminas (spec CA, regra 13).
fn write_cover(
    canvas: &Project,
    album: &Ctx,
    options: &ExportOptions,
    progress: &dyn Fn(usize, usize),
) -> Result<Vec<PathBuf>, ExportError> {
    let ctx = Ctx {
        project: canvas,
        folder: album.folder,
        photos: album.photos,
        quality: album.quality,
        space: album.space,
        format: album.format,
    };
    let jobs = [cover::job(options.format)];
    match options.format {
        ExportFormat::Pdf => Ok(vec![pdf::write(&ctx, &jobs, options, progress)?]),
        _ => write_all(&ctx, &jobs, progress),
    }
}

/// Substituir sem gravar a capa apaga o `<nome>-capa.pdf` da exportação anterior (CA-B10).
fn remove_old_cover_pdf(
    project: &Project,
    options: &ExportOptions,
    wrote_cover: bool,
    folder: &Path,
) -> Result<Vec<PathBuf>, ExportError> {
    let old = folder.join(format!("{}.pdf", cover::pdf_project_name(project)));
    let stale = options.overwrite && !wrote_cover;
    if !stale || !old.exists() {
        return Ok(Vec::new());
    }
    std::fs::remove_file(&old).map_err(|e| ExportError::Folder {
        folder: folder.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(vec![old])
}

/// Os avisos do que saiu: os da capa antes dos das lâminas (spec CA, regra 14).
fn all_warnings(project: &Project, options: &ExportOptions, spreads: &[usize]) -> Vec<Warning> {
    let mut warnings = Vec::new();
    if options.cover != CoverExport::Without {
        warnings.extend(cover_preflight(project));
    }
    warnings.extend(preflight(project, spreads));
    warnings
}

/// Os arquivos de saída: os JPEGs de cada lâmina, ou o PDF do álbum.
fn names_of(project: &Project, options: &ExportOptions, jobs: &[SpreadJob]) -> Vec<String> {
    match options.format {
        ExportFormat::Pdf => vec![pdf::file_name(project)],
        _ => jobs
            .iter()
            .flat_map(|j| j.names().into_iter().map(str::to_owned).collect::<Vec<_>>())
            .collect(),
    }
}

/// Nada é gravado se algum arquivo de saída já existe (regra 5).
fn refuse_existing(names: &[String], folder: &Path) -> Result<(), ExportError> {
    match names.iter().find(|name| folder.join(name).exists()) {
        Some(name) => Err(ExportError::Exists {
            name: name.clone(),
            folder: folder.display().to_string(),
        }),
        None => Ok(()),
    }
}

/// Grava até `at_once()` lâminas ao mesmo tempo. Se uma falha, o erro leva os arquivos que ficaram
/// gravados (inclusive os das outras lâminas que estavam em andamento).
fn write_all(
    ctx: &Ctx,
    jobs: &[SpreadJob],
    progress: &dyn Fn(usize, usize),
) -> Result<Vec<PathBuf>, ExportError> {
    let total: usize = jobs.iter().map(|j| j.names().len()).sum();
    let mut files = Vec::with_capacity(total);
    let mut failure = None;
    in_order(
        jobs,
        at_once(),
        &|job| write::spread(ctx, job),
        &mut |result| {
            match result {
                Ok(paths) => {
                    for path in paths {
                        files.push(path);
                        progress(files.len(), total);
                    }
                }
                Err(e) => failure = failure.take().or(Some(e)),
            }
            failure.is_none()
        },
    );
    match failure {
        Some(source) => Err(ExportError::Stopped {
            done: files,
            source: Box::new(source),
        }),
        None => Ok(files),
    }
}
