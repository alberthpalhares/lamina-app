//! Cenários RD-B8, RD-B9, RD-B11 e RD-B12 da spec `specs/renderizacao.md`.

use super::*;
use lamina_core::testing::{TRIES, budget};

/// Degradê suave: muda pouco de um pixel para o vizinho, então prévia e impressão podem ser
/// comparadas no centro dos quadros.
fn gradient(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        #[allow(clippy::cast_possible_truncation)]
        let (r, g) = ((x * 255 / w) as u8, (y * 255 / h) as u8);
        Rgba([r, g, 128, 255])
    })
}

/// Três fotos numa lâmina sem sangria, como a montagem automática arrumaria.
fn three_frames() -> (Album, [[f64; 4]; 3]) {
    let mut album = Album::new(0.0);
    let frames = [
        [0.05, 0.1, 0.4, 0.8],
        [0.55, 0.1, 0.2, 0.35],
        [0.55, 0.55, 0.2, 0.35],
    ];
    for (i, rect) in frames.iter().enumerate() {
        let photo = album.photo(&format!("g{i}.jpg"), gradient(1200, 800));
        album.frame(photo, *rect, Crop::default());
    }
    (album, frames)
}

#[test]
fn rd_b8_previa_igual_a_impressao() {
    let (album, frames) = three_frames();
    let print = album.render(RenderTarget::Print).unwrap();
    let preview = album
        .render(RenderTarget::Preview { max_side: 1200 })
        .unwrap();
    assert_eq!(preview.dimensions(), (1200, 600));
    for rect in frames {
        let (a, b) = (
            at(&print, 0.0, rect, 0.5, 0.5),
            at(&preview, 0.0, rect, 0.5, 0.5),
        );
        assert!(close(a, b, 3), "{rect:?}: impressão {a:?} × prévia {b:?}");
    }
}

#[test]
fn rd_b9_foto_ilegivel() {
    let mut album = Album::new(0.0);
    let a = album.photo("a.jpg", solid(30, 20, RED));
    album.frame(a, [0.1, 0.1, 0.2, 0.2], Crop::default());
    let b = album.project.add_photo("b.jpg", 30, 20);
    album.frame(b, [0.6, 0.1, 0.2, 0.2], Crop::default());
    let err = album.render(RenderTarget::Print).unwrap_err();
    assert!(
        matches!(&err, RenderError::Photo { name, reason } if name == "b.jpg" && reason == "arquivo sumiu")
    );
    assert_eq!(
        err.to_string(),
        "Não foi possível ler a foto b.jpg: arquivo sumiu."
    );
}

#[test]
fn rd_b11_lamina_inexistente_e_previa_pequena_demais() {
    let mut album = Album::new(0.0);
    for _ in 1..10 {
        album.project.add_spread();
    }
    let source = |_: &Photo, _: (u32, u32)| Err::<Arc<RgbaImage>, String>("sem fotos".to_owned());
    let err = render_spread(&album.project, 11, RenderTarget::Print, &source).unwrap_err();
    assert!(matches!(
        err,
        RenderError::NoSpread {
            number: 12,
            count: 10
        }
    ));
    assert_eq!(err.to_string(), "A lâmina 12 não existe; o projeto tem 10.");
    let err = album
        .render(RenderTarget::Preview { max_side: 8 })
        .unwrap_err();
    assert!(matches!(err, RenderError::PreviewTooSmall));
    assert_eq!(
        err.to_string(),
        "A prévia precisa ter pelo menos 16 pixels."
    );
}

fn fastest(album: &Album, target: RenderTarget, goal: Duration) -> (Duration, RgbaImage) {
    let mut best = (Duration::MAX, RgbaImage::new(1, 1));
    for _ in 0..TRIES {
        let start = Instant::now();
        let img = album.render(target).unwrap();
        let elapsed = start.elapsed();
        if elapsed < best.0 {
            best = (elapsed, img);
        }
        if best.0 < goal {
            break;
        }
    }
    best
}

#[test]
fn rd_b12_rapido_o_bastante() {
    let mut album = Album::new(3.0);
    // Seis fotos diferentes de 24 megapixels, 3 por página (auditoria do Marco 1, M5: antes era
    // uma foto só, compartilhada pelos 6 quadros).
    for col in 0..2u32 {
        for row in 0..3u32 {
            let n = col * 3 + row;
            let mut image = gradient(6000, 4000);
            image.put_pixel(n, n, Rgba([255, 255, 255, 255]));
            let id = album.photo(&format!("g{n}.jpg"), image);
            let x = 0.03 + f64::from(col) * 0.5;
            let y = 0.04 + f64::from(row) * 0.32;
            album.frame(id, [x, y, 0.44, 0.29], Crop::default());
        }
    }
    let (print_time, _) = fastest(&album, RenderTarget::Print, budget(2000));
    let preview_target = RenderTarget::Preview { max_side: 2000 };
    let (preview_time, preview) = fastest(&album, preview_target, budget(300));
    assert!(print_time < budget(2000), "impressão: {print_time:?}");
    assert!(preview_time < budget(300), "prévia: {preview_time:?}");
    let again = album
        .render(RenderTarget::Preview { max_side: 2000 })
        .unwrap();
    assert_eq!(again, preview, "mesmas entradas, pixels diferentes");
}
