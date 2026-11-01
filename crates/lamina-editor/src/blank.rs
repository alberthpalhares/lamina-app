//! Páginas em branco (spec PB): marcar, desmarcar e o que acontece com as fotos da página. Funções
//! puras sobre o projeto, chamadas dentro de uma edição da sessão.

use lamina_core::{FrameContent, PageSide, PhotoId, Project, RelRect};

use crate::EditorError;
use crate::edits::{capacity, layout_with, spread_photos};

/// A frase de quando uma foto não pode entrar (regra 7).
pub const BLANK_SPREAD: &str = "Esta lâmina está em branco. Desmarque uma página para pôr fotos.";

const EPS: f64 = 1e-6;

/// O retângulo pega a página `side`?
fn touches(rect: &RelRect, side: PageSide) -> bool {
    match side {
        PageSide::Left => rect.x() < 0.5 - EPS,
        PageSide::Right => rect.x() + rect.width() > 0.5 + EPS,
    }
}

/// Um quadro de foto em `rect` pode ficar na lâmina `index`? Não numa página em branco (regra 10).
pub fn check_photo_rect(
    project: &Project,
    index: usize,
    rect: &RelRect,
) -> Result<(), EditorError> {
    let blank = project
        .spreads
        .get(index)
        .ok_or(EditorError::NoFrame)?
        .blank;
    let hits = [PageSide::Left, PageSide::Right]
        .iter()
        .any(|side| blank.has(*side) && touches(rect, *side));
    if hits {
        return Err(EditorError::BlankPage);
    }
    Ok(())
}

/// "4 fotos voltaram ao painel", "1 foto voltou ao painel".
pub fn returned_text(n: usize) -> String {
    if n == 1 {
        "1 foto voltou ao painel".to_owned()
    } else {
        format!("{n} fotos voltaram ao painel")
    }
}

/// Marca ou desmarca a página `side` da lâmina `index`. Marcar uma página com fotos as rearranja no
/// que sobra (regra 4); devolve quantas voltaram ao painel. Desmarcar não mexe em nada (regra 5).
pub fn set_blank(
    project: &mut Project,
    index: usize,
    side: PageSide,
    blank: bool,
) -> Result<usize, EditorError> {
    let active = project
        .spec
        .edges()
        .active_pages(index, project.spreads.len());
    let exists = match side {
        PageSide::Left => active.left,
        PageSide::Right => active.right,
    };
    if !exists {
        return Err(EditorError::NoFrame);
    }
    let spread = project.spreads.get_mut(index).ok_or(EditorError::NoFrame)?;
    spread.blank.set(side, blank);
    if !blank {
        return Ok(0);
    }
    let photo_there = spread
        .frames
        .iter()
        .any(|f| matches!(f.content, FrameContent::Photo { .. }) && touches(&f.rect, side));
    if !photo_there {
        // Só quadros vazios na página: saem, e o resto da lâmina fica como está.
        spread
            .frames
            .retain(|f| !(matches!(f.content, FrameContent::Empty) && touches(&f.rect, side)));
        return Ok(0);
    }
    let photos = spread_photos(project, index);
    let keep = fitting(project, &photos, capacity(project, index));
    layout_with(project, index, &keep)?;
    Ok(photos.len() - keep.len())
}

/// As fotos que cabem em `room`, na ordem de `photos`: saem as últimas pela captura (regra 4).
fn fitting(project: &Project, photos: &[PhotoId], room: usize) -> Vec<PhotoId> {
    let mut by_capture = photos.to_vec();
    by_capture.sort_by_key(|id| {
        let taken = project.photo(*id).ok().and_then(|p| p.taken_at);
        (taken.is_none(), taken, *id)
    });
    by_capture.truncate(room);
    photos
        .iter()
        .copied()
        .filter(|p| by_capture.contains(p))
        .collect()
}

/// Sem página com conteúdo que sobre, a marca sai: as fotos não somem do álbum ao mudar de lugar.
pub fn free_stuck_photos(project: &mut Project, index: usize, photos: &[PhotoId]) {
    let room = project.content_pages(index);
    if !room.left && !room.right && !photos.is_empty() {
        project.spreads[index].blank = lamina_core::BlankPages::default();
    }
}

/// Limpa a marca dos lados que deixaram de existir na posição de cada lâmina (regra 3).
pub fn clear_lost_marks(project: &mut Project) {
    let edges = project.spec.edges();
    let n = project.spreads.len();
    for (i, spread) in project.spreads.iter_mut().enumerate() {
        let active = edges.active_pages(i, n);
        spread.blank.left &= active.left;
        spread.blank.right &= active.right;
    }
}

#[cfg(test)]
mod tests {
    use lamina_core::{AlbumSpec, Dpi, Mm};

    use super::*;

    fn album() -> Project {
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_spread();
        project.add_spread();
        project
    }

    #[test]
    fn quadro_de_foto_nao_entra_na_pagina_em_branco() {
        let mut project = album();
        project.spreads[0].blank.left = true;
        let left = RelRect::new(0.1, 0.1, 0.2, 0.2).unwrap();
        let right = RelRect::new(0.6, 0.1, 0.2, 0.2).unwrap();
        let across = RelRect::new(0.4, 0.1, 0.2, 0.2).unwrap();
        assert!(matches!(
            check_photo_rect(&project, 0, &left),
            Err(EditorError::BlankPage)
        ));
        assert!(matches!(
            check_photo_rect(&project, 0, &across),
            Err(EditorError::BlankPage)
        ));
        check_photo_rect(&project, 0, &right).unwrap();
    }

    #[test]
    fn frase_das_fotos_que_voltaram_tem_singular_e_plural() {
        assert_eq!(returned_text(1), "1 foto voltou ao painel");
        assert_eq!(returned_text(4), "4 fotos voltaram ao painel");
    }

    #[test]
    fn marcar_lado_que_nao_existe_e_recusado() {
        let mut project = album();
        project.spec = project.spec.with_edges(
            lamina_core::AlbumEdge::SinglePage,
            lamina_core::AlbumEdge::FullSpread,
        );
        assert!(matches!(
            set_blank(&mut project, 0, PageSide::Left, true),
            Err(EditorError::NoFrame)
        ));
    }
}
