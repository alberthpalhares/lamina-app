//! Edições de fotos e lâminas (spec FQ, regras 1 a 8): funções puras sobre o projeto, chamadas
//! pela sessão dentro de [`crate::Session::edit`] para entrarem no histórico.

use lamina_core::{Crop, FrameContent, PhotoId, Project};
use lamina_layout::{LayoutOptions, MAX_PER_PAGE, layout_options};

use crate::EditorError;

pub use crate::removing::remove_photos;
pub use crate::spreads::{delete_spread, insert_spread, move_spread};

/// Só a lista na ordem: a 1ª opção dela é a mesma da lista completa, sem calcular as reordenadas.
/// Com a montagem do álbum (spec NP, regra 6).
fn first_only(gap: lamina_core::Mm, mounting: lamina_core::Mounting) -> LayoutOptions {
    LayoutOptions {
        gap,
        max_candidates: 12,
        reordered: false,
        mounting,
    }
}

fn photo_of(content: &FrameContent) -> Option<PhotoId> {
    match content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    }
}

/// As fotos dos quadros da lâmina, na ordem dos quadros, sem repetir.
pub fn spread_photos(project: &Project, index: usize) -> Vec<PhotoId> {
    let mut out = Vec::new();
    for f in project.spreads.get(index).map_or(&[][..], |s| &s.frames) {
        if let Some(p) = photo_of(&f.content)
            && !out.contains(&p)
        {
            out.push(p);
        }
    }
    out
}

/// Quantas fotos cabem na lâmina `index` (até 12 por página com conteúdo; spec PB, regra 7).
pub fn capacity(project: &Project, index: usize) -> usize {
    let active = project.content_pages(index);
    MAX_PER_PAGE * (usize::from(active.left) + usize::from(active.right))
}

/// Põe `photo` no quadro `frame` da lâmina `spread`, com o recorte no centro (regra 1).
pub fn place_photo(
    project: &mut Project,
    spread: usize,
    frame: usize,
    photo: PhotoId,
) -> Result<(), EditorError> {
    project.photo(photo)?;
    let f = project
        .spreads
        .get_mut(spread)
        .and_then(|s| s.frames.get_mut(frame))
        .ok_or(EditorError::NoFrame)?;
    f.content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    Ok(())
}

/// Troca as fotos de dois quadros da mesma lâmina; os quadros não mudam (regra 2).
pub fn swap_photos(
    project: &mut Project,
    spread: usize,
    a: usize,
    b: usize,
) -> Result<(), EditorError> {
    let s = project
        .spreads
        .get_mut(spread)
        .ok_or(EditorError::NoFrame)?;
    if a >= s.frames.len() || b >= s.frames.len() || a == b {
        return Err(EditorError::NoFrame);
    }
    let reset = |c: FrameContent| match c {
        FrameContent::Photo { photo, crop } => FrameContent::Photo {
            photo,
            crop: crate::suggestions::keep_center(crop),
        },
        other => other,
    };
    let (ca, cb) = (s.frames[a].content.clone(), s.frames[b].content.clone());
    s.frames[a].content = reset(cb);
    s.frames[b].content = reset(ca);
    Ok(())
}

/// Deixa o quadro vazio (regra 3).
pub fn clear_frame(project: &mut Project, spread: usize, frame: usize) -> Result<(), EditorError> {
    let f = project
        .spreads
        .get_mut(spread)
        .and_then(|s| s.frames.get_mut(frame))
        .ok_or(EditorError::NoFrame)?;
    f.content = FrameContent::Empty;
    Ok(())
}

/// Arranja a lâmina `index` só com `photos`, na 1ª opção do motor (sem fotos, fica vazia).
pub fn layout_with(
    project: &mut Project,
    index: usize,
    photos: &[PhotoId],
) -> Result<(), EditorError> {
    let id = project.spreads.get(index).ok_or(EditorError::NoFrame)?.id;
    let mut frames = Vec::new();
    if !photos.is_empty() {
        let aspects = photos
            .iter()
            .map(|p| Ok(project.photo(*p)?.aspect_ratio()))
            .collect::<Result<Vec<f64>, EditorError>>()?;
        let active = project.content_pages(index);
        let lists = layout_options(
            &project.spec,
            active,
            &aspects,
            &first_only(
                project.spread_gap(&project.spreads[index]),
                project.mounting,
            ),
        )?;
        let first = lists
            .in_order
            .into_iter()
            .next()
            .ok_or(EditorError::NoFrame)?;
        frames = photos.iter().copied().zip(first.frames).collect();
    }
    // A borda e os cantos ficam com as fotos da lâmina; numa lâmina sem fotos, as primeiras
    // ganham os do álbum (spec BR, regra 5).
    let (border, radius) = photo_style(project, index);
    // Os textos ficam; a lâmina volta ao motor (spec TX, regras 8 e 9).
    let spread = project.spread_mut(id)?;
    spread
        .frames
        .retain(|f| matches!(f.content, FrameContent::Text(_)));
    spread.manual = false;
    spread.model = None;
    for (photo, rect) in frames {
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        let frame = project.add_frame(id, rect, content)?;
        if let Some(f) = project
            .spread_mut(id)?
            .frames
            .iter_mut()
            .find(|f| f.id == frame)
        {
            (f.border, f.radius) = (border, radius);
        }
    }
    Ok(())
}

/// A borda e os cantos das fotos novas na lâmina `index`: os da primeira foto que está nela ou,
/// numa lâmina sem fotos, os do álbum (spec BR, regra 5).
pub fn photo_style(project: &Project, index: usize) -> (lamina_core::Border, lamina_core::Mm) {
    project
        .spreads
        .get(index)
        .and_then(|s| {
            s.frames
                .iter()
                .find(|f| matches!(f.content, FrameContent::Photo { .. }))
        })
        .map_or((project.look.border, project.look.radius), |f| {
            (f.border, f.radius)
        })
}

/// As fotos de cada lâmina, na ordem do álbum (a timeline; regra 8).
pub fn timeline(project: &Project) -> Vec<Vec<PhotoId>> {
    (0..project.spreads.len())
        .map(|i| spread_photos(project, i))
        .collect()
}

/// Move o corte entre as lâminas `k` e `k + 1`: `delta` < 0 passa as últimas fotos de `k` para o
/// começo de `k + 1`; `delta` > 0, as primeiras de `k + 1` para o fim de `k`. Para nos limites
/// (nenhuma lâmina vazia ou cheia demais). `false` se nada mudou.
pub fn move_cut(project: &mut Project, k: usize, delta: i32) -> Result<bool, EditorError> {
    if k + 1 >= project.spreads.len() || delta == 0 {
        return Ok(false);
    }
    let (mut a, mut b) = (spread_photos(project, k), spread_photos(project, k + 1));
    let want = usize::try_from(delta.unsigned_abs()).unwrap_or(usize::MAX);
    let n = if delta < 0 {
        want.min(a.len().saturating_sub(1))
            .min(capacity(project, k + 1).saturating_sub(b.len()))
    } else {
        want.min(b.len().saturating_sub(1))
            .min(capacity(project, k).saturating_sub(a.len()))
    };
    if n == 0 {
        return Ok(false);
    }
    if delta < 0 {
        let moved = a.split_off(a.len() - n);
        b.splice(0..0, moved);
    } else {
        let rest = b.split_off(n);
        a.extend(b);
        b = rest;
    }
    layout_with(project, k, &a)?;
    layout_with(project, k + 1, &b)?;
    Ok(true)
}
