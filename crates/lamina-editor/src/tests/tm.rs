//! Cenários da spec `temperatura-e-matiz` (TM): a conta da cor, o histórico, o formato 7 e a
//! exportação igual à tela.

use std::path::Path;

use lamina_core::{CURRENT_SCHEMA_VERSION, CoreError, FrameContent, PhotoAdjust, PhotoId, Project};
use lamina_export::{ExportOptions, export, open_project};
use lamina_media::original_source;
use lamina_render::{RenderTarget, apply_tone, render_spread};

use super::common::Base;
use super::tl_common::{editor_at, three_spreads};
use crate::Editor;
use crate::editor::AdjustChange;

fn color(temperature: i32, tint: i32) -> PhotoAdjust {
    PhotoAdjust {
        temperature,
        tint,
        ..PhotoAdjust::default()
    }
}

/// Um pixel cinza (128, 128, 128) com a cor de `adjust`.
fn gray_with(adjust: &PhotoAdjust) -> [u8; 3] {
    let mut px = [128, 128, 128, 255];
    apply_tone(&mut px, adjust);
    [px[0], px[1], px[2]]
}

/// Na lâmina 1, com o primeiro quadro de foto selecionado.
fn selected() -> (Editor, PhotoId) {
    let mut editor = editor_at(three_spreads(), 0);
    let s = editor.session().unwrap();
    let (frame, photo) = s.project().spreads[0]
        .frames
        .iter()
        .enumerate()
        .find_map(|(i, f)| match &f.content {
            FrameContent::Photo { photo, .. } => Some((i, *photo)),
            _ => None,
        })
        .unwrap();
    editor.select_frame(Some(frame));
    (editor, photo)
}

#[test]
fn tm_b1_neutro_nao_mexe_nos_pixels() {
    let mut pixels: Vec<u8> = (0..=255u8).flat_map(|v| [v, 255 - v, v / 2, 200]).collect();
    let before = pixels.clone();
    apply_tone(&mut pixels, &PhotoAdjust::default());
    assert_eq!(pixels, before);
    assert!(!color(0, 0).has_color());
}

#[test]
fn tm_b2_mais_quente_e_mais_fria() {
    assert_eq!(gray_with(&color(100, 0)), [160, 128, 96]);
    assert_eq!(gray_with(&color(-100, 0)), [96, 128, 160]);
}

#[test]
fn tm_b3_verde_e_magenta() {
    assert_eq!(gray_with(&color(0, 100)), [128, 102, 128]);
    assert_eq!(gray_with(&color(0, -100)), [128, 154, 128]);
}

#[test]
fn tm_b4_um_arrasto_e_um_passo() {
    let (mut editor, photo) = selected();
    let before = editor.session().unwrap().history_list().steps.len();
    for v in [10, 25, 40] {
        assert!(editor.adjust(AdjustChange::Temperature(v)));
    }
    editor.end_adjust();
    let s = editor.session().unwrap();
    assert_eq!(s.project().photo(photo).unwrap().adjust.temperature, 40);
    let steps = &s.history_list().steps;
    assert_eq!(steps.len(), before + 1);
    assert_eq!(steps.last().unwrap().label, "Temperatura · lâmina 1");
    // O matiz é outro passo, e soltar fecha a emenda.
    assert!(editor.adjust(AdjustChange::Tint(-15)));
    assert_eq!(
        editor.session().unwrap().history_list().steps.len(),
        before + 2
    );
}

#[test]
fn tm_b5_salvar_abrir_e_a_versao_7() {
    let base = Base::with_photos(4, (600, 400));
    let mut project = base.project(1);
    project.photos[0].adjust = color(30, -15);
    let json = project.to_json().unwrap();
    assert!(json.contains("\"temperatura\": 30") && json.contains("\"matiz\": -15"));
    let current = format!("\"schema_version\": {CURRENT_SCHEMA_VERSION}");
    assert!(json.contains(&current));
    let opened = Project::from_json(&json).unwrap();
    assert_eq!(opened.photos[0].adjust, color(30, -15));
    // Um arquivo do formato 6, sem os campos, abre com os dois em zero.
    let mut plain = base.project(1);
    plain.photos[0].adjust = PhotoAdjust::default();
    let v6 = plain
        .to_json()
        .unwrap()
        .replace(&current, "\"schema_version\": 6");
    let old = Project::from_json(&v6).unwrap();
    assert_eq!(old.schema_version, CURRENT_SCHEMA_VERSION);
    assert!(old.photos.iter().all(|p| !p.adjust.has_color()));
}

/// A média de vermelho menos azul dos JPEGs exportados em `folder`.
fn warmth(folder: &Path) -> f64 {
    let file = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "jpg"))
        .unwrap();
    let img = image::open(file).unwrap().to_rgb8();
    let n = f64::from(img.width() * img.height());
    img.pixels()
        .map(|p| f64::from(p[0]) - f64::from(p[2]))
        .sum::<f64>()
        / n
}

#[test]
fn tm_b6_a_exportacao_sai_igual_a_tela() {
    let base = Base::with_photos(4, (600, 400));
    let neutral = base.project(1);
    let mut warm = neutral.clone();
    for p in &mut warm.photos {
        p.adjust = color(50, 0);
    }
    // Abertos do disco, com os caminhos das fotos resolvidos como na exportação de verdade.
    let warm = open_project(&base.save(warm, "quente.lamina")).unwrap();
    let neutral = open_project(&base.save(neutral, "neutra.lamina")).unwrap();
    let options = ExportOptions::default();
    let out = |project: &Project, name: &str| {
        let folder = base.path(name);
        export(project, &folder, &original_source(), &options, &|_, _| {}).unwrap();
        folder
    };
    let (a, b, plain) = (out(&warm, "a"), out(&warm, "b"), out(&neutral, "neutra"));
    let bytes = |f: &Path| {
        let mut files: Vec<_> = std::fs::read_dir(f)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        files
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(bytes(&a), bytes(&b), "exportar de novo é idêntico");
    assert!(warmth(&a) > warmth(&plain) + 5.0, "a exportação esquenta");
    // A prévia da tela esquenta do mesmo jeito.
    let preview = |p: &Project| {
        let img = render_spread(
            p,
            0,
            RenderTarget::Preview { max_side: 400 },
            &original_source(),
        )
        .unwrap();
        let n = f64::from(img.width() * img.height());
        img.pixels()
            .map(|px| f64::from(px[0]) - f64::from(px[2]))
            .sum::<f64>()
            / n
    };
    assert!(preview(&warm) > preview(&neutral) + 5.0);
}

#[test]
fn tm_b7_fora_da_faixa() {
    let base = Base::with_photos(4, (600, 400));
    let mut project = base.project(1);
    project.photos[0].adjust = color(150, 0);
    let json = project.to_json().unwrap();
    assert!(matches!(
        Project::from_json(&json),
        Err(CoreError::InvalidAdjust { .. })
    ));
    assert!(!color(0, -101).is_valid());
    assert!(color(100, -100).is_valid());
}
