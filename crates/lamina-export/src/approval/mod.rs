//! Aprovação do cliente (spec AC): imagens JPG leves, com o número da lâmina, para o fotógrafo mandar
//! ao cliente. Função e opções próprias, sem passar pela exportação de impressão (regra 2): nada aqui
//! lê `ExportOptions` nem `ExportPrefs`.

mod check;
mod clean;
mod code;
pub(crate) mod compose;
mod contact;
pub(crate) mod draw;
mod html;
mod plan;
mod text;

use std::path::{Path, PathBuf};

use lamina_core::{ColorSpace, Project};
use lamina_render::PhotoSource;

pub use check::{approval_photos_found, approval_warnings, html_cover_notice};
pub use code::proof_code;
pub use contact::{
    WHATSAPP_START, clean_whatsapp, client_name_problem, email_problem, outputs_problem,
    whatsapp_display,
};
pub use html::html_file_name;
pub use plan::{approval_existing, approval_names};
pub use text::{approval_question, approval_summary, watermark_problem};

use plan::{Job, canvas_of, jobs_of};

use crate::export::{at_once, refuse_name};
use crate::in_order::in_order;
use crate::write::write_atomic;
use crate::{ExportError, jpeg};

/// Qualidade do JPEG para web (regra 5). A de impressão não muda: é `DEFAULT_QUALITY`.
const WEB_QUALITY: u8 = 80;
/// Densidade gravada no arquivo (regra 5).
const WEB_DPI: u32 = 72;

/// O que a caixa pede. O tamanho, a qualidade e a cor são fixos: não são escolha do fotógrafo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalOptions {
    /// Intervalo de lâminas (ex.: "5, 12"), sem a capa e sem apagar nada (regra 10); `None` = tudo.
    pub range: Option<String>,
    /// A faixa com o número embaixo da imagem (regra 11).
    pub number: bool,
    /// O texto da marca-d'água (regra 12); `None` = desligada.
    pub watermark: Option<String>,
    /// Substituir o que já existe; sem intervalo, as sobras também saem (regra 14).
    pub overwrite: bool,
    /// Imagens JPG para web.
    pub jpg: bool,
    /// O álbum animado em HTML (spec AH); `None` = desligado.
    pub html: Option<ApprovalHtml>,
}

impl ApprovalOptions {
    /// A geração cobre o álbum inteiro: o intervalo só vale para os JPG.
    fn whole(&self) -> bool {
        self.range.is_none() || !self.jpg
    }
}

/// O que só o HTML pede (spec AH, seção 3): os contatos já vêm conferidos, pelas funções de `contact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalHtml {
    /// O nome que o cliente vê.
    pub name: String,
    /// Só os dígitos, com o país ([`clean_whatsapp`]).
    pub whatsapp: Option<String>,
    pub email: Option<String>,
}

impl Default for ApprovalOptions {
    fn default() -> Self {
        ApprovalOptions {
            range: None,
            number: true,
            watermark: None,
            overwrite: false,
            jpg: true,
            html: None,
        }
    }
}

/// O que foi gravado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalReport {
    /// Arquivos gravados, na ordem do álbum: a capa primeiro e o HTML por último.
    pub files: Vec<PathBuf>,
    pub bytes: u64,
    /// O código da prova (regra 20).
    pub code: String,
    /// Sobras de uma geração anterior, apagadas ao substituir.
    pub removed: Vec<PathBuf>,
    /// O peso do HTML, quando ele foi gerado.
    pub html_bytes: Option<u64>,
}

/// O que todo arquivo precisa para ser gerado.
struct Ctx<'a> {
    project: &'a Project,
    folder: &'a Path,
    photos: &'a dyn PhotoSource,
    code: &'a str,
    options: &'a ApprovalOptions,
}

/// O que sai de uma imagem desenhada.
struct Done {
    jpg: Option<(PathBuf, u64)>,
    html: Vec<html::Image>,
}

/// Desenha uma vez e monta o que foi pedido: as imagens do HTML saem da mesma imagem do JPG, sem a
/// faixa (spec AH, regra 3).
fn draw_job(ctx: &Ctx, job: &Job) -> Result<Done, ExportError> {
    let clean = draw::web_image(job.album, job.index, ctx.photos)?;
    let mark = ctx.options.watermark.as_deref();
    let html = if job.html {
        html::images_of(ctx.project, job, &clean, mark)?
    } else {
        Vec::new()
    };
    let jpg = match &job.jpg {
        Some(name) => Some(write_jpg(ctx, job, name, clean)?),
        None => None,
    };
    Ok(Done { jpg, html })
}

