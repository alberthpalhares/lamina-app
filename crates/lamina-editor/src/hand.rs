//! Quadros à mão (spec TX): criar, apagar, mover, redimensionar e ordenar quadros, e mudar um
//! texto. As contas são em milímetros, a partir do corte; o projeto guarda frações (ADR 0003).

use lamina_core::{FrameContent, Mm, PageSide, Project, Rect, RelRect, Rgb, Spread, TextBox};

use crate::EditorError;

/// Menor quadro aceito, em mm (regra 6).
pub const MIN_FRAME_MM: f64 = 5.0;
/// Altura de um texto novo: duas linhas de 24 pt com entrelinha 1,2 (regra 11).
pub(crate) const NEW_TEXT_HEIGHT_MM: f64 = 2.0 * 24.0 * 1.2 * 25.4 / 72.0;

fn spread_mut(project: &mut Project, index: usize) -> Result<&mut Spread, EditorError> {
    project.spreads.get_mut(index).ok_or(EditorError::NoFrame)
}

/// O retângulo em mm de um quadro da lâmina.
pub fn frame_mm(project: &Project, index: usize, frame: usize) -> Option<Rect> {
    let f = project.spreads.get(index)?.frames.get(frame)?;
    Some(project.frame_rect(f))
}

/// Prende o retângulo ao corte e ao tamanho mínimo e o devolve em frações (regra 6).
pub fn fit_rect(project: &Project, r: Rect) -> Result<RelRect, EditorError> {
    let trim = project.spec.trim_rect();
    let (tw, th) = (trim.width.get(), trim.height.get());
    let w = r.width.get().clamp(MIN_FRAME_MM.min(tw), tw);
    let h = r.height.get().clamp(MIN_FRAME_MM.min(th), th);
    let fit = crate::group::fit;
    let x = fit(r.x.get(), trim.x.get(), trim.right().get() - w);
    let y = fit(r.y.get(), trim.y.get(), trim.bottom().get() - h);
    let rel = |v: f64, total: f64| (v / total).clamp(0.0, 1.0);
    Ok(RelRect::new(
        rel(x - trim.x.get(), tw),
        rel(y - trim.y.get(), th),
        rel(w, tw),
        rel(h, th),
    )?)
}

/// Um retângulo `w` × `h` mm no centro da página.
fn centered(project: &Project, side: PageSide, w: f64, h: f64) -> Rect {
    let page = project.spec.page_rect(side);
    Rect::new(
        Mm(page.x.get() + (page.width.get() - w) / 2.0),
        Mm(page.y.get() + (page.height.get() - h) / 2.0),
        Mm(w),
        Mm(h),
    )
}

/// A página de um quadro novo: a pedida, se existe nesta lâmina; senão, a que existe.
pub fn page_for(project: &Project, index: usize, wanted: Option<PageSide>) -> PageSide {
    let active = project
        .spec
        .edges()
        .active_pages(index, project.spreads.len());
    match wanted {
        Some(PageSide::Right) if active.right => PageSide::Right,
        Some(PageSide::Left) if active.left => PageSide::Left,
        _ if active.left => PageSide::Left,
        _ => PageSide::Right,
    }
}

/// A página de um quadro de foto novo: como `page_for`, sem as páginas em branco (spec PB,
/// regra 10).
pub fn photo_page_for(
    project: &Project,
    index: usize,
    wanted: Option<PageSide>,
) -> Result<PageSide, EditorError> {
    let room = project.content_pages(index);
    match wanted {
        Some(PageSide::Right) if room.right => Ok(PageSide::Right),
        Some(PageSide::Left) if room.left => Ok(PageSide::Left),
        _ if room.left => Ok(PageSide::Left),
        _ if room.right => Ok(PageSide::Right),
        _ => Err(EditorError::BlankPage),
    }
}

fn push(
    project: &mut Project,
    index: usize,
    rect: Rect,
    content: FrameContent,
) -> Result<usize, EditorError> {
    let rel = fit_rect(project, rect)?;
    if !is_text(&content) {
        crate::blank::check_photo_rect(project, index, &rel)?;
    }
    let id = spread_mut(project, index)?.id;
    project.add_frame(id, rel, content)?;
    Ok(project.spreads[index].frames.len() - 1)
}

/// Texto vazio no centro da página, com 60% da largura dela e a cor do fundo (regra 11).
pub fn add_text(project: &mut Project, index: usize, side: PageSide) -> Result<usize, EditorError> {
    let page = project.spec.page_rect(side);
    let rect = centered(project, side, page.width.get() * 0.6, NEW_TEXT_HEIGHT_MM);
    let background = spread_mut(project, index)?.page_background(side);
    push(
        project,
        index,
        rect,
        FrameContent::Text(TextBox::new(Rgb::text_on(background))),
    )
}

