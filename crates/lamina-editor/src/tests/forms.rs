//! Campos, passos e textos das janelas, que saíram do `lamina-app` (auditoria do Marco 2, M4).

use std::path::Path;

use lamina_core::{Mm, Rgb};

use super::tl_common::{editor_at, three_spreads};
use crate::forms::{
    BORDER_FIRST, ExportForm, LookField, SpecText, album_name_error, decimal, format_index,
    limits_text, mm_text, parse_limits, replace_question,
};
use crate::{Command, SpecFields};

#[test]
fn campos_do_novo_album_com_virgula_e_mensagens() {
    let defaults = SpecFields::default();
    let mut text = SpecText::from(&defaults);
    assert_eq!(text.width_cm, "30");
    assert_eq!(text.spine_mm, "");
    assert_eq!(text.parse().unwrap(), defaults);
    text.bleed_mm = "2,5".into();
    text.spine_mm = "12".into();
    let parsed = text.parse().unwrap();
    assert!((parsed.bleed_mm - 2.5).abs() < 1e-9);
    assert_eq!(parsed.spine_mm, Some(12.0));
    text.dpi = "300,5".into();
    assert_eq!(text.parse().unwrap_err(), "DPI: digite um número inteiro.");
    text.dpi = "300".into();
    text.width_cm = "trinta".into();
    assert_eq!(text.parse().unwrap_err(), "Largura: digite um número.");
    assert_eq!(album_name_error("  "), Some("Dê um nome ao álbum."));
    assert_eq!(album_name_error("Casamento"), None);
}

#[test]
fn campos_da_exportacao_e_a_pergunta() {
    let form = ExportForm {
        quality: 250,
        all_spreads: false,
        range: " 1, 3-4 ".into(),
        pages: true,
        format: crate::exporting::ExportFormat::Jpeg,
        crop_marks: true,
        space: lamina_core::ColorSpace::AdobeRgb,
        cover: crate::exporting::CoverExport::default(),
    };
    let options = form.options(true);
    assert_eq!(options.quality, 100);
    assert_eq!(options.range.as_deref(), Some("1, 3-4"));
    assert!(options.pages && options.overwrite);
    assert_eq!(options.format, crate::exporting::ExportFormat::Jpeg);
    assert!(!options.crop_marks, "marcas de corte só no PDF");
    assert_eq!(options.space, lamina_core::ColorSpace::AdobeRgb);
    let pdf = ExportForm {
        format: crate::exporting::ExportFormat::Pdf,
        ..form.clone()
    };
    let options = pdf.options(false);
    assert_eq!(options.format, crate::exporting::ExportFormat::Pdf);
    assert!(options.crop_marks);
    let all = ExportForm {
        all_spreads: true,
        quality: 0,
        ..form
    };
    let options = all.options(false);
    assert_eq!((options.quality, options.range), (1, None));
    let question = replace_question(1, Path::new("saida"));
    assert!(question.starts_with("A pasta saida já tem 1 arquivo deste álbum."));
    assert!(replace_question(3, Path::new("x")).starts_with("A pasta x já tem 3 arquivos"));
}

#[test]
fn passos_da_aparencia_com_limites() {
    let mut editor = editor_at(three_spreads(), 0);
    let gap = editor.look().unwrap().gap.get();
    editor.step_look(LookField::Gap, 1);
    assert!((editor.look().unwrap().gap.get() - (gap + 0.5)).abs() < 1e-9);
    for _ in 0..30 {
        editor.step_look(LookField::Radius, 1);
    }
    assert!(
        (editor.look().unwrap().radius.get() - 10.0).abs() < 1e-9,
        "máximo"
    );
    editor.step_look(LookField::Border, -1);
    assert_eq!(editor.look().unwrap().border.width, Mm(0.0), "mínimo");
    // Cor de borda sem espessura ganha a primeira espessura.
    let red = Rgb { r: 200, g: 0, b: 0 };
    assert!(editor.pick_border_color(red));
    let border = editor.look().unwrap().border;
    assert_eq!((border.width, border.color), (Mm(BORDER_FIRST), red));
    assert_eq!(mm_text(2.54), "2,5 mm");
    assert_eq!(decimal(30.0), "30");
}

#[test]
fn dica_conforme_a_selecao() {
    assert_eq!(crate::Editor::new().hint(), "");
    let mut editor = editor_at(three_spreads(), 0);
    assert!(
        editor.hint().starts_with("Arraste uma foto"),
        "{}",
        editor.hint()
    );
    editor.select_frame(Some(0));
    assert!(editor.hint().starts_with("Delete tira a foto"));
    assert!(editor.run(Command::CropMode));
    assert!(editor.hint().starts_with("Roda do mouse"));
}

#[test]
fn faixa_de_paginas_e_preferencias_da_exportacao() {
    use lamina_core::{OutputFormat, PageLimits};
    let limits = parse_limits("20", " 80 ").unwrap();
    assert_eq!((limits.min, limits.max), (Some(20), Some(80)));
    assert_eq!(limits_text(limits), ("20".to_owned(), "80".to_owned()));
    assert_eq!(parse_limits("", "").unwrap(), PageLimits::default());
    assert_eq!(
        parse_limits("50", "20").unwrap_err(),
        "O mínimo de páginas passa do máximo."
    );
    assert_eq!(
        parse_limits("vinte", "").unwrap_err(),
        "Páginas mínimas: digite um número inteiro maior que zero."
    );
    assert_eq!(format_index(OutputFormat::Tiff), 2);
    let form = ExportForm {
        quality: 95,
        all_spreads: true,
        range: String::new(),
        pages: true,
        format: crate::exporting::ExportFormat::Pdf,
        crop_marks: false,
        space: lamina_core::ColorSpace::AdobeRgb,
        cover: crate::exporting::CoverExport::default(),
    };
    let prefs = form.prefs();
    assert_eq!(prefs.format, OutputFormat::Pdf);
    assert!(prefs.pages);
}