fn write_jpg(
    ctx: &Ctx,
    job: &Job,
    name: &str,
    clean: image::RgbaImage,
) -> Result<(PathBuf, u64), ExportError> {
    let band = ctx
        .options
        .number
        .then(|| compose::band_text(job.number, ctx.code));
    let image = compose::compose(clean, band.as_deref(), ctx.options.watermark.as_deref());
    let path = ctx.folder.join(name);
    let bytes = web_jpeg(image)?;
    write_atomic(ctx.folder, &path, &bytes)?;
    Ok((path, bytes.len() as u64))
}

/// O JPEG de toda imagem da aprovação (regra 5).
fn web_jpeg(image: image::RgbaImage) -> Result<Vec<u8>, ExportError> {
    jpeg::encode(image, WEB_QUALITY, WEB_DPI, ColorSpace::Srgb).map_err(ExportError::Encode)
}

/// Gera os arquivos em `folder`. `progress` recebe (feitos, total) a cada imagem pronta. Se uma
/// lâmina falha, o erro leva os arquivos que ficaram gravados, e nada fica pela metade.
pub fn export_approval(
    project: &Project,
    folder: &Path,
    photos: &dyn PhotoSource,
    options: &ApprovalOptions,
    progress: &dyn Fn(usize, usize),
) -> Result<ApprovalReport, ExportError> {
    let canvas = canvas_of(project, options)?;
    let jobs = jobs_of(project, canvas.as_ref(), options)?;
    if !options.overwrite
        && let Some(name) = approval_existing(project, folder, options)?
            .into_iter()
            .next()
    {
        return Err(refuse_name(name, folder));
    }
    std::fs::create_dir_all(folder).map_err(|e| ExportError::Folder {
        folder: folder.display().to_string(),
        reason: e.to_string(),
    })?;
    let code = proof_code(project)?;
    let ctx = Ctx {
        project,
        folder,
        photos,
        code: &code,
        options,
    };
    let total = jobs.len() + usize::from(options.html.is_some());
    let mut drawn = draw_all(&ctx, &jobs, &|done| progress(done, total))?;
    let html_bytes = write_html(&ctx, &mut drawn)?;
    if html_bytes.is_some() {
        progress(total, total);
    }
    let names = plan::approval_names(project, options)?;
    let removed = if options.overwrite && options.whole() {
        clean::remove_leftovers(folder, &names, options)?
    } else {
        Vec::new()
    };
    Ok(ApprovalReport {
        files: drawn.files,
        bytes: drawn.bytes + html_bytes.unwrap_or(0),
        code,
        removed,
        html_bytes,
    })
}

/// O que as imagens deixaram: os JPG gravados e as imagens do HTML, ainda na memória.
struct Drawn {
    files: Vec<PathBuf>,
    bytes: u64,
    html: Vec<html::Image>,
}

/// Grava o HTML, se ele foi pedido, e devolve o peso.
fn write_html(ctx: &Ctx, drawn: &mut Drawn) -> Result<Option<u64>, ExportError> {
    let Some(options) = &ctx.options.html else {
        return Ok(None);
    };
    let images = std::mem::take(&mut drawn.html);
    let page = html::page(ctx.project, options, ctx.code, images)?;
    let path = ctx.folder.join(html_file_name(&options.name));
    write_atomic(ctx.folder, &path, page.as_bytes())?;
    drawn.files.push(path);
    Ok(Some(page.len() as u64))
}

/// Até `at_once()` imagens ao mesmo tempo, gravadas na ordem do álbum (regra 18).
fn draw_all(ctx: &Ctx, jobs: &[Job], progress: &dyn Fn(usize)) -> Result<Drawn, ExportError> {
    let mut drawn = Drawn {
        files: Vec::with_capacity(jobs.len()),
        bytes: 0,
        html: Vec::new(),
    };
    let (mut done, mut failure) = (0, None);
    in_order(jobs, at_once(), &|job| draw_job(ctx, job), &mut |result| {
        match result {
            Ok(one) => {
                if let Some((path, size)) = one.jpg {
                    drawn.files.push(path);
                    drawn.bytes += size;
                }
                drawn.html.extend(one.html);
                done += 1;
                progress(done);
            }
            Err(e) => failure = failure.take().or(Some(e)),
        }
        failure.is_none()
    });
    match failure {
        Some(source) => Err(ExportError::Stopped {
            done: drawn.files,
            source: Box::new(source),
        }),
        None => Ok(drawn),
    }
}
