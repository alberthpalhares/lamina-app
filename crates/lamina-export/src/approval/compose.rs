//! A imagem final (spec AC, regras 11 a 13): a faixa com o número embaixo e a marca-d'água por cima,
//! antes de codificar, para os testes compararem pixels sem o JPEG no meio. Tudo na Inter embutida,
//! que sai igual em qualquer máquina (AGENTS.md, regra 12). O `lamina-render` não muda.

use image::{Rgba, RgbaImage};
use lamina_core::{MM_PER_INCH, Rgb, TextAlign, TextBox, TextVAlign};
use lamina_render::text_image;

const BAND_BACKGROUND: [u8; 4] = [250, 250, 250, 255];
const BAND_INK: u8 = 40;
const BAND_MIN: u32 = 24;
/// Branco e sombra escura da marca-d'água, os dois a 35%, com a sombra deslocada 2 px.
const MARK_LIGHT: u8 = 255;
const MARK_DARK: u8 = 0;
const MARK_PERCENT: u32 = 35;
const MARK_SHADOW: i64 = 2;
/// Quanto do espaço entre dois centros o texto pode ocupar.
const MARK_FILL: u32 = 90;
/// Largura do desenho, em letras: o limite de 40 caracteres cabe numa linha só.
const MARK_COLUMNS: u32 = 40;
const FULL_PERCENT: u32 = 100;

/// O texto da faixa: "Lâmina 5 · prova 3F2A9C", ou "Capa · prova 3F2A9C" sem número.
pub fn band_text(number: Option<usize>, code: &str) -> String {
    match number {
        Some(number) => format!("Lâmina {number} · prova {code}"),
        None => format!("Capa · prova {code}"),
    }
}

/// A marca-d'água sobre a imagem e a faixa embaixo dela, que nunca cobre a foto.
pub fn compose(mut image: RgbaImage, band: Option<&str>, mark: Option<&str>) -> RgbaImage {
    if let Some(text) = mark {
        watermark(&mut image, text);
    }
    match band {
        Some(text) => with_band(&image, text),
        None => image,
    }
}

/// `value` dividido por `by`, arredondado, sem passar pelo ponto flutuante.
fn rounded_div(value: u32, by: u32) -> u32 {
    (value + by / 2) / by
}

fn with_band(image: &RgbaImage, text: &str) -> RgbaImage {
    let (width, height) = image.dimensions();
    let band = rounded_div(height, 24).max(BAND_MIN);
    let mut out = RgbaImage::from_pixel(width, height + band, Rgba(BAND_BACKGROUND));
    image::imageops::replace(&mut out, image, 0, 0);
    if let Some(tile) = text_tile(text, rounded_div(band * 2, 3), (width, band)) {
        paint(
            &mut out,
            &tile,
            (0, i64::from(height)),
            (BAND_INK, FULL_PERCENT),
        );
    }
    out
}

/// O texto repetido com os centros em 1/4, 2/4 e 3/4 da largura e da altura (regra 12).
fn watermark(image: &mut RgbaImage, text: &str) {
    let (width, height) = image.dimensions();
    let letter = rounded_div(height, 14).max(1);
    let Some(tile) = mark_tile(text, letter, width / 4) else {
        return;
    };
    let half = (i64::from(tile.width()) / 2, i64::from(tile.height()) / 2);
    for row in 1..=3 {
        for column in 1..=3 {
            let x = i64::from(width) * column / 4 - half.0;
            let y = i64::from(height) * row / 4 - half.1;
            let shadow = (x + MARK_SHADOW, y + MARK_SHADOW);
            paint(image, &tile, shadow, (MARK_DARK, MARK_PERCENT));
            paint(image, &tile, (x, y), (MARK_LIGHT, MARK_PERCENT));
        }
    }
}

/// O texto com a letra de `letter` pixels, ou menor, se ele não couber em 90% do espaço entre dois
/// centros: sem isso, "Estúdio Exemplo" já invade o vizinho numa lâmina de 2048 px.
fn mark_tile(text: &str, letter: u32, spacing: u32) -> Option<RgbaImage> {
    let limit = (spacing * MARK_FILL / FULL_PERCENT).max(1);
    let tile = |letter: u32| text_tile(text, letter, (letter * MARK_COLUMNS, letter * 2));
    let drawn = tile(letter)?;
    let ink = ink_width(&drawn);
    if ink <= limit {
        return Some(drawn);
    }
    tile((letter * limit / ink).max(1))
}

/// Quantos pixels o desenho ocupa, da primeira à última coluna com tinta.
fn ink_width(tile: &RgbaImage) -> u32 {
    let columns = tile
        .enumerate_pixels()
        .filter(|(_, _, pixel)| pixel[3] > 0)
        .map(|(x, _, _)| x);
    let (first, last) = columns.fold((u32::MAX, 0), |(a, b), x| (a.min(x), b.max(x)));
    (last + 1).saturating_sub(first)
}

/// O texto centrado numa imagem de `size` pixels, com a letra de `letter` pixels. Um ponto vale um
/// pixel: o quadro em milímetros é o tamanho em pontos. Só o alfa interessa a [`paint`].
fn text_tile(text: &str, letter: u32, size: (u32, u32)) -> Option<RgbaImage> {
    let text = TextBox {
        text: text.to_owned(),
        size_pt: f64::from(letter),
        align: TextAlign::Center,
        valign: TextVAlign::Middle,
        ..TextBox::new(Rgb::BLACK)
    };
    let mm = |pixels: u32| f64::from(pixels) * MM_PER_INCH / 72.0;
    text_image(&text, (mm(size.0), mm(size.1)), size)
}

/// Pinta a forma de `tile` em `at`, na cor cinza `shade` e na força `percent`. O que cai fora da
/// imagem fica de fora.
fn paint(image: &mut RgbaImage, tile: &RgbaImage, at: (i64, i64), (shade, percent): (u8, u32)) {
    for (x, y, pixel) in tile.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        let target = (at.0 + i64::from(x), at.1 + i64::from(y));
        let (Ok(tx), Ok(ty)) = (u32::try_from(target.0), u32::try_from(target.1)) else {
            continue;
        };
        if tx >= image.width() || ty >= image.height() {
            continue;
        }
        let alpha = u32::from(pixel[3]) * percent / FULL_PERCENT;
        let spot = image.get_pixel_mut(tx, ty);
        for channel in &mut spot.0[..3] {
            let mixed = u32::from(shade) * alpha + u32::from(*channel) * (255 - alpha);
            *channel = u8::try_from(rounded_div(mixed, 255)).unwrap_or(u8::MAX);
        }
    }
}
