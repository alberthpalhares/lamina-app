//! Cenários da spec `capa` (CA) na exportação e no desenho: texto em pé, arquivos, limpeza, avisos e
//! os mesmos bytes (CA-B7, B9, B10, B11, B15 e B19).

use std::path::Path;

use lamina_core::{
    AlbumSpec, CoverGuide, CoverMeasures, Crop, Dpi, FrameContent, Mm, Rgb, TextBox, TextRotation,
};
use lamina_render::text_image;
use lopdf::{Document, Object};

use super::ex::{Album, names, solid, spec};
use crate::{CoverExport, ExportFormat, ExportOptions, cover_preflight};

/// Um álbum de 3 lâminas com capa de lombada calculada de 1,5 cm e uma foto na frente.
fn album(spec: AlbumSpec) -> Album {
    let mut album = Album::new(spec, 3);
    for s in 0..3 {
        let p = album.photo(&format!("f{s}.jpg"), solid(600, 400, [200, 50, 50]));
        album.photo_frame(s, p, [0.55, 0.2, 0.3, 0.4]);
    }
    let mut measures = CoverMeasures::defaults(&album.project.spec);
    measures.spine = lamina_core::Spine::Fixed {
        mm: Mm(15.0),
        pages: album.project.page_count(),
    };
    album.project.create_cover(measures).unwrap();
    let photo = album.photo("capa.jpg", solid(900, 600, [30, 120, 200]));
    let content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    let rect = lamina_core::RelRect::new(0.55, 0.0, 0.45, 1.0).unwrap();
    let cover = album.project.cover.as_mut().unwrap();
    let id = cover.content.id;
    let mut canvas = album.project.cover_canvas().unwrap().unwrap();
    canvas.add_frame(id, rect, content).unwrap();
    album.project.absorb_cover(canvas);
    album
}

/// O corte de 30 × 30 cm a 72 DPI: a capa fica leve para as contas que não olham os pixels.
fn light() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(72).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
}

fn with_cover(cover: CoverExport) -> ExportOptions {
    ExportOptions {
        cover,
        ..ExportOptions::default()
    }
}

#[test]
fn ca_b7_texto_em_pe() {
    let text = |rotation| TextBox {
        text: "Ana & João · 2026".to_owned(),
        rotation,
        ..TextBox::new(Rgb::BLACK)
    };
    let flat = text_image(&text(TextRotation::Zero), (200.0, 15.0), (800, 60)).unwrap();
    let up = text_image(&text(TextRotation::Up), (15.0, 200.0), (60, 800)).unwrap();
    assert_eq!(up.dimensions(), (60, 800));
    let inked = flat.pixels().filter(|p| p[3] > 0).count();
    assert!(inked > 0);
    for (x, y, pixel) in flat.enumerate_pixels() {
        assert_eq!(up.get_pixel(y, 799 - x), pixel, "de baixo para cima");
    }
    let down = text_image(&text(TextRotation::Down), (15.0, 200.0), (60, 800)).unwrap();
    assert_eq!(down.get_pixel(59, 0), flat.get_pixel(0, 0));

    // O arquivo só guarda o giro quando há giro.
    let up_json = serde_json::to_string(&text(TextRotation::Up)).unwrap();
    assert!(up_json.contains("\"giro\":\"cima\""), "{up_json}");
    let zero_json = serde_json::to_string(&text(TextRotation::Zero)).unwrap();
    assert!(!zero_json.contains("giro"), "{zero_json}");
}

fn box_of(doc: &Document, name: &[u8]) -> Vec<f64> {
    let page = doc.get_pages().into_values().next().unwrap();
    let dict = doc.get_object(page).unwrap().as_dict().unwrap();
    let values = dict.get(name).unwrap().as_array().unwrap();
    values
        .iter()
        .map(|v| match v {
            Object::Real(r) => f64::from(*r),
            Object::Integer(i) => *i as f64,
            _ => 0.0,
        })
        .collect()
}

fn mm_of(points: f64) -> f64 {
    points / 72.0 * 25.4
}

fn file_names(folder: &Path) -> Vec<String> {
    let mut list = names(folder);
    list.sort();
    list
}

