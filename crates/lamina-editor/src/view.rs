//! A visão da lâmina (spec JP, regra 5): a lâmina em pixels da tela, centralizada na maior escala
//! que cabe, com cada quadro onde vai sair impresso e a mesma parte da foto que a renderização
//! usa (as funções `bleed` e `region` do `lamina-render`).

use lamina_core::{FrameContent, FrameId, PageSide, PhotoId, Project, Rect, Rgb, TextBox};
use lamina_render::{bleed, region};

/// Retângulo em pixels da tela (com fração: a janela desenha sem arredondar).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PxRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// O que um quadro mostra na tela.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameKind {
    Empty,
    Photo {
        id: PhotoId,
        /// Parte da foto mostrada, em frações da foto inteira (0 a 1): x, y, largura, altura.
        source: PxRect,
        /// A foto não está no caminho salvo (regra 11).
        missing: bool,
    },
    /// Quadro de texto: a janela pede a imagem dele ao `lamina-render` (spec TX).
    Text {
        text: TextBox,
        /// Largura e altura do quadro em mm.
        size_mm: (f64, f64),
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameView {
    /// O quadro do projeto: as vagas da tela casam por ele (spec MV, regra 4).
    pub id: FrameId,
    /// Quadro já estendido até a sangria quando encosta no corte, como na impressão.
    pub rect: PxRect,
    pub kind: FrameKind,
    /// O quadro no corte final, onde a borda é desenhada (spec AP, regra 4).
    pub trim: PxRect,
    /// Espessura da borda (0 = sem borda) e cor.
    pub border: f64,
    pub border_color: Rgb,
    /// Raio de cada canto: cima-esquerda, cima-direita, baixo-direita, baixo-esquerda (regra 5).
    pub radii: [f64; 4],
    /// A opacidade da foto, de 0 a 1 (spec OP, regra 6): só a foto fica translúcida.
    pub opacity: f64,
}

/// A lâmina pronta para desenhar.
#[derive(Debug, Clone, PartialEq)]
pub struct SpreadView {
    /// Pixels da tela por milímetro.
    pub scale: f64,
    /// A lâmina inteira com a sangria (o arquivo exportado).
    pub canvas: PxRect,
    /// O corte final.
    pub trim: PxRect,
    /// Zona segura da página esquerda e da direita.
    pub safe: [PxRect; 2],
    /// Posição x da lombada.
    pub spine_x: f64,
    /// Páginas sem conteúdo (lâmina de ponta com página simples).
    pub inactive: Vec<PxRect>,
    /// Páginas em branco de propósito, para o rótulo só de tela (spec PB).
    pub blank: Vec<PxRect>,
    pub background: Rgb,
    /// Fundo da página direita (igual ao `background` quando não há fundo por página).
    pub background_right: Rgb,
    pub frames: Vec<FrameView>,
}

/// Área da tela onde a lâmina é desenhada, com uma margem livre em volta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub width: f64,
    pub height: f64,
    pub margin: f64,
}

