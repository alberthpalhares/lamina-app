//! A capa na exportação (spec CA, regras 13 e 14): os arquivos e a pré-impressão dela. A capa sai
//! pela projeção, do mesmo código das lâminas: mesma qualidade, cor, perfil e DPI.

use lamina_core::{FrameContent, Project, Spine};

use crate::jobs::{Output, SpreadJob};
use crate::preflight::{Warning, WarningKind, check};
use crate::{ExportError, ExportFormat};

/// O que fazer com a capa ao exportar (só faz diferença num álbum com capa).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoverExport {
    #[default]
    WithSpreads,
    Only,
    Without,
}

/// `capa.jpg` ou `capa.tif` (o PDF vai num arquivo próprio, pelo nome da projeção).
fn image_name(format: ExportFormat) -> String {
    match format {
        ExportFormat::Tiff => "capa.tif".to_owned(),
        _ => "capa.jpg".to_owned(),
    }
}

/// `<nome>-capa`, o nome do álbum na projeção: o PDF sai como `<nome>-capa.pdf`.
pub(crate) fn pdf_project_name(project: &Project) -> String {
    format!("{}-capa", project.name)
}

/// A projeção a exportar, com o nome do arquivo próprio; `None` sem capa ou com "Sem a capa".
pub(crate) fn canvas_of(
    project: &Project,
    wanted: CoverExport,
) -> Result<Option<Project>, ExportError> {
    if wanted == CoverExport::Without {
        return Ok(None);
    }
    let canvas = project.cover_canvas().map_err(ExportError::Cover)?;
    Ok(canvas.map(|mut canvas| {
        canvas.name = pdf_project_name(project);
        canvas
    }))
}

/// A capa inteira: nunca se divide em páginas (regra 13).
pub(crate) fn job(format: ExportFormat) -> SpreadJob {
    SpreadJob {
        spread: 0,
        output: Output::Whole(image_name(format)),
    }
}

/// Os avisos da capa (regra 14): os de sempre, mais o texto sobre a dobra, a lombada fixa cujo
/// álbum mudou e a face menor que a página. Vêm com a lâmina 0, antes das lâminas.
pub fn cover_preflight(project: &Project) -> Vec<Warning> {
    let Some(cover) = project.cover.as_ref() else {
        return Vec::new();
    };
    let Ok(Some(canvas)) = project.cover_canvas() else {
        return Vec::new();
    };
    let warn = |frame: usize, kind: WarningKind| Warning {
        spread: 0,
        frame,
        kind,
    };
    let mut out = Vec::new();
    let pages = project.page_count();
    if let Spine::Fixed { pages: fixed, .. } = cover.measures.spine
        && fixed != pages
    {
        out.push(warn(
            0,
            WarningKind::SpineOutdated {
                fixed_for: fixed,
                pages,
            },
        ));
    }
    if cover.measures.face_width.get() < project.spec.page_width().get() {
        out.push(warn(
            0,
            WarningKind::SmallFace {
                face: cover.measures.face_width,
                page: project.spec.page_width(),
            },
        ));
    }
    for (f, frame) in cover.content.frames.iter().enumerate() {
        for kind in cover_checks(&canvas, frame, pages, cover) {
            out.push(warn(f, kind));
        }
    }
    out
}

fn cover_checks(
    canvas: &Project,
    frame: &lamina_core::Frame,
    pages: usize,
    cover: &lamina_core::Cover,
) -> Vec<WarningKind> {
    let mut kinds = check(canvas, frame);
    if !matches!(frame.content, FrameContent::Text(_)) {
        return kinds;
    }
    let spine = cover.measures.spine_width(pages).get();
    let open = cover.measures.open_width(pages).get();
    let (x0, x1) = (
        frame.rect.x() * open,
        (frame.rect.x() + frame.rect.width()) * open,
    );
    let height = cover.measures.face_height.get();
    let (y0, y1) = (
        frame.rect.y() * height,
        (frame.rect.y() + frame.rect.height()) * height,
    );
    let edge = |a: f64, b: f64| (a - b).abs() < TOUCH_MM;
    let on_wrap = edge(x0, 0.0) || edge(x1, open) || edge(y0, 0.0) || edge(y1, height);
    let face = cover.measures.face_width.get();
    let in_spine = x0 >= face - TOUCH_MM && x1 <= face + spine + TOUCH_MM;
    // Na lombada, o texto é de propósito: só a dobra e o que a fonte não tem contam como aviso.
    if on_wrap || in_spine {
        kinds.retain(|k| !matches!(k, WarningKind::TextOutsideSafe { .. }));
    }
    if on_wrap {
        kinds.push(WarningKind::TextOnWrap);
    }
    kinds
}

/// Encostar no corte da capa é entrar na dobra.
const TOUCH_MM: f64 = 0.5;
