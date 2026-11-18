//! Cenários da spec `pdf-de-impressao` (PD) na exportação: B1 a B9 e B11. O B10 (janela = linha
//! de comando) roda o binário `lamina`. O PDF é lido de volta pelo `lopdf`, que também confere a
//! tabela de referências escrita à mão.

use std::path::Path;

use lamina_core::testing::{HEAVY_TRIES, budget};
use lamina_core::{AlbumEdge, AlbumSpec, Dpi, Mm, PageSide};
use lopdf::{Document, Object, ObjectId};

use super::ex::{Album, solid};
use crate::{ExportError, ExportFormat, ExportOptions};

/// 30 × 30 cm, 3 mm de sangria, a 50 DPI (rápido; as medidas em mm não dependem do DPI).
fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(50).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap()
}

/// `spreads` lâminas, cada uma com uma foto de cor própria.
fn album(spec: AlbumSpec, spreads: usize) -> Album {
    let mut album = Album::new(spec, spreads);
    for s in 0..spreads {
        let tone = u8::try_from(20 + s * 30).unwrap();
        let p = album.photo(&format!("f{s}.jpg"), solid(60, 40, [tone, 80, 220 - tone]));
        album.photo_frame(s, p, [0.1, 0.2, 0.8, 0.6]);
    }
    album
}

fn pdf_options() -> ExportOptions {
    ExportOptions {
        format: ExportFormat::Pdf,
        ..ExportOptions::default()
    }
}

fn mm(pt: f32) -> f64 {
    f64::from(pt) * 25.4 / 72.0
}

fn rect(doc: &Document, page: ObjectId, key: &[u8]) -> [f64; 4] {
    let dict = doc.get_dictionary(page).unwrap();
    let array = dict.get(key).unwrap().as_array().unwrap();
    let v: Vec<f64> = array.iter().map(|o| mm(o.as_float().unwrap())).collect();
    [v[0], v[1], v[2], v[3]]
}

fn close(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01)
}

/// Os bytes da imagem da página (o JPEG embutido).
fn image_of(doc: &Document, page: ObjectId) -> Vec<u8> {
    let (resources, _) = doc.get_page_resources(page).unwrap();
    let xobjects = resources
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap();
    let id = xobjects.get(b"Im0").unwrap().as_reference().unwrap();
    doc.get_object(id)
        .unwrap()
        .as_stream()
        .unwrap()
        .content
        .clone()
}

fn load(dir: &Path) -> Document {
    Document::load(dir.join("teste.pdf")).unwrap()
}

#[test]
fn pd_b1_um_pdf_com_uma_pagina_por_lamina() {
    let dir = tempfile::tempdir().unwrap();
    let report = album(spec(), 3).export(dir.path(), &pdf_options()).unwrap();
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["teste.pdf"]);
    assert_eq!(report.files, [dir.path().join("teste.pdf")]);
    let doc = load(dir.path());
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 3);
    for page in pages.values() {
        assert!(close(
            rect(&doc, *page, b"MediaBox"),
            [0.0, 0.0, 606.0, 306.0]
        ));
    }
}

#[test]
fn pd_b2_caixas_de_corte_e_de_sangria() {
    let dir = tempfile::tempdir().unwrap();
    album(spec(), 3).export(dir.path(), &pdf_options()).unwrap();
    let doc = load(dir.path());
    for page in doc.get_pages().values() {
        assert!(close(
            rect(&doc, *page, b"BleedBox"),
            [0.0, 0.0, 606.0, 306.0]
        ));
        assert!(close(
            rect(&doc, *page, b"TrimBox"),
            [3.0, 3.0, 603.0, 303.0]
        ));
    }
}

#[test]
fn pd_b3_a_mesma_imagem_do_jpeg() {
    let (jpeg, pdf) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let album = album(spec(), 3);
    album
        .export(jpeg.path(), &ExportOptions::default())
        .unwrap();
    album.export(pdf.path(), &pdf_options()).unwrap();
    let doc = load(pdf.path());
    let page2 = doc.get_pages()[&2];
    let file = std::fs::read(jpeg.path().join("lamina-02.jpg")).unwrap();
    assert!(
        image_of(&doc, page2) == file,
        "a imagem da página 2 é o lamina-02.jpg"
    );
    let image = doc.get_dictionary(page2).unwrap();
    assert!(image.has(b"Resources"));
}

