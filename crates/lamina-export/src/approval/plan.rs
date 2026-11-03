//! Que arquivos a aprovação gera (spec AC, regras 7, 9, 10 e 14), sem gravar nada.

use std::path::Path;

use lamina_core::Project;

use super::html::html_file_name;
use super::{ApprovalOptions, clean, draw};
use crate::range::parse_range;
use crate::{ExportError, ExportFormat, numbered};

pub(super) const COVER_NAME: &str = "web-capa.jpg";

/// Uma imagem a desenhar: a lâmina `index` de `album`, que é o projeto ou a projeção da capa. Ela vira
/// um JPG, imagens do HTML, ou os dois.
pub(super) struct Job<'a> {
    /// O arquivo JPG, quando ele foi pedido.
    pub jpg: Option<String>,
    /// Ela entra no HTML (spec AH, regra 3).
    pub html: bool,
    pub album: &'a Project,
    pub index: usize,
    /// O número da lâmina (começando em 1); `None` = a capa.
    pub number: Option<usize>,
}

/// As lâminas dos JPG (começando em 0), na ordem do álbum. O intervalo não vale para o HTML, que sai
/// sempre com o álbum inteiro (spec AH, seção 3).
pub(super) fn chosen(
    project: &Project,
    options: &ApprovalOptions,
) -> Result<Vec<usize>, ExportError> {
    let total = project.spreads.len();
    match &options.range {
        Some(text) if options.jpg => parse_range(text, total),
        _ => Ok((0..total).collect()),
    }
}

/// A projeção da capa, quando alguma saída a usa: no JPG, a capa só sai com o álbum inteiro (regra 10).
pub(super) fn canvas_of(
    project: &Project,
    options: &ApprovalOptions,
) -> Result<Option<Project>, ExportError> {
    if options.html.is_none() && (!options.jpg || options.range.is_some()) {
        return Ok(None);
    }
    project.cover_canvas().map_err(ExportError::Cover)
}

/// O que desenhar: a capa e as lâminas que têm alguma página (regra 7).
pub(super) fn jobs_of<'a>(
    project: &'a Project,
    canvas: Option<&'a Project>,
    options: &ApprovalOptions,
) -> Result<Vec<Job<'a>>, ExportError> {
    let total = project.spreads.len();
    let html = options.html.is_some();
    let picked = chosen(project, options)?;
    let cover = canvas.map(|album| Job {
        jpg: (options.jpg && options.whole()).then(|| COVER_NAME.to_owned()),
        html,
        album,
        index: 0,
        number: None,
    });
    let spreads = (0..total)
        .filter(|&index| draw::visible_region(project, index).is_some())
        .map(|index| Job {
            jpg: (options.jpg && picked.contains(&index))
                .then(|| numbered("web-lamina", index + 1, total, ExportFormat::Jpeg)),
            html,
            album: project,
            index,
            number: Some(index + 1),
        });
    let jobs: Vec<Job> = cover
        .into_iter()
        .chain(spreads)
        .filter(|job| job.jpg.is_some() || job.html)
        .collect();
    if jobs.is_empty() || (html && total == 0) {
        return Err(ExportError::NoSpreads);
    }
    Ok(jobs)
}

/// Os nomes dos arquivos que a geração grava, na ordem do álbum, sem gravar nada: os JPG e o HTML.
pub fn approval_names(
    project: &Project,
    options: &ApprovalOptions,
) -> Result<Vec<String>, ExportError> {
    let canvas = canvas_of(project, options)?;
    let jobs = jobs_of(project, canvas.as_ref(), options)?;
    let html = options.html.as_ref().map(|h| html_file_name(&h.name));
    Ok(jobs
        .into_iter()
        .filter_map(|job| job.jpg)
        .chain(html)
        .collect())
}

/// O que a geração substituiria, para a caixa perguntar (regra 14). Sem intervalo, conta tudo o que
/// é da aprovação: de 100 para 99 lâminas nenhum nome se repete (`001` e `01`), e as sobras sairiam.
pub fn approval_existing(
    project: &Project,
    folder: &Path,
    options: &ApprovalOptions,
) -> Result<Vec<String>, ExportError> {
    let names = approval_names(project, options)?;
    if options.whole() {
        return clean::approval_files(folder, options);
    }
    Ok(names
        .into_iter()
        .filter(|name| folder.join(name).exists())
        .collect())
}