#[test]
fn ca_b9_exportar_junto_so_a_capa_e_sem() {
    let album = album(spec());
    let dir = tempfile::tempdir().unwrap();
    album
        .export(dir.path(), &with_cover(CoverExport::WithSpreads))
        .unwrap();
    assert_eq!(
        file_names(dir.path()),
        [
            "capa.jpg",
            "lamina-01.jpg",
            "lamina-02.jpg",
            "lamina-03.jpg"
        ]
    );
    assert_eq!(
        image::image_dimensions(dir.path().join("capa.jpg")).unwrap(),
        (7689, 3969)
    );

    let only = tempfile::tempdir().unwrap();
    album
        .export(only.path(), &with_cover(CoverExport::Only))
        .unwrap();
    assert_eq!(file_names(only.path()), ["capa.jpg"]);

    let without = tempfile::tempdir().unwrap();
    album
        .export(without.path(), &with_cover(CoverExport::Without))
        .unwrap();
    assert_eq!(
        file_names(without.path()),
        ["lamina-01.jpg", "lamina-02.jpg", "lamina-03.jpg"]
    );

    // Num álbum sem capa, a opção não muda nada.
    let mut plain = Album::new(spec(), 1);
    let p = plain.photo("a.jpg", solid(600, 400, [1, 2, 3]));
    plain.photo_frame(0, p, [0.1, 0.1, 0.3, 0.3]);
    let none = tempfile::tempdir().unwrap();
    plain
        .export(none.path(), &with_cover(CoverExport::Only))
        .unwrap();
    assert_eq!(file_names(none.path()), ["lamina-01.jpg"]);

    // No PDF, a capa vai em arquivo próprio, com as caixas da capa aberta.
    let pdf = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        format: ExportFormat::Pdf,
        ..ExportOptions::default()
    };
    album.export(pdf.path(), &options).unwrap();
    assert_eq!(file_names(pdf.path()), ["teste-capa.pdf", "teste.pdf"]);
    let doc = Document::load(pdf.path().join("teste.pdf")).unwrap();
    assert_eq!(doc.get_pages().len(), 3);
    let doc = Document::load(pdf.path().join("teste-capa.pdf")).unwrap();
    assert_eq!(doc.get_pages().len(), 1);
    let trim = box_of(&doc, b"TrimBox");
    assert!((mm_of(trim[2] - trim[0]) - 621.0).abs() < 0.1, "{trim:?}");
    assert!((mm_of(trim[3] - trim[1]) - 306.0).abs() < 0.1, "{trim:?}");
    let bleed = box_of(&doc, b"BleedBox");
    assert!(
        (mm_of(bleed[2] - bleed[0]) - 651.0).abs() < 0.1,
        "{bleed:?}"
    );
    assert!(
        (mm_of(bleed[3] - bleed[1]) - 336.0).abs() < 0.1,
        "{bleed:?}"
    );
}

#[test]
fn ca_b10_substituir_limpa_a_capa_velha() {
    let album = album(light());
    let dir = tempfile::tempdir().unwrap();
    album
        .export(dir.path(), &with_cover(CoverExport::WithSpreads))
        .unwrap();
    let pdf = ExportOptions {
        format: ExportFormat::Pdf,
        ..ExportOptions::default()
    };
    album.export(dir.path(), &pdf).unwrap();
    assert!(dir.path().join("capa.jpg").exists());
    assert!(dir.path().join("teste-capa.pdf").exists());

    let again = ExportOptions {
        overwrite: true,
        ..with_cover(CoverExport::Without)
    };
    let report = album.export(dir.path(), &again).unwrap();
    assert!(!dir.path().join("capa.jpg").exists());
    assert!(!dir.path().join("teste-capa.pdf").exists());
    assert!(report.removed.iter().any(|f| f.ends_with("capa.jpg")));
    assert!(report.removed.iter().any(|f| f.ends_with("teste-capa.pdf")));
    assert!(dir.path().join("teste.pdf").exists(), "o PDF do álbum fica");
}

#[test]
fn ca_b11_pre_impressao_da_capa() {
    let mut album = album(light());
    // A foto da capa bem pequena, para um quadro enorme.
    let tiny = album.photo("mini.jpg", solid(60, 40, [9, 9, 9]));
    let text = FrameContent::Text(TextBox {
        text: "Título".to_owned(),
        ..TextBox::new(Rgb::BLACK)
    });
    let mut canvas = album.project.cover_canvas().unwrap().unwrap();
    let id = canvas.spreads[0].id;
    let big = lamina_core::RelRect::new(0.0, 0.0, 0.45, 1.0).unwrap();
    let content = FrameContent::Photo {
        photo: tiny,
        crop: Crop::default(),
    };
    canvas.add_frame(id, big, content).unwrap();
    // O texto encosta no corte, e a dobra começa ali.
    let edge = lamina_core::RelRect::new(0.0, 0.4, 0.2, 0.1).unwrap();
    canvas.add_frame(id, edge, text).unwrap();
    album.project.absorb_cover(canvas);

    let texts: Vec<String> = cover_preflight(&album.project)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        texts.iter().any(|t| t.starts_with("Capa: a foto mini.jpg")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "Capa: texto sobre a dobra."),
        "{texts:?}"
    );
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("fora da zona segura") && t.contains("texto")),
        "{texts:?}"
    );
    // Os avisos da exportação põem a capa antes das lâminas.
    let dir = tempfile::tempdir().unwrap();
    let report = album.export(dir.path(), &ExportOptions::default()).unwrap();
    assert_eq!(report.warnings[0].spread, 0);
}