#[test]
fn pd_b4_perfil_de_saida_e_pdf_x4() {
    let dir = tempfile::tempdir().unwrap();
    album(spec(), 1).export(dir.path(), &pdf_options()).unwrap();
    let bytes = std::fs::read(dir.path().join("teste.pdf")).unwrap();
    assert!(bytes.starts_with(b"%PDF-1.6\n"));
    let doc = Document::load_mem(&bytes).unwrap();
    let catalog = doc.catalog().unwrap();
    let intents = catalog.get(b"OutputIntents").unwrap().as_array().unwrap();
    let intent = match &intents[0] {
        Object::Dictionary(d) => d.clone(),
        other => doc
            .get_dictionary(other.as_reference().unwrap())
            .unwrap()
            .clone(),
    };
    assert_eq!(intent.get(b"S").unwrap().as_name().unwrap(), b"GTS_PDFX");
    let profile = intent
        .get(b"DestOutputProfile")
        .unwrap()
        .as_reference()
        .unwrap();
    let icc = &doc
        .get_object(profile)
        .unwrap()
        .as_stream()
        .unwrap()
        .content;
    assert_eq!(&icc[12..16], b"prtr", "perfil de impressora");
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    let version = doc
        .get_dictionary(info)
        .unwrap()
        .get(b"GTS_PDFXVersion")
        .unwrap();
    assert_eq!(version.as_str().unwrap(), b"PDF/X-4");
    let metadata = catalog.get(b"Metadata").unwrap().as_reference().unwrap();
    let xmp = &doc
        .get_object(metadata)
        .unwrap()
        .as_stream()
        .unwrap()
        .content;
    let xmp = String::from_utf8_lossy(xmp);
    assert!(xmp.contains("<pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>"));
    assert!(
        doc.trailer
            .get(b"ID")
            .is_ok_and(|id| id.as_array().is_ok_and(|a| a.len() == 2))
    );
}

#[test]
fn pd_b5_paginas_soltas() {
    let edges = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        pages: true,
        ..pdf_options()
    };
    album(edges.clone(), 3)
        .export(dir.path(), &options)
        .unwrap();
    let doc = load(dir.path());
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 4, "as páginas ativas");
    let sides = [
        PageSide::Right,
        PageSide::Left,
        PageSide::Right,
        PageSide::Left,
    ];
    for (page, side) in pages.values().zip(sides) {
        let canvas = edges.page_canvas_rect(side);
        let (w, h) = (canvas.width.get(), canvas.height.get());
        assert!(
            close(rect(&doc, *page, b"MediaBox"), [0.0, 0.0, w, h]),
            "{side:?}"
        );
        let trim = edges.page_rect(side);
        let x = trim.x.get() - canvas.x.get();
        let y = h - (trim.y.get() - canvas.y.get()) - trim.height.get();
        let want = [x, y, x + trim.width.get(), y + trim.height.get()];
        assert!(close(rect(&doc, *page, b"TrimBox"), want), "{side:?}");
    }
}

#[test]
fn pd_b6_intervalo() {
    let (jpeg, pdf) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let album = album(spec(), 6);
    let range = Some("2, 4-5".to_owned());
    let jpeg_options = ExportOptions {
        range: range.clone(),
        ..ExportOptions::default()
    };
    album.export(jpeg.path(), &jpeg_options).unwrap();
    let options = ExportOptions {
        range,
        ..pdf_options()
    };
    album.export(pdf.path(), &options).unwrap();
    let doc = load(pdf.path());
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 3);
    for (page, name) in pages
        .values()
        .zip(["lamina-02.jpg", "lamina-04.jpg", "lamina-05.jpg"])
    {
        let file = std::fs::read(jpeg.path().join(name)).unwrap();
        assert!(image_of(&doc, *page) == file, "{name}");
    }
}

#[test]
fn pd_b7_marcas_de_corte_fora_da_sangria() {
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        crop_marks: true,
        ..pdf_options()
    };
    album(spec(), 1).export(dir.path(), &options).unwrap();
    let doc = load(dir.path());
    let page = doc.get_pages()[&1];
    assert!(close(
        rect(&doc, page, b"MediaBox"),
        [0.0, 0.0, 626.0, 326.0]
    ));
    let bleed = rect(&doc, page, b"BleedBox");
    assert!(close(bleed, [10.0, 10.0, 616.0, 316.0]));
    assert!(close(
        rect(&doc, page, b"TrimBox"),
        [13.0, 13.0, 613.0, 313.0]
    ));
    let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
    let points: Vec<(f64, f64)> = content
        .operations
        .iter()
        .filter(|op| op.operator == "m" || op.operator == "l")
        .map(|op| {
            let v: Vec<f64> = op
                .operands
                .iter()
                .map(|o| mm(o.as_float().unwrap()))
                .collect();
            (v[0], v[1])
        })
        .collect();
    assert_eq!(points.len(), 16, "4 cantos × 2 traços × 2 pontos");
    for (x, y) in points {
        let inside = x > bleed[0] && x < bleed[2] && y > bleed[1] && y < bleed[3];
        assert!(!inside, "marca dentro da sangria: ({x}, {y})");
    }
}