/// Quadro vazio de 1/3 da página, no centro dela; a lâmina fica à mão (regra 8).
pub fn add_empty_frame(
    project: &mut Project,
    index: usize,
    side: PageSide,
) -> Result<usize, EditorError> {
    let page = project.spec.page_rect(side);
    let rect = centered(
        project,
        side,
        page.width.get() / 3.0,
        page.height.get() / 3.0,
    );
    let at = push(project, index, rect, FrameContent::Empty)?;
    let spread = spread_mut(project, index)?;
    spread.manual = true;
    spread.model = None;
    Ok(at)
}

/// Um quadro desenhado com a paleta do modo organizar (spec EG, seção 3): texto (na cor que lê
/// sobre o fundo da página do meio do retângulo) ou quadro de foto vazio, que deixa a lâmina à mão.
pub fn add_frame_at(
    project: &mut Project,
    index: usize,
    rect: Rect,
    text: bool,
) -> Result<usize, EditorError> {
    let content = if text {
        let left = project.spec.page_rect(PageSide::Left);
        let center = rect.x.get() + rect.width.get() / 2.0;
        let side = if center < left.right().get() {
            PageSide::Left
        } else {
            PageSide::Right
        };
        let background = spread_mut(project, index)?.page_background(side);
        FrameContent::Text(TextBox::new(Rgb::text_on(background)))
    } else {
        FrameContent::Empty
    };
    let at = push(project, index, rect, content)?;
    if !text {
        let spread = spread_mut(project, index)?;
        spread.manual = true;
        spread.model = None;
    }
    Ok(at)
}

fn is_text(content: &FrameContent) -> bool {
    matches!(content, FrameContent::Text(_))
}

/// Apaga o quadro; se não era texto, a lâmina fica à mão.
pub fn remove_frame(project: &mut Project, index: usize, frame: usize) -> Result<(), EditorError> {
    let spread = spread_mut(project, index)?;
    if frame >= spread.frames.len() {
        return Err(EditorError::NoFrame);
    }
    let removed = spread.frames.remove(frame);
    if !is_text(&removed.content) {
        spread.manual = true;
        spread.model = None;
    }
    Ok(())
}

/// Põe o quadro em `rect` (mm), preso ao corte e ao mínimo. O recorte da foto guarda o centro,
/// então ele não muda (regra 10).
pub fn set_rect(
    project: &mut Project,
    index: usize,
    frame: usize,
    rect: Rect,
) -> Result<(), EditorError> {
    let rel = fit_rect(project, rect)?;
    let photo = project.spreads.get(index).and_then(|s| s.frames.get(frame));
    if photo.is_some_and(|f| !is_text(&f.content)) {
        crate::blank::check_photo_rect(project, index, &rel)?;
    }
    let spread = spread_mut(project, index)?;
    let f = spread.frames.get_mut(frame).ok_or(EditorError::NoFrame)?;
    f.rect = rel;
    if !is_text(&f.content) {
        spread.manual = true;
        spread.model = None;
    }
    Ok(())
}

/// Troca o quadro de foto com o próximo (`forward`) ou o anterior na ordem do desenho; os
/// textos ficam sempre por cima (regra 5). Devolve a posição nova.
pub fn reorder(
    project: &mut Project,
    index: usize,
    frame: usize,
    forward: bool,
) -> Result<usize, EditorError> {
    let frames = &mut spread_mut(project, index)?.frames;
    if frames.get(frame).is_none_or(|f| is_text(&f.content)) {
        return Err(EditorError::NoFrame);
    }
    let photo = |i: &usize| frames.get(*i).is_some_and(|f| !is_text(&f.content));
    let other = if forward {
        (frame + 1..frames.len()).find(|i| photo(i))
    } else {
        (0..frame).rev().find(|i| photo(i))
    };
    let other = other.ok_or(EditorError::NoFrame)?;
    frames.swap(frame, other);
    Ok(other)
}

/// Muda o texto do quadro com `f`.
pub fn update_text(
    project: &mut Project,
    index: usize,
    frame: usize,
    f: impl FnOnce(&mut TextBox),
) -> Result<(), EditorError> {
    let spread = spread_mut(project, index)?;
    match spread.frames.get_mut(frame).map(|fr| &mut fr.content) {
        Some(FrameContent::Text(text)) => {
            f(text);
            Ok(())
        }
        _ => Err(EditorError::NoFrame),
    }
}
