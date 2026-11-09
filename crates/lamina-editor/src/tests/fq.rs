//! Cenários da spec `fotos-nos-quadros` (FQ-B1 a B12).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{Crop, CropLevel, FrameContent, PhotoId, Project};
use lamina_layout::{FamilyKind, LayoutOptions, layout_options};

use super::tl_common::{H, V, built, editor_at, frames, single_start, spec, three_spreads};
use crate::edits::{layout_with, spread_photos};
use crate::photos::{Filter, photo_list};
use crate::{Command, Editor, Part, Shortcut};

fn photos_on(editor: &Editor, index: usize) -> Vec<PhotoId> {
    spread_photos(editor.session().unwrap().project(), index)
}

fn uses(editor: &Editor, id: PhotoId) -> usize {
    let project = editor.session().unwrap().project();
    photo_list(project, Filter::All)
        .into_iter()
        .find(|p| p.id == id)
        .unwrap()
        .uses
}

#[test]
fn fq_b1_colocar_foto_do_navegador() {
    let project = three_spreads();
    let mut editor = editor_at(project.clone(), 1);
    let outside = spread_photos(&project, 0)[0];
    let old = frames(&editor)[0].0;
    assert!(editor.drop_photo(0, outside));
    let s = editor.session().unwrap();
    let frame = &s.project().spreads[1].frames[0];
    assert!(
        matches!(&frame.content, FrameContent::Photo { photo, crop } if *photo == outside && *crop == Crop::default())
    );
    s.project().validate().unwrap();
    assert_eq!(uses(&editor, old), 0, "a antiga ficou sem uso");
    assert_eq!(uses(&editor, outside), 2);
    assert!(editor.run(Command::Undo));
    assert_eq!(
        editor.session().unwrap().project().spreads[1],
        project.spreads[1]
    );
}

#[test]
fn fq_b2_trocar_fotos_de_quadro() {
    let mut editor = editor_at(three_spreads(), 1);
    let before = frames(&editor);
    assert!(editor.drop_frame(0, 1));
    let after = frames(&editor);
    assert_eq!((after[0].0, after[1].0), (before[1].0, before[0].0));
    assert_eq!(
        (after[0].1, after[1].1),
        (before[0].1, before[1].1),
        "quadros iguais"
    );
    assert!(!editor.drop_frame(1, 1), "no mesmo quadro nada muda");
}

fn five_photos(editor: &Editor) -> Vec<PhotoId> {
    editor.session().unwrap().project().photos[..5]
        .iter()
        .map(|p| p.id)
        .collect()
}

#[test]
fn fq_b4_opcoes_para_a_selecao() {
    let mut editor = editor_at(three_spreads(), 1);
    let chosen = five_photos(&editor);
    editor.set_selection(chosen.clone());
    let panel = editor.panel(Part::Spread);
    let project = editor.session().unwrap().project();
    let aspects: Vec<f64> = chosen
        .iter()
        .map(|id| project.photo(*id).unwrap().aspect_ratio())
        .collect();
    let active = project.spec.edges().active_pages(1, 3);
    // O painel da janela usa as listas novas (spec RM, beta 9.1).
    let window_options = LayoutOptions {
        window: true,
        ..LayoutOptions::default()
    };
    let expected = layout_options(&project.spec, active, &aspects, &window_options).unwrap();
    assert_eq!(panel.options, expected.in_order);
    assert_eq!(panel.in_use, None);
}

#[test]
fn fq_b5_montar_a_lamina_com_a_selecao() {
    let mut editor = editor_at(three_spreads(), 1);
    let chosen = five_photos(&editor);
    editor.set_selection(chosen.clone());
    assert!(editor.run(Command::BuildFromSelection));
    assert_eq!(photos_on(&editor, 1), chosen);
    assert!(editor.selection().is_empty());
    assert!(editor.run(Command::Undo));
    editor.set_selection(chosen.clone());
    assert!(editor.run(Command::NewSpreadFromSelection));
    let s = editor.session().unwrap();
    assert_eq!((s.spread_count(), s.current()), (4, 2));
    assert_eq!(photos_on(&editor, 2), chosen);
}

#[test]
fn fq_b6_selecao_grande_demais() {
    let photos: Vec<(u32, u32)> = (0..28).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut editor = editor_at(built(spec(30.0, 30.0), &photos, 7), 2);
    let all: Vec<PhotoId> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.id)
        .collect();
    editor.set_selection(all[..25].to_vec());
    let reason = editor.panel(Part::Spread).unavailable.unwrap();
    assert_eq!(
        reason.to_string(),
        "Selecione até 24 fotos para uma lâmina."
    );
    assert!(!editor.available(Command::BuildFromSelection));
    assert!(!editor.available(Command::NewSpreadFromSelection));
}