#[test]
fn pd_b8_mesmo_pdf_byte_a_byte() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let album = album(spec(), 3);
    album.export(a.path(), &pdf_options()).unwrap();
    album.export(b.path(), &pdf_options()).unwrap();
    let (x, y) = (
        std::fs::read(a.path().join("teste.pdf")).unwrap(),
        std::fs::read(b.path().join("teste.pdf")).unwrap(),
    );
    assert!(x == y, "os dois PDFs diferem");
}

#[test]
fn pd_b9_substituir_e_recusar() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("teste.pdf"), b"antigo").unwrap();
    std::fs::write(dir.path().join("lamina-01.jpg"), b"outro").unwrap();
    let album = album(spec(), 2);
    let err = album.export(dir.path(), &pdf_options()).unwrap_err();
    assert!(matches!(&err, ExportError::Exists { name, .. } if name == "teste.pdf"));
    assert_eq!(
        std::fs::read(dir.path().join("teste.pdf")).unwrap(),
        b"antigo"
    );
    let options = ExportOptions {
        overwrite: true,
        ..pdf_options()
    };
    let report = album.export(dir.path(), &options).unwrap();
    assert!(
        report.removed.is_empty(),
        "a limpeza é só da exportação em JPEG"
    );
    assert_eq!(
        std::fs::read(dir.path().join("lamina-01.jpg")).unwrap(),
        b"outro"
    );
    assert_eq!(load(dir.path()).get_pages().len(), 2);
}

#[test]
fn pd_b9_marcas_de_corte_so_no_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        crop_marks: true,
        ..ExportOptions::default()
    };
    let err = album(spec(), 1).export(dir.path(), &options).unwrap_err();
    assert!(matches!(err, ExportError::CropMarksNeedPdf));
    assert_eq!(err.to_string(), "As marcas de corte só existem no PDF.");
}

#[test]
fn pd_b11_rapido_o_bastante() {
    let print = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut album = Album::new(print, 10);
    let photos: Vec<_> = (0..6u32)
        .map(|n| {
            let mut image = solid(6000, 4000, [120, 90, 60]);
            image.put_pixel(n, n, image::Rgba([255, 255, 255, 255]));
            album.photo(&format!("g{n}.jpg"), image)
        })
        .collect();
    for s in 0..10 {
        for (i, photo) in photos.iter().enumerate() {
            let (col, row) = (i / 3, i % 3);
            let x = 0.03 + f64::from(u32::try_from(col).unwrap()) * 0.5;
            let y = 0.04 + f64::from(u32::try_from(row).unwrap()) * 0.32;
            album.photo_frame(s, *photo, [x, y, 0.44, 0.29]);
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        overwrite: true,
        ..pdf_options()
    };
    let mut elapsed = std::time::Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let start = std::time::Instant::now();
        album.export(dir.path(), &options).unwrap();
        elapsed = elapsed.min(start.elapsed());
        if elapsed < budget(15_000) {
            break;
        }
    }
    assert!(elapsed < budget(15_000), "{elapsed:?}");
}

#[test]
fn nome_do_pdf_limpa_caractere_proibido_de_album_antigo() {
    let dir = tempfile::tempdir().unwrap();
    let mut item = album(spec(), 1);
    item.project.name = "ESPETACULO: ALICE NO PAÍS ENCANTADO DO AMANHÃ".to_owned();
    let report = item.export(dir.path(), &pdf_options()).unwrap();
    let expected = dir
        .path()
        .join("ESPETACULO ALICE NO PAÍS ENCANTADO DO AMANHÃ.pdf");
    assert!(expected.is_file());
    assert_eq!(report.files, [expected]);
    assert_eq!(
        crate::pdf::file_name_for("CON"),
        "album.pdf",
        "nome reservado do Windows cai no padrão seguro"
    );
    assert_eq!(crate::pdf::file_name_for("Casamento"), "Casamento.pdf");
}
