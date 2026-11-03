//! O que a caixa confere antes de gerar (spec AC, regras 15 e 16).

use lamina_core::{FrameContent, Project};

use super::ApprovalOptions;
use super::plan::{canvas_of, chosen, jobs_of};
use crate::cover_preflight;
use crate::preflight::{Warning, WarningKind, preflight};
use crate::{ExportError, ProjectFileError};

/// Os avisos que mudam o que o cliente vê (regra 15). Ampliação, zona segura, lombada e medidas da
/// capa são assunto da impressão.
pub fn approval_warnings(
    project: &Project,
    options: &ApprovalOptions,
) -> Result<Vec<Warning>, ExportError> {
    let spreads = if options.html.is_some() {
        (0..project.spreads.len()).collect()
    } else {
        chosen(project, options)?
    };
    let mut warnings = if options.whole() || options.html.is_some() {
        cover_preflight(project)
    } else {
        Vec::new()
    };
    warnings.extend(preflight(project, &spreads));
    warnings.retain(|warning| {
        matches!(
            warning.kind,
            WarningKind::EmptyFrame
                | WarningKind::TextOverflow { .. }
                | WarningKind::MissingChar { .. }
                | WarningKind::UnknownFont { .. }
        )
    });
    Ok(warnings)
}

/// O aviso de que o HTML vai sair com a capa lisa, porque o álbum não tem capa (spec AH, regra 8).
pub fn html_cover_notice(project: &Project, options: &ApprovalOptions) -> Option<&'static str> {
    (options.html.is_some() && project.cover.is_none()).then_some(
        "O álbum ainda não tem capa. No HTML, o cliente vê uma capa lisa escrita \"Capa ainda não criada\".",
    )
}

/// A primeira foto usada no que vai sair que não está no disco (regra 16): a caixa chama antes de
/// gerar. Um intervalo inválido não é assunto daqui: a geração o recusa em seguida.
pub fn approval_photos_found(
    project: &Project,
    options: &ApprovalOptions,
) -> Result<(), ProjectFileError> {
    let Ok(canvas) = canvas_of(project, options) else {
        return Ok(());
    };
    let Ok(jobs) = jobs_of(project, canvas.as_ref(), options) else {
        return Ok(());
    };
    let frames = jobs
        .iter()
        .filter_map(|job| job.album.spreads.get(job.index))
        .flat_map(|spread| &spread.frames);
    for frame in frames {
        let FrameContent::Photo { photo, .. } = &frame.content else {
            continue;
        };
        if let Ok(photo) = project.photo(*photo)
            && !photo.path.exists()
        {
            let name = photo.path.file_name().unwrap_or(photo.path.as_os_str());
            return Err(ProjectFileError::MissingPhoto {
                name: name.to_string_lossy().into_owned(),
                path: photo.path.display().to_string(),
            });
        }
    }
    Ok(())
}
