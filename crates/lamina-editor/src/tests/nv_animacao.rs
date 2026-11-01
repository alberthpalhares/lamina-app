//! Cenários da spec `o-que-ha-de-novo` (NV), etapa 2: as animações da janela. A leitura quadro a
//! quadro (NV-B14), o laço (NV-B15) e o "Reduzir movimento" (NV-B16).

use std::io::Cursor;
use std::path::Path;

use image::AnimationDecoder;
use image::codecs::png::PngDecoder;

use crate::updates::news::{Animation, Frame, parse};

const REAL: &str = include_str!("../../../lamina-app/dist/novidades.md");

/// Três quadros de 8×8: vermelho por 100 ms, verde por 200 ms e azul por 100 ms.
const THREE: &[u8] = include_bytes!("dados/tres-quadros.png");

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];

fn color(frame: &Frame) -> [u8; 3] {
    let p = &frame.picture.rgba;
    [p[0], p[1], p[2]]
}

/// Um PNG parado de 8×8, da cor dada.
fn still(rgb: [u8; 3]) -> Vec<u8> {
    let picture = image::RgbImage::from_pixel(8, 8, image::Rgb(rgb));
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

/// A duração de uma animação, somando o tempo de cada quadro (zero se é parada).
fn duration_ms(bytes: &[u8]) -> u32 {
    let decoder = PngDecoder::new(Cursor::new(bytes)).unwrap();
    if !decoder.is_apng().unwrap() {
        return 0;
    }
    let frames = decoder.apng().unwrap().into_frames();
    frames
        .map(|f| {
            let (n, d) = f.unwrap().delay().numer_denom_ms();
            n / d.max(1)
        })
        .sum()
}

#[test]
fn nv_b14_o_lamina_le_a_animacao_quadro_a_quadro() {
    let mut animation = Animation::open(THREE, false).unwrap();
    assert!(animation.animated());
    assert_eq!(color(animation.first()), RED);
    assert_eq!(animation.first().delay_ms, 100);
    let green = animation.advance().unwrap();
    assert_eq!((color(&green), green.delay_ms), (GREEN, 200));
    let blue = animation.advance().unwrap();
    assert_eq!((color(&blue), blue.delay_ms), (BLUE, 100));
    assert_eq!(
        (
            animation.first().picture.width,
            animation.first().picture.height
        ),
        (8, 8)
    );

    // Parada: não anima e não tem quadro seguinte.
    let bytes = still(GREEN);
    let mut stopped = Animation::open(&bytes, false).unwrap();
    assert!(!stopped.animated());
    assert_eq!(color(stopped.first()), GREEN);
    assert!(stopped.advance().is_none());

    // Bytes estragados não abrem.
    assert!(Animation::open(b"nada", false).is_none());
}

#[test]
fn nv_b14_as_ilustracoes_de_verdade_cabem_no_limite() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lamina-app/dist/novidades");
    // O leitor pula o "Não lançado"; aqui ele conta como uma versão, para conferir as ilustrações
    // que vão sair.
    let sections = parse(&REAL.replace("## [Não lançado]", "## [9.9.9]"));
    let images: Vec<&String> = sections
        .iter()
        .flat_map(|s| &s.items)
        .filter_map(|i| i.image.as_ref())
        .collect();
    assert!(!images.is_empty());
    for name in images {
        let bytes = std::fs::read(folder.join(name)).unwrap();
        assert!(bytes.len() <= 1_500_000, "{name} tem {} bytes", bytes.len());
        let animation = Animation::open(&bytes, false).unwrap();
        let first = &animation.first().picture;
        assert!(first.width <= 1280 && first.height <= 720, "{name}");
        if animation.animated() {
            assert_eq!((first.width, first.height), (1280, 720), "{name}");
            let ms = duration_ms(&bytes);
            assert!((3_000..=8_000).contains(&ms), "{name} dura {ms} ms");
        }
    }
}

#[test]
fn nv_b15_a_animacao_toca_em_laco() {
    let mut animation = Animation::open(THREE, false).unwrap();
    let seen: Vec<[u8; 3]> = (0..7)
        .map(|_| color(&animation.advance().unwrap()))
        .collect();
    assert_eq!(seen, [GREEN, BLUE, RED, GREEN, BLUE, RED, GREEN]);
}

#[test]
fn nv_b16_reduzir_movimento_deixa_o_primeiro_quadro_parado() {
    let mut animation = Animation::open(THREE, true).unwrap();
    assert!(!animation.animated());
    assert_eq!(color(animation.first()), RED);
    assert!(animation.advance().is_none());
}

/// Toda imagem de `dist/novidades/` vai dentro do programa (`build.rs`): uma que nenhuma nota
/// cita só pesaria no instalador.
#[test]
fn as_ilustracoes_embutidas_sao_todas_citadas() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lamina-app/dist/novidades");
    let sections = parse(&REAL.replace("## [Não lançado]", "## [9.9.9]"));
    let cited: Vec<&String> = sections
        .iter()
        .flat_map(|s| &s.items)
        .filter_map(|i| i.image.as_ref())
        .collect();
    for entry in std::fs::read_dir(folder).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            cited.iter().any(|c| **c == name),
            "{name} está em dist/novidades, mas nenhuma nota cita"
        );
    }
}
