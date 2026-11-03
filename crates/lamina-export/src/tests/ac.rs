//! Cenários da spec `aprovacao-cliente` (AC), as imagens: medidas, recorte, nomes, pontas, capa, faixa
//! e marca-d'água. Os da pasta, do código da prova e dos textos estão em `ac_files.rs`.

use std::path::Path;

use image::{ImageDecoder, RgbaImage};
use lamina_core::{AlbumEdge, ColorSpace, CoverMeasures, Mm, Photo, Rgb, Spine};

use super::ex::{Album, names, solid, spec};
use crate::approval::compose::{band_text, compose};
use crate::approval::draw::web_image;
use crate::{
    ApprovalOptions, ApprovalReport, ExportError, approval_names, export_approval, srgb_profile,
};

const BLUE: Rgb = Rgb { r: 0, g: 0, b: 255 };
const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };
const GREEN: Rgb = Rgb { r: 0, g: 200, b: 0 };

/// Sem o número: as medidas ficam só as da imagem.
pub fn plain() -> ApprovalOptions {
    ApprovalOptions {
        number: false,
        ..ApprovalOptions::default()
    }
}

pub fn generate(
    album: &Album,
    folder: &Path,
    options: &ApprovalOptions,
) -> Result<ApprovalReport, ExportError> {
    let source = |p: &Photo, _: (u32, u32)| {
        album
            .images
            .get(&p.path)
            .cloned()
            .ok_or_else(|| "arquivo sumiu".to_owned())
    };
    export_approval(&album.project, folder, &source, options, &|_, _| {})
}

/// A imagem antes de codificar, sem faixa e sem marca-d'água.
fn clean(album: &Album, index: usize) -> RgbaImage {
    let source = |p: &Photo, _: (u32, u32)| {
        album
            .images
            .get(&p.path)
            .cloned()
            .ok_or_else(|| "arquivo sumiu".to_owned())
    };
    web_image(&album.project, index, &source).unwrap()
}

/// Uma capa de face 303 × 306 mm, lombada de 15 mm e dobra de 15 mm (a do CA-B2).
pub fn with_cover(album: &mut Album) {
    let mut measures = CoverMeasures::defaults(&album.project.spec);
    measures.face_width = Mm(303.0);
    measures.face_height = Mm(306.0);
    measures.wrap = Mm(15.0);
    measures.spine = Spine::Fixed {
        mm: Mm(15.0),
        pages: album.project.page_count(),
    };
    album.project.create_cover(measures).unwrap();
}

fn all_are(image: &RgbaImage, color: Rgb) -> bool {
    image
        .pixels()
        .all(|p| p[0] == color.r && p[1] == color.g && p[2] == color.b)
}

/// Fatores de amostragem (H, V) de cada componente, lidos do cabeçalho SOF do JPEG.
fn sampling(jpeg: &[u8]) -> Vec<(u8, u8)> {
    let sof = jpeg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
    let count = usize::from(jpeg[sof + 9]);
    (0..count)
        .map(|k| {
            let f = jpeg[sof + 11 + 3 * k];
            (f >> 4, f & 15)
        })
        .collect()
}

/// Qualidade estimada pela tabela de quantização do brilho, como na trava da regra 14.
fn estimated_quality(jpeg: &[u8]) -> u8 {
    const BASE: [u32; 8] = [16, 11, 10, 16, 24, 40, 51, 61];
    let dqt = jpeg.windows(2).position(|w| w == [0xFF, 0xDB]).unwrap();
    let zz = &jpeg[dqt + 5..dqt + 5 + 64];
    let row: [u32; 8] = [zz[0], zz[1], zz[5], zz[6], zz[14], zz[15], zz[27], zz[28]].map(u32::from);
    (1..=100u8)
        .min_by_key(|&q| {
            let q = u32::from(q);
            let scale = if q < 50 { 5000 / q } else { 200 - 2 * q };
            BASE.iter()
                .zip(row)
                .map(|(b, v)| ((b * scale + 50) / 100).clamp(1, 255).abs_diff(v))
                .sum::<u32>()
        })
        .unwrap()
}

/// Densidade do cabeçalho JFIF: (unidade, x, y).
fn density(bytes: &[u8]) -> (u8, u16, u16) {
    let at = bytes.windows(5).position(|w| w == b"JFIF\0").unwrap();
    let x = u16::from_be_bytes([bytes[at + 8], bytes[at + 9]]);
    let y = u16::from_be_bytes([bytes[at + 10], bytes[at + 11]]);
    (bytes[at + 7], x, y)
}

fn icc_of(bytes: &[u8]) -> Vec<u8> {
    let mut decoder = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(bytes)).unwrap();
    decoder.icc_profile().unwrap().unwrap()
}

