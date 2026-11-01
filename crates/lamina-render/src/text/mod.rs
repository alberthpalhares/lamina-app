//! Texto nas lâminas (spec TX): fontes embutidas e do computador (spec FS), linhas calculadas uma
//! vez e desenho em qualquer escala.

mod draw;
mod fonts;
mod installed;
mod layout;
pub(crate) mod source;
pub mod testing;

use image::RgbaImage;
use lamina_core::TextBox;
use skrifa::{FontRef, MetadataProvider};
use tiny_skia::Pixmap;

pub use fonts::{FAMILIES, Family, family, known_families};
pub use installed::{
    FontReads, InstalledFont, installed_name, installed_names, installed_names_ready,
    request_installed_names,
};
pub use layout::text_lines;
pub use source::{Face, FontSource};

use fonts::{drawn_or_default, resolve};
use layout::{lay_out, pt};

/// O que uma família tem (spec FS, regra 3): sem o arquivo, a caixa fica apagada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontStyles {
    pub bold: bool,
    pub italic: bool,
}

/// Negrito e itálico da família de um nome; `None` quando ela não está neste computador nem é
/// embutida. Uma embutida responde sem tocar nas fontes do computador.
pub fn font_styles(name: &str) -> Option<FontStyles> {
    resolve(name).map(|d| FontStyles {
        bold: d.bold(),
        italic: d.italic(),
    })
}

/// O nome de uma família escrito nela mesma, numa imagem de `px` pixels (a prévia da lista, spec
/// FS, regra 11). `None` quando a família não está aqui ou não desenha nada.
pub fn font_preview(name: &str, px: (u32, u32)) -> Option<RgbaImage> {
    let drawn = resolve(name)?;
    let text = TextBox {
        text: name.to_owned(),
        font: drawn.name().to_owned(),
        // Uma linha de 7 mm de altura: o tamanho vem da altura da imagem.
        size_pt: 12.0,
        align: lamina_core::TextAlign::Left,
        valign: lamina_core::TextVAlign::Middle,
        ..TextBox::new(lamina_core::Rgb::BLACK)
    };
    let height_mm = 7.0;
    let width_mm = height_mm * f64::from(px.0) / f64::from(px.1.max(1));
    let img = text_image(&text, (width_mm, height_mm), px)?;
    img.pixels().any(|p| p[3] > 0).then_some(img)
}

/// O que a pré-impressão precisa saber de um texto (spec TX, seção 4).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextCheck {
    /// O bloco passa da altura do quadro.
    pub overflow: bool,
    /// Caracteres que a fonte não tem, na ordem em que aparecem, sem repetir.
    pub missing: Vec<char>,
    /// A fonte do projeto não é embutida nem está neste computador (o texto sai na padrão).
    pub unknown_font: bool,
}

/// Confere o texto num quadro de `size_mm` (largura, altura).
pub fn check_text(text: &TextBox, size_mm: (f64, f64)) -> TextCheck {
    if text.is_blank() {
        return TextCheck::default();
    }
    let found = resolve(&text.font);
    let unknown_font = found.is_none();
    let drawn = found.unwrap_or_else(|| drawn_or_default(lamina_core::DEFAULT_FONT));
    let mut missing = Vec::new();
    let (bytes, index) = drawn.face(text.italic && drawn.italic());
    if let Ok(face) = FontRef::from_index(bytes, index) {
        let map = face.charmap();
        for c in text.text.chars().filter(|c| !c.is_whitespace()) {
            if map.map(c).is_none() && !missing.contains(&c) {
                missing.push(c);
            }
        }
    }
    // Em pé, a largura do texto é a altura do quadro (spec CA, regra 9).
    let upright = if text.rotation.is_zero() {
        size_mm
    } else {
        (size_mm.1, size_mm.0)
    };
    TextCheck {
        overflow: lay_out(text, upright).overflow,
        missing,
        unknown_font,
    }
}

/// O texto desenhado numa imagem transparente de `px` pixels, para um quadro de `size_mm`. As
/// cores saem pré-multiplicadas pelo alfa. `None` para texto vazio ou tamanho zero.
pub(crate) fn text_pixmap(text: &TextBox, size_mm: (f64, f64), px: (u32, u32)) -> Option<Pixmap> {
    if text.is_blank() || size_mm.0 <= 0.0 {
        return None;
    }
    if !text.rotation.is_zero() {
        let upright = TextBox {
            rotation: lamina_core::TextRotation::Zero,
            ..text.clone()
        };
        let flat = text_pixmap(&upright, (size_mm.1, size_mm.0), (px.1, px.0))?;
        return turned(&flat, text.rotation);
    }
    let mut pixmap = Pixmap::new(px.0, px.1)?;
    let laid = lay_out(text, size_mm);
    // Pixels por ponto, pela largura (a altura tem a mesma escala, a menos do arredondamento).
    #[allow(clippy::cast_possible_truncation)]
    let k = (f64::from(px.0) / pt(size_mm.0)) as f32;
    draw::draw(&mut pixmap, &laid, text.color, k);
    Some(pixmap)
}

/// O bloco pronto girado 90°: `Up` para a esquerda (lê de baixo para cima), `Down` para a direita.
fn turned(flat: &Pixmap, rotation: lamina_core::TextRotation) -> Option<Pixmap> {
    let (w, h) = (flat.width() as usize, flat.height() as usize);
    let mut out = Pixmap::new(flat.height(), flat.width())?;
    let source = flat.data();
    let target = out.data_mut();
    for y in 0..h {
        for x in 0..w {
            let (tx, ty) = match rotation {
                lamina_core::TextRotation::Up => (y, w - 1 - x),
                _ => (h - 1 - y, x),
            };
            let from = (y * w + x) * 4;
            let to = (ty * h + tx) * 4;
            target[to..to + 4].copy_from_slice(&source[from..from + 4]);
        }
    }
    Some(out)
}

/// Como [`text_pixmap`], numa `RgbaImage` (cores pré-multiplicadas), para a janela.
pub fn text_image(text: &TextBox, size_mm: (f64, f64), px: (u32, u32)) -> Option<RgbaImage> {
    let pixmap = text_pixmap(text, size_mm, px)?;
    RgbaImage::from_raw(px.0, px.1, pixmap.take())
}

/// Desenha os textos da lâmina por cima das fotos, na ordem dos quadros (spec TX, regra 5).
pub(crate) fn draw_texts(
    pixmap: &mut Pixmap,
    project: &lamina_core::Project,
    spread: &lamina_core::Spread,
    scale: crate::place::Scale,
) {
    let size = (pixmap.width(), pixmap.height());
    let trim = project.spec.trim_rect();
    for frame in &spread.frames {
        let lamina_core::FrameContent::Text(text) = &frame.content else {
            continue;
        };
        let rect = frame.rect.to_absolute(&trim);
        let Some(at) = crate::place::pixels(rect, scale, size) else {
            continue;
        };
        let size_mm = (rect.width.get(), rect.height.get());
        if let Some(t) = text_pixmap(text, size_mm, (at.width(), at.height())) {
            // As posições cabem em i32: a tela tem no máximo alguns milhares de pixels.
            #[allow(clippy::cast_possible_wrap)]
            let (x, y) = (at.x0 as i32, at.y0 as i32);
            pixmap.draw_pixmap(
                x,
                y,
                t.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }
}
