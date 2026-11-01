//! Fotos entre lâminas (spec ML, regras 1 a 4, 8 e 9): levar a foto de um quadro para um quadro de
//! outra lâmina (enche ou troca) ou para a lâmina, fora dos quadros (acrescenta). Funções puras
//! sobre o projeto, chamadas dentro de uma edição só: um Ctrl+Z volta as duas lâminas.

use lamina_core::{Crop, FrameContent, PhotoId, Project};

use crate::EditorError;
use crate::edits::{capacity, layout_with, spread_photos};
use crate::suggestions::keep_center;

/// Um quadro: a lâmina e a posição dele nela (a partir de 0).
pub type FrameAt = (usize, usize);

/// O que levar a foto para um quadro fez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moved {
    /// O destino estava vazio: a foto foi para ele e a origem ficou vazia (regra 1).
    Filled,
    /// O destino tinha foto: as duas trocaram de quadro (regra 2).
    Swapped,
}

fn content(project: &Project, (spread, frame): FrameAt) -> Result<&FrameContent, EditorError> {
    project
        .spreads
        .get(spread)
        .and_then(|s| s.frames.get(frame))
        .map(|f| &f.content)
        .ok_or(EditorError::NoFrame)
}

fn set(
    project: &mut Project,
    (spread, frame): FrameAt,
    c: FrameContent,
) -> Result<(), EditorError> {
    let f = project
        .spreads
        .get_mut(spread)
        .and_then(|s| s.frames.get_mut(frame))
        .ok_or(EditorError::NoFrame)?;
    f.content = c;
    Ok(())
}

/// A foto e o recorte do quadro `at`; quadro vazio ou de texto não tem foto para levar.
fn photo_in(project: &Project, at: FrameAt) -> Result<(PhotoId, Crop), EditorError> {
    match content(project, at)? {
        FrameContent::Photo { photo, crop } => Ok((*photo, *crop)),
        _ => Err(EditorError::NoFrame),
    }
}

/// Leva a foto do quadro `from` para o quadro de foto `to` de outra lâmina: enche (destino vazio)
/// ou troca (destino com foto), cada foto mantendo o centro do recorte com o zoom em 1. Quadro de
/// texto nunca recebe foto (regra 13).
pub fn move_to_frame(
    project: &mut Project,
    from: FrameAt,
    to: FrameAt,
) -> Result<Moved, EditorError> {
    let (photo, crop) = photo_in(project, from)?;
    let back = match content(project, to)? {
        FrameContent::Text(_) => return Err(EditorError::NoFrame),
        FrameContent::Empty => FrameContent::Empty,
        FrameContent::Photo { photo, crop } => FrameContent::Photo {
            photo: *photo,
            crop: keep_center(*crop),
        },
    };
    let moved = if matches!(back, FrameContent::Empty) {
        Moved::Filled
    } else {
        Moved::Swapped
    };
    let crop = keep_center(crop);
    set(project, to, FrameContent::Photo { photo, crop })?;
    set(project, from, back)?;
    Ok(moved)
}

/// Leva a foto do quadro `from` para a lâmina `to`, como uma foto a mais: o destino é refeito com
/// a 1ª opção do gerador para as fotos dele mais esta, os textos ficam; a origem só perde a foto
/// (regras 3 e 4). Recusa a lâmina cheia e a foto que já está nela (regras 8 e 9).
pub fn move_to_spread(project: &mut Project, from: FrameAt, to: usize) -> Result<(), EditorError> {
    let (photo, _) = photo_in(project, from)?;
    let mut photos = spread_photos(project, to);
    check_room(project, to, &photos, &[photo])?;
    photos.push(photo);
    set(project, from, FrameContent::Empty)?;
    layout_with(project, to, &photos)
}

/// As fotos `adding` cabem na lâmina `to`, que já tem `present`? Nenhuma nova: a foto já está lá
/// (regra 9); passou da capacidade: lâmina cheia (regra 8).
pub fn check_room(
    project: &Project,
    to: usize,
    present: &[PhotoId],
    adding: &[PhotoId],
) -> Result<usize, EditorError> {
    if to >= project.spreads.len() {
        return Err(EditorError::NoFrame);
    }
    let fresh = adding.iter().filter(|p| !present.contains(p)).count();
    if fresh == 0 {
        return Err(EditorError::AlreadyInSpread { spread: to });
    }
    let capacity = capacity(project, to);
    if capacity == 0 {
        return Err(EditorError::BlankPage);
    }
    if present.len() + fresh > capacity {
        return Err(EditorError::SpreadFull {
            spread: to,
            capacity,
        });
    }
    Ok(fresh)
}
