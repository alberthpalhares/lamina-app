//! Cenário da spec `pagina-em-branco` na exportação (PB-B11): a página sai lisa, com o fundo e o
//! texto dela, conta nas páginas soltas e não gera aviso.

use lamina_core::{AlbumEdge, FrameContent, Rgb, TextBox};

use super::ex::{Album, spec};
use crate::{ExportOptions, preflight};

const CREAM: Rgb = Rgb {
    r: 245,
    g: 235,
    b: 210,
};

fn album() -> Album {
    let edges = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::FullSpread);
    let mut album = Album::new(edges, 2);
    let spread = &mut album.project.spreads[0];
    spread.background = CREAM;
    spread.blank.right = true;
    let text = TextBox {
        text: "Título".to_owned(),
        ..TextBox::new(Rgb::BLACK)
    };
    album.frame(0, FrameContent::Text(text), [0.6, 0.1, 0.3, 0.1]);
    album
}

fn close_to(pixel: image::Rgb<u8>, color: Rgb) -> bool {
    let d = |a: u8, b: u8| a.abs_diff(b) <= 3;
    d(pixel[0], color.r) && d(pixel[1], color.g) && d(pixel[2], color.b)
}

#[test]
fn pb_b11_exportacao_e_pre_impressao() {
    let album = album();
    assert!(preflight(&album.project, &[0]).is_empty());

    let dir = tempfile::tempdir().unwrap();
    let again = tempfile::tempdir().unwrap();
    let options = ExportOptions::default();
    let first = album.export(dir.path(), &options).unwrap();
    let second = album.export(again.path(), &options).unwrap();
    let bytes = |report: &crate::ExportReport| std::fs::read(&report.files[0]).unwrap();
    assert_eq!(
        bytes(&first),
        bytes(&second),
        "exportar duas vezes dá os mesmos bytes"
    );

    // O meio da página em branco é o fundo creme: o rótulo da tela não sai.
    let image = image::open(&first.files[0]).unwrap().to_rgb8();
    let (w, h) = image.dimensions();
    assert!(close_to(*image.get_pixel(w * 3 / 4, h / 2), CREAM));
    // O texto saiu: algum pixel da região dele não é creme.
    let dark = (w * 6 / 10..w * 9 / 10)
        .flat_map(|x| (h / 10..h / 5).map(move |y| (x, y)))
        .any(|(x, y)| !close_to(*image.get_pixel(x, y), CREAM));
    assert!(dark, "o texto da página em branco sai na exportação");

    // Nas páginas soltas, ela é a página 1, como qualquer outra.
    let pages = tempfile::tempdir().unwrap();
    let report = album
        .export(
            pages.path(),
            &ExportOptions {
                pages: true,
                ..ExportOptions::default()
            },
        )
        .unwrap();
    let names: Vec<String> = report
        .files
        .iter()
        .filter_map(|f| f.file_name()?.to_str().map(str::to_owned))
        .collect();
    assert_eq!(names.first().map(String::as_str), Some("pagina-01.jpg"));
    assert_eq!(names.len(), 3, "1 + 2 páginas, com a em branco contada");
}