#[test]
fn texto_na_lombada_nao_avisa_zona_segura() {
    let mut album = album(light());
    let open = 621.0;
    let spine = lamina_core::RelRect::new(304.0 / open, 0.2, 13.0 / open, 0.6).unwrap();
    let text = FrameContent::Text(TextBox {
        text: "A".to_owned(),
        size_pt: 12.0,
        rotation: TextRotation::Up,
        ..TextBox::new(Rgb::BLACK)
    });
    let mut canvas = album.project.cover_canvas().unwrap().unwrap();
    let id = canvas.spreads[0].id;
    canvas.add_frame(id, spine, text).unwrap();
    album.project.absorb_cover(canvas);
    let texts: Vec<String> = cover_preflight(&album.project)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(!texts.iter().any(|t| t.contains("texto")), "{texts:?}");
}

#[test]
fn ca_b15_exportar_duas_vezes_da_o_mesmo_arquivo() {
    let album = album(light());
    for format in [ExportFormat::Jpeg, ExportFormat::Tiff, ExportFormat::Pdf] {
        let options = ExportOptions {
            format,
            ..with_cover(CoverExport::Only)
        };
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let first = album.export(a.path(), &options).unwrap();
        let second = album.export(b.path(), &options).unwrap();
        assert_eq!(first.files.len(), 1);
        assert_eq!(
            std::fs::read(&first.files[0]).unwrap(),
            std::fs::read(&second.files[0]).unwrap(),
            "{format:?}"
        );
    }
}

#[test]
fn ca_b19_o_guia_nao_muda_o_arquivo_exportado() {
    let plain = album(light());
    let mut guided = album(light());
    guided.project.cover.as_mut().unwrap().guide = Some(CoverGuide {
        path: "nao-existe.png".into(),
        includes_wrap: true,
        visible: true,
    });
    let bytes = |a: &Album| {
        let dir = tempfile::tempdir().unwrap();
        let report = a
            .export(dir.path(), &with_cover(CoverExport::Only))
            .unwrap();
        std::fs::read(&report.files[0]).unwrap()
    };
    assert_eq!(bytes(&plain), bytes(&guided));
}

#[test]
fn so_a_capa_com_substituir_nao_apaga_as_laminas() {
    let album = album(light());
    let dir = tempfile::tempdir().unwrap();
    album
        .export(dir.path(), &with_cover(CoverExport::WithSpreads))
        .unwrap();
    let again = ExportOptions {
        overwrite: true,
        ..with_cover(CoverExport::Only)
    };
    let report = album.export(dir.path(), &again).unwrap();
    assert!(report.removed.is_empty(), "{:?}", report.removed);
    assert_eq!(
        file_names(dir.path()),
        [
            "capa.jpg",
            "lamina-01.jpg",
            "lamina-02.jpg",
            "lamina-03.jpg"
        ]
    );
}

#[test]
fn o_andamento_conta_as_laminas_e_a_capa() {
    use std::sync::Mutex;

    let album = album(light());
    let source = |p: &lamina_core::Photo, _: (u32, u32)| {
        album
            .images
            .get(&p.path)
            .cloned()
            .ok_or_else(|| "arquivo sumiu".to_owned())
    };
    let dir = tempfile::tempdir().unwrap();
    let seen = Mutex::new(Vec::new());
    crate::export(
        &album.project,
        dir.path(),
        &source,
        &ExportOptions::default(),
        &|done, total| seen.lock().unwrap().push((done, total)),
    )
    .unwrap();
    assert_eq!(seen.into_inner().unwrap(), [(1, 4), (2, 4), (3, 4), (4, 4)]);
}

#[test]
fn album_sem_laminas_exporta_so_a_capa() {
    let mut album = Album::new(light(), 0);
    let measures = CoverMeasures::defaults(&album.project.spec);
    album.project.create_cover(measures).unwrap();
    let dir = tempfile::tempdir().unwrap();
    album
        .export(dir.path(), &with_cover(CoverExport::WithSpreads))
        .unwrap();
    assert_eq!(file_names(dir.path()), ["capa.jpg"]);
}