#[test]
fn fq_b7_nova_lamina_e_apagar() {
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.run(Command::NewSpread));
    let s = editor.session().unwrap();
    assert_eq!((s.spread_count(), s.current()), (4, 2));
    assert!(s.project().spreads[2].frames.is_empty());
    assert!(editor.run(Command::DeleteSpread));
    assert_eq!(editor.session().unwrap().spread_count(), 3);
    let single = editor_at(built(spec(30.0, 30.0), &[H, H], 1), 0);
    assert!(!single.available(Command::DeleteSpread));
}

#[test]
fn fq_b8_reordenar_laminas() {
    let project = three_spreads();
    let ids: Vec<_> = project.spreads.iter().map(|s| s.id).collect();
    let mut editor = editor_at(project.clone(), 0);
    assert!(editor.move_current_to(1));
    let s = editor.session().unwrap();
    let now: Vec<_> = s.project().spreads.iter().map(|s| s.id).collect();
    assert_eq!(now, [ids[1], ids[0], ids[2]]);
    assert_eq!(s.current(), 1);
    assert_eq!(
        s.project().spreads[1],
        project.spreads[0],
        "fotos e quadros iguais"
    );
    assert!(editor.run(Command::MoveSpreadLeft));
    assert_eq!(editor.session().unwrap().current(), 0);
}

#[test]
fn fq_b9_reordenar_com_pagina_simples() {
    let photos: Vec<(u32, u32)> = (0..10).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let project = built(single_start(spec(30.0, 30.0)), &photos, 3);
    let old_first = project.spreads[0].clone();
    let mut editor = editor_at(project, 1);
    assert!(editor.move_current_to(0));
    let s = editor.session().unwrap();
    let first = &s.project().spreads[0];
    assert!(
        first.frames.iter().all(|f| f.rect.x() >= 0.5 - 1e-6),
        "nada na página inativa"
    );
    let moved = &s.project().spreads[1];
    assert_eq!(moved.id, old_first.id);
    assert_ne!(
        moved.frames, old_first.frames,
        "rearranjada para as duas páginas"
    );
}

#[test]
fn sb_b16_espelhar_uma_lamina_sem_borda() {
    let mut project = three_spreads();
    // De propósito: x + largura passa de 1 por arredondamento, dentro da tolerância do RelRect.
    let rect = lamina_core::RelRect::new(0.7, 0.0, 0.3 + 1e-12, 1.0).unwrap();
    project.spreads[1].frames[0].rect = rect;
    crate::suggestions::mirror(&mut project, 1).unwrap();
    let mirrored = project.spreads[1].frames[0].rect;
    assert_eq!(mirrored.x(), 0.0);
    assert_eq!(mirrored.width(), rect.width());
}

#[test]
fn sb_b13_a_remontagem_usa_a_primeira_opcao() {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    let ids = [
        project.add_photo("a.jpg", H.0, H.1),
        project.add_photo("b.jpg", H.0, H.1),
    ];
    project.add_spread();
    project.add_spread();
    project.add_spread();
    layout_with(&mut project, 1, &ids).unwrap();
    let options = LayoutOptions {
        gap: project.spread_gap(&project.spreads[1]),
        mounting: project.mounting,
        ..LayoutOptions::default()
    };
    let both = project.spec.edges().active_pages(1, 3);
    let aspects = [f64::from(H.0) / f64::from(H.1); 2];
    let first = layout_options(&project.spec, both, &aspects, &options)
        .unwrap()
        .in_order
        .remove(0);
    assert_eq!(first.family.kind(), FamilyKind::FullBleed);
    let placed: Vec<_> = project.spreads[1].frames.iter().map(|f| f.rect).collect();
    assert_eq!(placed, first.frames);
    // Com "fotos inteiras", a foto mantém a proporção.
    project.mounting.crop = CropLevel::Whole;
    layout_with(&mut project, 1, &ids).unwrap();
    let trim = project.spec.trim_rect();
    for frame in &project.spreads[1].frames {
        let r = frame.rect.to_absolute(&trim);
        assert!((r.width.get() / r.height.get() - aspects[0]).abs() < 0.01);
    }
}