#[test]
fn ac_b1_medidas_e_arquivo() {
    let mut album = Album::new(spec(), 2);
    let photo = album.photo("f.jpg", solid(1600, 1000, [120, 90, 60]));
    album.photo_frame(0, photo, [0.1, 0.1, 0.5, 0.6]);
    let dir = tempfile::tempdir().unwrap();
    let report = generate(&album, dir.path(), &plain()).unwrap();
    assert_eq!(report.files.len(), 2);
    for file in &report.files {
        let bytes = std::fs::read(file).unwrap();
        assert_eq!(image::image_dimensions(file).unwrap(), (2048, 1024));
        assert_eq!(estimated_quality(&bytes), 80);
        assert_eq!(sampling(&bytes), [(1, 1), (1, 1), (1, 1)], "4:4:4");
        assert_eq!(icc_of(&bytes), srgb_profile().unwrap());
        assert_eq!(density(&bytes), (1, 72, 72), "1 = pontos por polegada");
        assert!(!bytes.windows(5).any(|w| w == b"Exif\0"), "dados de câmera");
    }
}

#[test]
fn ac_b2_sem_sangria() {
    let mut album = Album::new(spec(), 1);
    album.project.spreads[0].background = BLUE;
    album.project.spreads[0].background_right = Some(RED);
    let image = clean(&album, 0);
    assert_eq!(image.dimensions(), (2048, 1024));
    assert_eq!(image.get_pixel(1023, 512).0[..3], [0, 0, 255]);
    assert_eq!(image.get_pixel(1024, 512).0[..3], [255, 0, 0]);
    assert_eq!(image.get_pixel(0, 0).0[..3], [0, 0, 255]);
    assert_eq!(image.get_pixel(2047, 1023).0[..3], [255, 0, 0]);
}

#[test]
fn ac_b3_nomes() {
    let mut album = Album::new(spec(), 12);
    with_cover(&mut album);
    let dir = tempfile::tempdir().unwrap();
    generate(&album, dir.path(), &plain()).unwrap();
    let mut expected = vec!["web-capa.jpg".to_owned()];
    expected.extend((1..=12).map(|n| format!("web-lamina-{n:02}.jpg")));
    assert_eq!(names(dir.path()), expected);

    let big = Album::new(spec(), 100);
    let listed = approval_names(&big.project, &plain()).unwrap();
    assert_eq!(listed.len(), 100);
    assert_eq!(listed[0], "web-lamina-001.jpg");
    assert_eq!(listed[99], "web-lamina-100.jpg");
}

#[test]
fn ac_b4_intervalo() {
    let mut album = Album::new(spec(), 12);
    with_cover(&mut album);
    let dir = tempfile::tempdir().unwrap();
    let options = ApprovalOptions {
        range: Some("5, 12".to_owned()),
        ..plain()
    };
    generate(&album, dir.path(), &options).unwrap();
    assert_eq!(
        names(dir.path()),
        ["web-lamina-05.jpg", "web-lamina-12.jpg"]
    );
}

#[test]
fn ac_b5_texto_da_faixa() {
    let spread = band_text(Some(5), "3F2A9C");
    let cover = band_text(None, "3F2A9C");
    assert_eq!(spread, "Lâmina 5 · prova 3F2A9C");
    assert_eq!(cover, "Capa · prova 3F2A9C");
    let text = lamina_core::TextBox {
        text: spread,
        ..lamina_core::TextBox::new(Rgb::BLACK)
    };
    let check = lamina_render::check_text(&text, (200.0, 20.0));
    assert!(check.missing.is_empty(), "{:?}", check.missing);
}

#[test]
fn ac_b6_pontas_de_pagina_simples() {
    let edges = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let mut album = Album::new(edges, 3);
    album.project.spreads[0].background_right = Some(BLUE);
    let first = clean(&album, 0);
    assert_eq!(first.dimensions(), (1024, 1024));
    assert!(all_are(&first, BLUE));
    let dir = tempfile::tempdir().unwrap();
    let report = generate(&album, dir.path(), &plain()).unwrap();
    let sizes: Vec<_> = report
        .files
        .iter()
        .map(|f| image::image_dimensions(f).unwrap())
        .collect();
    assert_eq!(sizes, [(1024, 1024), (2048, 1024), (1024, 1024)]);
}

#[test]
fn ac_b7_lamina_sem_paginas() {
    let edges = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let mut album = Album::new(edges, 1);
    with_cover(&mut album);
    let dir = tempfile::tempdir().unwrap();
    generate(&album, dir.path(), &plain()).unwrap();
    assert_eq!(names(dir.path()), ["web-capa.jpg"]);
}

