//! CT-B9 (spec `cor-e-tiff`): o `--cor` da linha de comando. Desde 2026-11-09 a saída é sempre
//! sRGB (pedido do dono: os laboratórios pedem sRGB); `adobe-rgb` e valores inválidos dão erro.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, files, text};
use lamina_core::ColorSpace;
use lamina_export::color_profile;

#[test]
fn ct_b9_linha_de_comando() {
    let base = Base::with_photos(4, (300, 200));
    let out = base.build("saida", &["--laminas", "2", "--nome", "album"]);
    assert!(out.status.success(), "{}", text(&out.stderr));

    // Sem --cor (o padrão): TIFF em sRGB.
    let out = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "tiff",
        "--formato",
        "tiff",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        files(&base.path("tiff")),
        ["lamina-01.tif", "lamina-02.tif"]
    );
    let bytes = std::fs::read(base.path("tiff/lamina-01.tif")).unwrap();
    let mut decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(&bytes)).unwrap();
    let icc = decoder.get_tag_u8_vec(tiff::tags::Tag::IccProfile).unwrap();
    assert_eq!(icc, color_profile(ColorSpace::Srgb).unwrap());

    // O que saiu junto com a opção: adobe-rgb não é mais valor de --cor.
    let bad = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "x",
        "--cor",
        "adobe-rgb",
    ]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(
        text(&bad.stderr).contains("--cor") && text(&bad.stderr).contains("adobe-rgb"),
        "erro do --cor em vez de exportar: {}",
        text(&bad.stderr)
    );

    let bad = base.lamina(&[
        "exportar",
        "saida/album.lamina",
        "--saida",
        "x",
        "--cor",
        "adobe",
    ]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(
        text(&bad.stderr).contains("--cor") && text(&bad.stderr).contains("adobe"),
        "erro do --cor em vez de exportar: {}",
        text(&bad.stderr)
    );
}
