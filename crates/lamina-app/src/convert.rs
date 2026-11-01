//! Tradução dos dados do `lamina-editor` para as estruturas das telas Slint. Nenhuma decisão
//! aqui: as contas são do editor (regra 1).

use std::rc::Rc;

use lamina_core::{PhotoId, Project, Rgb};
use lamina_editor::motion::PhotoRow;
use lamina_editor::{Area, FrameKind, PxRect, layers, photo_extent, spread_view};
use slint::{Color, Image, ModelRc, VecModel};

use crate::texts::{picture, pixels};
use crate::{BoxData, FrameData, PhotoData, SpreadData};

/// Proporção altura ÷ largura da lâmina com a sangria.
pub fn aspect(project: &Project) -> f32 {
    let canvas = project.spec.canvas_rect();
    to_f32(canvas.height.get() / canvas.width.get())
}

// As medidas da tela cabem com folga em f32 (frações e pixels).
#[allow(clippy::cast_possible_truncation)]
pub fn to_f32(v: f64) -> f32 {
    v as f32
}

pub fn boxed(r: PxRect) -> BoxData {
    BoxData {
        x: to_f32(r.x),
        y: to_f32(r.y),
        w: to_f32(r.width),
        h: to_f32(r.height),
    }
}

pub fn color(c: Rgb) -> Color {
    Color::from_rgb_u8(c.r, c.g, c.b)
}

/// A lâmina `index` em frações da largura da lâmina (a tela escala sozinha), com as imagens
/// que `image` der (imagem de tela na lâmina grande, miniatura na faixa) e os textos desenhados
/// para uma lâmina de `text_px` pixels de largura (spec TX).
pub fn spread_data(
    project: &Project,
    index: usize,
    missing: &[PhotoId],
    image: &dyn Fn(PhotoId) -> Option<Image>,
    (warning, text_px): (bool, f64),
) -> SpreadData {
    let a = f64::from(aspect(project));
    let area = Area {
        width: 1.0,
        height: a,
        margin: 0.0,
    };
    let Some(view) = spread_view(project, index, area, missing) else {
        return SpreadData::default();
    };
    let layer = layers(&view.frames);
    let frames: Vec<FrameData> = view
        .frames
        .iter()
        .enumerate()
        .map(|(i, f)| {
            // A foto inteira (ou a imagem do texto, do tamanho do quadro), recortada pelo quadro.
            let whole = photo_extent(f).unwrap_or(f.rect);
            let mut data = FrameData {
                px: to_f32(whole.x),
                py: to_f32(whole.y),
                pw: to_f32(whole.width),
                ph: to_f32(whole.height),
                index: i32::try_from(i).unwrap_or(-1),
                z: to_f32(layer[i] as f64),
                live: true,
                x: to_f32(f.rect.x),
                y: to_f32(f.rect.y),
                w: to_f32(f.rect.width),
                h: to_f32(f.rect.height),
                tl: to_f32(f.radii[0]),
                tr: to_f32(f.radii[1]),
                br: to_f32(f.radii[2]),
                bl: to_f32(f.radii[3]),
                bw: to_f32(f.border),
                bcolor: color(f.border_color),
                bx: to_f32(f.trim.x),
                by: to_f32(f.trim.y),
                bww: to_f32(f.trim.width),
                bh: to_f32(f.trim.height),
                op: to_f32(f.opacity),
                ..FrameData::default()
            };
            match &f.kind {
                FrameKind::Empty => {}
                FrameKind::Text { text, size_mm } => {
                    data.kind = 2;
                    data.blank = text.is_blank();
                    let px = (
                        pixels(f.rect.width, text_px),
                        pixels(f.rect.height, text_px),
                    );
                    if let Some(img) = picture(text, *size_mm, px) {
                        data.img = img;
                        data.has_img = true;
                    }
                }
                FrameKind::Photo { id, missing, .. } => {
                    data.kind = 1;
                    data.photo = i32::try_from(id.get()).unwrap_or(-1);
                    data.missing = *missing;
                    data.label = project
                        .photo(*id)
                        .ok()
                        .and_then(|p| p.path.file_name())
                        .map(|n| n.to_string_lossy().into_owned().into())
                        .unwrap_or_default();
                    if let Some(img) = image(*id).filter(|_| !*missing) {
                        data.img = img;
                        data.has_img = true;
                    }
                }
            }
            data
        })
        .collect();
    let inactive: Vec<BoxData> = view.inactive.iter().copied().map(boxed).collect();
    let blank: Vec<BoxData> = view.blank.iter().copied().map(boxed).collect();
    SpreadData {
        badge: "".into(),
        aspect: to_f32(a),
        bg: color(view.background),
        bg_right: color(view.background_right),
        trim: boxed(view.trim),
        safe_l: boxed(view.safe[0]),
        safe_r: boxed(view.safe[1]),
        spine_x: to_f32(view.spine_x),
        inactive: ModelRc::from(Rc::new(VecModel::from(inactive))),
        blank: ModelRc::from(Rc::new(VecModel::from(blank))),
        frames: ModelRc::from(Rc::new(VecModel::from(frames))),
        warning,
        snaps: ModelRc::default(),
        selected_box: BoxData::default(),
        cropping_box: BoxData::default(),
    }
}

/// A proporção (largura / altura) da foto `id`, para o retângulo neutro antes da miniatura;
/// 0 sem tamanho (a peça fica quadrada).
pub fn photo_aspect(project: &Project, id: PhotoId) -> f32 {
    match project.photo(id) {
        Ok(p) if p.width_px > 0 && p.height_px > 0 => p.width_px as f32 / p.height_px as f32,
        _ => 0.0,
    }
}

/// Uma peça da grade do painel de fotos (spec PF): a miniatura (só perto da tela), o nome, os
/// usos, a seleção, o esmaecido e a proporção `aspect` da foto (0 = sem tamanho).
pub fn photo_data(row: &PhotoRow, aspect: f32, image: Option<Image>) -> PhotoData {
    PhotoData {
        img: image.unwrap_or_default(),
        name: row.item.name.clone().into(),
        uses: i32::try_from(row.item.uses).unwrap_or(i32::MAX),
        selected: row.picked,
        aspect,
        dim: row.dim,
        missing: row.missing,
    }
}

/// Lista Slint a partir de um vetor.
pub fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(items)))
}