#[test]
fn fq_b10_mover_o_corte() {
    let sizes = [H, H, H, V, H, H, H, V];
    let mut project = Project::new("teste", spec(30.0, 30.0));
    let ids: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, (w, h))| project.add_photo(format!("f{i}.jpg"), *w, *h))
        .collect();
    project.add_spread();
    project.add_spread();
    layout_with(&mut project, 0, &ids[..4]).unwrap();
    layout_with(&mut project, 1, &ids[4..]).unwrap();
    let mut editor = editor_at(project, 0);
    assert!(editor.drag_cut(0, -1));
    assert_eq!(photos_on(&editor, 0), ids[..3]);
    assert_eq!(photos_on(&editor, 1), [&ids[3..4], &ids[4..]].concat());
    let p = editor.session().unwrap().project();
    let aspects: Vec<f64> = ids[..3]
        .iter()
        .map(|id| p.photo(*id).unwrap().aspect_ratio())
        .collect();
    let both = p.spec.edges().active_pages(0, 2);
    let first = layout_options(&p.spec, both, &aspects, &LayoutOptions::default())
        .unwrap()
        .in_order
        .remove(0);
    let rects: Vec<_> = frames(&editor).into_iter().map(|(_, r)| r).collect();
    assert_eq!(rects, first.frames, "1ª opção do motor");
    assert!(editor.drag_cut(0, -10), "para no limite");
    assert_eq!(photos_on(&editor, 0).len(), 1);
    assert!(!editor.drag_cut(0, -1), "não esvazia a lâmina");
}

#[test]
fn fq_b11_registro_de_comandos() {
    // Sem repetir dentro do mesmo modo (spec EG, regra 2: as setas movem no modo organizar).
    let shortcuts: Vec<Shortcut> = Command::ALL
        .iter()
        .filter(|c| c.in_mode(crate::commands::Mode::Normal))
        .filter_map(|c| c.shortcut())
        .collect();
    let unique: std::collections::HashSet<_> = shortcuts.iter().collect();
    assert_eq!(unique.len(), shortcuts.len(), "atalhos repetidos");
    let editor = Editor::new();
    for command in [
        Command::ClearFrame,
        Command::BuildFromSelection,
        Command::NewSpread,
        Command::DeleteSpread,
        Command::MoveSpreadLeft,
    ] {
        assert!(!editor.available(command), "{} sem projeto", command.name());
    }
    let editor = editor_at(three_spreads(), 1);
    assert!(
        !editor.available(Command::ClearFrame),
        "sem quadro selecionado"
    );
    assert!(
        !editor.available(Command::BuildFromSelection),
        "sem seleção"
    );
}

#[test]
fn fq_b12_editar_rapido() {
    let photos: Vec<(u32, u32)> = (0..40).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let project = built(spec(30.0, 30.0), &photos, 10);
    let mut worst = Duration::ZERO;
    let mut best_of = |f: &mut (dyn FnMut(&mut Editor) -> bool + Send)| {
        let mut best = Duration::MAX;
        for _ in 0..TRIES {
            let mut editor = editor_at(project.clone(), 1);
            let elapsed = super::common::isolated(|| {
                let started = Instant::now();
                assert!(f(&mut editor));
                started.elapsed()
            });
            best = best.min(elapsed);
            if best < budget(50) {
                break;
            }
        }
        worst = worst.max(best);
    };
    let other = spread_photos(&project, 0)[0];
    best_of(&mut |e| e.drop_photo(0, other));
    best_of(&mut |e| e.drop_frame(0, 1));
    best_of(&mut |e| e.move_current_to(3));
    best_of(&mut |e| e.drag_cut(1, -1));
    assert!(worst < budget(50), "edição em {worst:?}");
}

#[test]
fn clique_ctrl_e_shift_no_navegador() {
    let mut editor = editor_at(three_spreads(), 0);
    let shown: Vec<PhotoId> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.id)
        .collect();
    editor.click_photo(&shown, 2, false, false);
    assert_eq!(editor.selection(), [shown[2]]);
    editor.click_photo(&shown, 5, false, true);
    assert_eq!(editor.selection(), &shown[2..=5]);
    editor.click_photo(&shown, 3, true, false);
    assert_eq!(editor.selection(), [shown[2], shown[4], shown[5]]);
    editor.click_photo(&shown, 0, true, false);
    assert_eq!(editor.selection(), [shown[0], shown[2], shown[4], shown[5]]);
}

#[test]
fn quadro_sob_o_ponteiro() {
    let editor = editor_at(super::tl_common::three_spreads_whole(), 1);
    let s = editor.session().unwrap();
    let canvas = s.project().spec.canvas_rect();
    let aspect = canvas.height.get() / canvas.width.get();
    let view = crate::spread_view(
        s.project(),
        1,
        crate::Area {
            width: 1.0,
            height: aspect,
            margin: 0.0,
        },
        &[],
    )
    .unwrap();
    for (i, f) in view.frames.iter().enumerate() {
        let (x, y) = (
            f.rect.x + f.rect.width / 2.0,
            f.rect.y + f.rect.height / 2.0,
        );
        assert_eq!(crate::frame_at(s.project(), 1, x, y), Some(i));
    }
    assert_eq!(
        crate::frame_at(s.project(), 1, 0.001, 0.001),
        None,
        "na sangria, fora dos quadros"
    );
}
