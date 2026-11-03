//! Cenários da spec `aprovacao-cliente` (AC), o código da prova: o que o muda e o que não muda.

use lamina_core::{Crop, FrameContent, Mm, Rgb, TextBox};

use super::ac::with_cover;
use super::ex::{Album, solid, spec};
use crate::proof_code;

/// Um álbum com foto, recorte, texto e capa, com a impressão digital preenchida.
fn coded() -> Album {
    let mut album = Album::new(spec(), 2);
    let first = album.photo("pasta/a.jpg", solid(600, 400, [1, 2, 3]));
    album.photo("pasta/b.jpg", solid(600, 400, [4, 5, 6]));
    for photo in &mut album.project.photos {
        photo.fingerprint = Some(format!("digital-{}", photo.path.display()));
    }
    album.photo_frame(0, first, [0.1, 0.1, 0.3, 0.3]);
    let text = FrameContent::Text(TextBox {
        text: "Ana".to_owned(),
        ..TextBox::new(Rgb::BLACK)
    });
    album.frame(1, text, [0.1, 0.1, 0.3, 0.3]);
    with_cover(&mut album);
    album
}

/// O nome de uma mudança e a mudança.
type Change = (&'static str, fn(&mut lamina_core::Project));

#[test]
fn ac_b28_o_que_muda_o_codigo() {
    let album = coded();
    let base = proof_code(&album.project).unwrap();
    assert_eq!(base.len(), 6);
    assert!(
        base.chars()
            .all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)),
        "{base}"
    );
    let changes: [Change; 6] = [
        ("foto", |p| {
            let other = p.photos[1].id;
            if let FrameContent::Photo { photo, .. } = &mut p.spreads[0].frames[0].content {
                *photo = other;
            }
        }),
        ("recorte", |p| {
            if let FrameContent::Photo { crop, .. } = &mut p.spreads[0].frames[0].content {
                *crop = Crop {
                    zoom: 2.0,
                    center_x: 0.3,
                    center_y: 0.3,
                };
            }
        }),
        ("ajuste", |p| p.photos[0].adjust.brightness = 10),
        ("texto", |p| {
            if let FrameContent::Text(text) = &mut p.spreads[1].frames[0].content {
                text.text = "Pedro".to_owned();
            }
        }),
        ("fundo", |p| {
            p.spreads[1].background = Rgb { r: 9, g: 9, b: 9 }
        }),
        ("capa", |p| {
            p.cover.as_mut().unwrap().measures.wrap = Mm(20.0);
        }),
    ];
    for (what, change) in changes {
        let mut project = album.project.clone();
        change(&mut project);
        assert_ne!(proof_code(&project).unwrap(), base, "{what}");
    }
}

#[test]
fn ac_b29_o_que_nao_muda_o_codigo() {
    let album = coded();
    let base = proof_code(&album.project).unwrap();
    let changes: [Change; 4] = [
        ("pasta das fotos", |p| {
            for photo in &mut p.photos {
                photo.path = std::path::Path::new("outra").join(photo.path.file_name().unwrap());
            }
        }),
        ("nome do álbum", |p| p.name = "Outro nome".to_owned()),
        ("foto não usada", |p| {
            p.add_photo("nova.jpg", 10, 10);
            p.photos[1].adjust.brightness = 30;
        }),
        ("preferências", |p| {
            p.export.space = lamina_core::ColorSpace::AdobeRgb;
        }),
    ];
    for (what, change) in changes {
        let mut project = album.project.clone();
        change(&mut project);
        assert_eq!(proof_code(&project).unwrap(), base, "{what}");
    }

    // A foto sem impressão digital entra pelo nome e pelo tamanho.
    let mut bare = album.project.clone();
    bare.photos[0].fingerprint = None;
    let named = proof_code(&bare).unwrap();
    assert_ne!(named, base);
    bare.photos[0].path = std::path::PathBuf::from("longe/a.jpg");
    assert_eq!(proof_code(&bare).unwrap(), named);
    bare.photos[0].width_px = 700;
    assert_ne!(proof_code(&bare).unwrap(), named);
}
