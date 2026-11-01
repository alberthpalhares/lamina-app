//! Buscas das ferramentas: a sessão, a lâmina, o quadro e a foto pelos números da tela (a partir
//! de 1), com as recusas da spec IA (seção 6), e a edição numa lâmina.

use lamina_core::{FrameContent, PhotoId, Project};

use super::ToolError;
use crate::{Editor, EditorError, Session};

pub(super) fn session(editor: &Editor) -> Result<&Session, ToolError> {
    editor.session().ok_or(ToolError::NoAlbum)
}

pub(super) fn session_mut(editor: &mut Editor) -> Result<&mut Session, ToolError> {
    editor.session_mut().ok_or(ToolError::NoAlbum)
}

/// A lâmina `lamina` (a partir de 1) como posição a partir de 0.
pub(super) fn spread_index(project: &Project, lamina: usize) -> Result<usize, ToolError> {
    let count = project.spreads.len();
    match lamina.checked_sub(1) {
        Some(index) if index < count => Ok(index),
        _ => Err(ToolError::NoSpread {
            asked: lamina,
            count,
        }),
    }
}

/// O quadro `quadro` (a partir de 1) da lâmina `index` como posição a partir de 0.
pub(super) fn frame_index(
    project: &Project,
    index: usize,
    quadro: usize,
) -> Result<usize, ToolError> {
    let count = project.spreads.get(index).map_or(0, |s| s.frames.len());
    match quadro.checked_sub(1) {
        Some(frame) if frame < count => Ok(frame),
        _ => Err(ToolError::NoFrame {
            spread: index + 1,
            count,
            asked: quadro,
        }),
    }
}

/// O quadro `quadro` é de foto (vazio ou com foto)? Texto recusa (seção 6).
pub(super) fn photo_frame(
    project: &Project,
    index: usize,
    quadro: usize,
) -> Result<usize, ToolError> {
    let frame = frame_index(project, index, quadro)?;
    match project.spreads[index].frames[frame].content {
        FrameContent::Text(_) => Err(ToolError::TextFrame {
            spread: index + 1,
            frame: quadro,
        }),
        _ => Ok(frame),
    }
}

/// A foto de número `n` (o mesmo de `listar_fotos`).
pub(super) fn photo_id(project: &Project, n: u64) -> Result<PhotoId, ToolError> {
    project
        .photos
        .iter()
        .find(|p| p.id.get() == n)
        .map(|p| p.id)
        .ok_or(ToolError::NoPhoto(n))
}

/// Uma edição na lâmina `index`, que passa a ser a atual: o rótulo ganha "· lâmina N".
pub(super) fn edit_on(
    editor: &mut Editor,
    index: usize,
    label: &str,
    change: impl FnOnce(&mut Project) -> Result<(), EditorError>,
) -> Result<(), ToolError> {
    // Com a capa aberta, a edição é nela: não há lâmina para onde ir (spec CA, regra 15).
    if !editor.on_cover() {
        editor.go_to_spread(index);
    }
    session_mut(editor)?.edit(label, change)?;
    Ok(())
}