#[test]
fn ac_b8_pagina_e_lamina_em_branco() {
    let mut album = Album::new(spec(), 3);
    let photo = album.photo("f.jpg", solid(1600, 1000, [10, 10, 10]));
    for spread in &mut album.project.spreads {
        spread.background = GREEN;
    }
    album.project.spreads[1].blank.left = true;
    album.project.spreads[1].blank.right = true;
    album.project.spreads[2].blank.right = true;
    album.photo_frame(2, photo, [0.05, 0.1, 0.4, 0.8]);
    let dir = tempfile::tempdir().unwrap();
    let report = generate(&album, dir.path(), &plain()).unwrap();
    assert_eq!(report.files.len(), 3);
    assert!(all_are(&clean(&album, 1), GREEN));
    let third = clean(&album, 2);
    let right = image::imageops::crop_imm(&third, 1024, 0, 1024, 1024).to_image();
    assert!(all_are(&right, GREEN));
    assert!(!all_are(&third, GREEN), "a foto da esquerda continua lá");
}

#[test]
fn ac_b9_capa() {
    let mut album = Album::new(spec(), 2);
    with_cover(&mut album);
    let dir = tempfile::tempdir().unwrap();
    generate(&album, dir.path(), &plain()).unwrap();
    let cover = dir.path().join("web-capa.jpg");
    assert_eq!(image::image_dimensions(&cover).unwrap(), (2048, 1009));
}

#[test]
fn ac_b10_sem_capa() {
    let album = Album::new(spec(), 2);
    let dir = tempfile::tempdir().unwrap();
    generate(&album, dir.path(), &plain()).unwrap();
    assert!(!dir.path().join("web-capa.jpg").exists());
}

fn gray(w: u32, h: u32) -> RgbaImage {
    solid(w, h, [90, 90, 90])
}

#[test]
fn ac_b11_faixa_do_numero() {
    let without = compose(gray(2048, 1024), None, None);
    let with = compose(gray(2048, 1024), Some("Lâmina 5 · prova 3F2A9C"), None);
    assert_eq!(without.dimensions(), (2048, 1024));
    assert_eq!(with.dimensions(), (2048, 1067));
    let top = image::imageops::crop_imm(&with, 0, 0, 2048, 1024).to_image();
    assert_eq!(top, without, "a faixa não cobre a imagem");
    let band = image::imageops::crop_imm(&with, 0, 1024, 2048, 43).to_image();
    assert_eq!(
        band.get_pixel(0, 0).0[..3],
        [250, 250, 250],
        "fundo da faixa"
    );
    assert!(
        band.pixels().any(|p| p[0] < 100),
        "o texto aparece na faixa"
    );
}

#[test]
fn ac_b12_marca_dagua() {
    let band = Some("Lâmina 5 · prova 3F2A9C");
    let without = compose(gray(2048, 1024), band, None);
    let with = compose(gray(2048, 1024), band, Some("Estúdio Exemplo"));
    assert_eq!(with.dimensions(), without.dimensions());
    let part = |image: &RgbaImage, y: u32, h: u32| {
        image::imageops::crop_imm(image, 0, y, 2048, h).to_image()
    };
    assert_ne!(part(&with, 0, 1024), part(&without, 0, 1024));
    assert_eq!(part(&with, 1024, 43), part(&without, 1024, 43));
}

/// Achado ao olhar a imagem: com a letra sempre em 1/14 da altura, "Estúdio Exemplo" invadia o
/// vizinho. A coluna do meio entre dois centros fica limpa, com o texto curto e com o de 40 letras.
#[test]
fn ac_b12_a_marca_dagua_nao_invade_a_vizinha() {
    for (size, text) in [
        ((2048, 1024), "Estúdio Exemplo"),
        ((2048, 1024), "Estúdio Fotográfico Exemplo de São Paulo"),
        ((1024, 1024), "Estúdio Exemplo"),
    ] {
        let without = gray(size.0, size.1);
        let with = compose(gray(size.0, size.1), None, Some(text));
        assert_ne!(with, without, "{text}");
        for column in [size.0 * 3 / 8, size.0 * 5 / 8] {
            for y in 0..size.1 {
                assert_eq!(
                    with.get_pixel(column, y),
                    without.get_pixel(column, y),
                    "{text} em ({column}, {y})"
                );
            }
        }
    }
}

#[test]
fn ac_b14_sempre_srgb() {
    let mut album = Album::new(spec(), 1);
    album.project.export.space = ColorSpace::AdobeRgb;
    let before = album.project.clone();
    let dir = tempfile::tempdir().unwrap();
    let report = generate(&album, dir.path(), &plain()).unwrap();
    let bytes = std::fs::read(&report.files[0]).unwrap();
    assert_eq!(icc_of(&bytes), srgb_profile().unwrap());
    assert_eq!(album.project, before, "a preferência do álbum não muda");
}