/// A lâmina `index` (a partir de 0) desenhada em `area`; `None` se a lâmina não existe ou a área
/// é pequena demais.
pub fn spread_view(
    project: &Project,
    index: usize,
    area: Area,
    missing: &[PhotoId],
) -> Option<SpreadView> {
    let spread = project.spreads.get(index)?;
    let spec = &project.spec;
    let canvas = spec.canvas_rect();
    let free = (
        area.width - 2.0 * area.margin,
        area.height - 2.0 * area.margin,
    );
    if free.0 <= 0.0 || free.1 <= 0.0 {
        return None;
    }
    let scale = (free.0 / canvas.width.get()).min(free.1 / canvas.height.get());
    let origin = (
        (area.width - canvas.width.get() * scale) / 2.0,
        (area.height - canvas.height.get() * scale) / 2.0,
    );
    let px = |r: Rect| PxRect {
        x: origin.0 + r.x.get() * scale,
        y: origin.1 + r.y.get() * scale,
        width: r.width.get() * scale,
        height: r.height.get() * scale,
    };
    let trim = spec.trim_rect();
    let active = spec.edges().active_pages(index, project.spreads.len());
    let inactive = [
        (active.left, PageSide::Left),
        (active.right, PageSide::Right),
    ]
    .into_iter()
    .filter(|(on, _)| !on)
    .map(|(_, side)| px(spec.page_canvas_rect(side)))
    .collect();
    let blank = [PageSide::Left, PageSide::Right]
        .into_iter()
        .filter(|side| spread.blank.has(*side))
        .map(|side| px(spec.page_rect(side)))
        .collect();
    let frames = spread
        .frames
        .iter()
        .map(|frame| {
            let rect = project.frame_rect(frame);
            let bled = bleed(spec, rect);
            let kind = match &frame.content {
                FrameContent::Empty => FrameKind::Empty,
                FrameContent::Text(text) => FrameKind::Text {
                    text: text.clone(),
                    size_mm: (rect.width.get(), rect.height.get()),
                },
                FrameContent::Photo { photo, crop } => {
                    let size = project
                        .photo(*photo)
                        .map(|p| (f64::from(p.width_px), f64::from(p.height_px)))
                        .unwrap_or((1.0, 1.0));
                    let r = region(crop, rect, &bled, size);
                    FrameKind::Photo {
                        id: *photo,
                        source: PxRect {
                            x: r.x / size.0,
                            y: r.y / size.1,
                            width: r.width / size.0,
                            height: r.height / size.1,
                        },
                        missing: missing.contains(photo),
                    }
                }
            };
            // O texto não cresce até a sangria (só as fotos; spec TX, regra 6).
            let is_text = matches!(kind, FrameKind::Text { .. });
            let at = if is_text { px(rect) } else { px(bled.rect) };
            let r = (frame.radius.get() * scale)
                .min(at.width.min(at.height) / 2.0)
                .max(0.0);
            let [left, top, right, bottom] = bled.grow.map(|g| g > 0.0);
            let corner = |a: bool, b: bool| if a || b { 0.0 } else { r };
            FrameView {
                id: frame.id,
                rect: at,
                kind,
                trim: px(rect),
                border: frame.border.width.get() * scale,
                border_color: frame.border.color,
                radii: [
                    corner(left, top),
                    corner(right, top),
                    corner(right, bottom),
                    corner(left, bottom),
                ],
                opacity: f64::from(frame.opacity.fraction()),
            }
        })
        .collect();
    Some(SpreadView {
        scale,
        canvas: px(canvas),
        trim: px(trim),
        safe: [
            px(spec.safe_rect(PageSide::Left)),
            px(spec.safe_rect(PageSide::Right)),
        ],
        spine_x: origin.0 + (trim.x.get() + trim.width.get() / 2.0) * scale,
        inactive,
        blank,
        background: spread.background,
        background_right: spread.page_background(PageSide::Right),
        frames,
    })
}

/// O quadro da lâmina `index` sob o ponto (`x`, `y`), em frações da largura da lâmina com a
/// sangria (como a janela desenha). Para soltar uma foto (spec FQ, regras 1 e 2).
pub fn frame_at(project: &Project, index: usize, x: f64, y: f64) -> Option<usize> {
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    let view = spread_view(project, index, area, &[])?;
    let hit = |f: &FrameView| {
        let r = f.rect;
        x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height
    };
    // O que está por cima ganha: os textos, depois as fotos da última para a primeira (TX).
    let text = |f: &FrameView| matches!(f.kind, FrameKind::Text { .. });
    let n = view.frames.len();
    (0..n)
        .rev()
        .find(|&i| text(&view.frames[i]) && hit(&view.frames[i]))
        .or_else(|| (0..n).rev().find(|&i| hit(&view.frames[i])))
}

/// A camada de cada quadro na tela: os de foto na ordem da lista e os textos por cima de todos,
/// como na impressão e no [`frame_at`] (spec TX, regra 5). Sem isso, depois de trocar o layout
/// as fotos novas cobriam os textos e pegavam o clique (correção TX de 2026-10-28).
pub fn layers(frames: &[FrameView]) -> Vec<usize> {
    let text = |f: &FrameView| matches!(f.kind, FrameKind::Text { .. });
    let mut photo = 0;
    let mut above = frames.iter().filter(|f| !text(f)).count();
    frames
        .iter()
        .map(|f| {
            let next = if text(f) { &mut above } else { &mut photo };
            *next += 1;
            *next - 1
        })
        .collect()
}

/// A foto inteira do quadro, na escala e na posição em que o recorte a coloca: a prévia do
/// recorte (spec RC, regra 6). A parte dela dentro do quadro coincide com a foto mostrada.
pub fn photo_extent(frame: &FrameView) -> Option<PxRect> {
    let FrameKind::Photo { source, .. } = &frame.kind else {
        return None;
    };
    if source.width <= 0.0 || source.height <= 0.0 {
        return None;
    }
    let r = frame.rect;
    let (width, height) = (r.width / source.width, r.height / source.height);
    Some(PxRect {
        x: r.x - source.x * width,
        y: r.y - source.y * height,
        width,
        height,
    })
}
