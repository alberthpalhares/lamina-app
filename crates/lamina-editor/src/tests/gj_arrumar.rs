//! Alinhar e Distribuir com a área da lâmina como referência (spec GJ, conferência do dono
//! 2026-11-04) — e os de sempre valendo também na edição comum.

use super::tl_common::{H, V, built, editor_at, spec};
use crate::Command;
use crate::snap::Grip;

/// Uma lâmina (a de índice 1) com os quadros nas posições dadas em mm, contadas do canto do corte.
pub(super) fn album_mm(rects: &[(f64, f64, f64, f64)]) -> lamina_core::Project {
    let photos: Vec<(u32, u32)> = (0..16).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 4);
    project.spreads[1].frames.truncate(rects.len().max(1));
    while project.spreads[1].frames.len() < rects.len() {
        crate::hand::add_empty_frame(&mut project, 1, lamina_core::PageSide::Left).unwrap();
    }
    let trim = project.spec.trim_rect();
    let (tw, th) = (trim.width.get(), trim.height.get());
    for (f, (x, y, w, h)) in project.spreads[1].frames.iter_mut().zip(rects) {
        f.rect = lamina_core::RelRect::new(x / tw, y / th, w / tw, h / th).unwrap();
    }
    project
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn gj_b11_alinhar_ao_centro_da_lamina() {
    // Dois quadros fora do centro: alinhar ao meio da lâmina centraliza os dois em y.
    let project = album_mm(&[(60.0, 50.0, 60.0, 60.0), (160.0, 220.0, 60.0, 40.0)]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(0, true, false);
    editor.frame_click(1, true, false);
    assert!(editor.run(Command::AlignMiddleToSpread));
    let frames = editor.frames_mm();
    let s = editor.session().unwrap();
    let area = s.project().spec.trim_rect();
    let cy = area.y.get() + area.height.get() / 2.0;
    assert!(
        close(frames[0].y.get() + 30.0, cy) && close(frames[1].y.get() + 20.0, cy),
        "os dois centros no meio da lâmina: {:?} (meio {cy})",
        frames
    );
}

#[test]
fn gj_b12_distribuir_na_area_da_lamina() {
    // Três quadros: distribuir na horizontal pela área; os vãos (e as pontas) ficam iguais.
    let project = album_mm(&[
        (30.0, 60.0, 50.0, 50.0),
        (150.0, 60.0, 50.0, 50.0),
        (270.0, 60.0, 50.0, 50.0),
    ]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(0, true, false);
    editor.frame_click(1, true, false);
    editor.frame_click(2, true, false);
    assert!(editor.run(Command::DistributeAcrossToSpread));
    let frames = editor.frames_mm();
    let s = editor.session().unwrap();
    let spec = &s.project().spec;
    // A área útil como o código calcula: a união das zonas seguras das páginas ativas.
    let left = spec.safe_rect(lamina_core::PageSide::Left);
    let right = spec.safe_rect(lamina_core::PageSide::Right);
    let start = left.x.get();
    let span = left.width.get() + right.width.get();
    let gap = (span - 150.0) / 4.0;
    for (i, expected) in [0.0, 1.0, 2.0].iter().enumerate() {
        let want = start + gap + *expected * (50.0 + gap);
        assert!(
            close(frames[i].x.get(), want),
            "quadro {i} em {} (esperado {want}); vãos e pontas iguais",
            frames[i].x.get()
        );
    }
}

#[test]
fn gj_b13_alinhar_na_edicao_comum_entre_selecionados() {
    // Os de sempre (entre os selecionados) agora valem fora do modo organizar (spec GJ).
    let project = album_mm(&[(60.0, 40.0, 60.0, 60.0), (160.0, 80.0, 60.0, 40.0)]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(0, true, false);
    editor.frame_click(1, true, false);
    assert!(editor.run(Command::AlignTop));
    let frames = editor.frames_mm();
    assert!(
        close(frames[0].y.get(), frames[1].y.get()),
        "os topos iguais, na edição comum"
    );
}

#[test]
fn gj_b14_arrastar_na_previa_reenquadra() {
    // A prévia com a foto de um quadro: arrastar nela muda o recorte gravado no quadro. O quadro
    // 60 × 40 com foto H tem folga no recorte.
    let project = album_mm(&[(60.0, 40.0, 60.0, 40.0), (160.0, 40.0, 60.0, 40.0)]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(1, false, false);
    let view = editor.photo_view();
    assert!(view.frame == Some(1), "a prévia vem do quadro clicado");
    let before = {
        let s = editor.session().unwrap();
        match &s.project().spreads[1].frames[1].content {
            lamina_core::FrameContent::Photo { crop, .. } => *crop,
            _ => panic!("quadro com foto"),
        }
    };
    assert!(editor.preview_drag(0.0, 0.1));
    assert!(editor.preview_release());
    let after = {
        let s = editor.session().unwrap();
        match &s.project().spreads[1].frames[1].content {
            lamina_core::FrameContent::Photo { crop, .. } => *crop,
            _ => panic!("quadro com foto"),
        }
    };
    assert_ne!(before, after, "o arrasto reenquadrou a foto");
}

#[test]
fn gj_b14b_previa_da_grade_nao_reenquadra() {
    // Sem quadro por trás (foto só da grade), o arrasto não faz nada.
    let project = album_mm(&[(60.0, 40.0, 60.0, 60.0), (160.0, 40.0, 60.0, 60.0)]);
    let mut editor = editor_at(project, 1);
    let ids = editor.shown_photos();
    editor.click_photo(&ids, 3, false, false);
    assert!(
        !editor.preview_drag(0.1, 0.05),
        "sem quadro, sem reenquadramento"
    );
}

#[test]
fn gj_b15_gesto_do_reenquadramento_e_uma_edicao() {
    // Quadro 60 × 40 com foto H: o recorte tem folga para andar.
    let project = album_mm(&[(60.0, 40.0, 60.0, 40.0), (160.0, 40.0, 60.0, 40.0)]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(1, false, false);
    assert!(editor.preview_drag(0.0, -0.1));
    assert!(editor.preview_release());
    let label = editor.session().unwrap().undo_label().map(str::to_owned);
    let undone = editor.run(Command::Undo);
    assert!(undone, "o reenquadramento desfaz (label: {label:?})");
    let crop = {
        let s = editor.session().unwrap();
        match &s.project().spreads[1].frames[1].content {
            lamina_core::FrameContent::Photo { crop, .. } => *crop,
            _ => panic!("quadro com foto"),
        }
    };
    // O quadro montado pelo motor vem com o recorte no centro: zoom 1, centro 0,5/0,5.
    assert!(
        close(crop.center_x, 0.5) && close(crop.center_y, 0.5),
        "desfazer volta o recorte: {crop:?}"
    );
}

#[test]
fn gj_b16_espaco_na_previa_do_quadro() {
    // Espaço com a prévia vinda de um quadro abre a foto daquele quadro.
    let project = album_mm(&[(60.0, 40.0, 60.0, 60.0), (160.0, 40.0, 60.0, 60.0)]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(1, false, false);
    assert!(editor.toggle_photo_view());
    let view = editor.photo_view();
    assert!(view.open);
    assert_eq!(view.frame, Some(1));
}

/// Guarda: o gesto comum (mover) continua funcionando com o mesmo quadro clicado.
#[test]
fn gj_b17_clique_no_quadro_continua_selecionando() {
    let project = album_mm(&[(60.0, 40.0, 60.0, 60.0), (160.0, 40.0, 60.0, 60.0)]);
    let mut editor = editor_at(project, 1);
    editor.frame_click(1, false, false);
    assert_eq!(editor.selected_frame(), Some(1));
    assert!(editor.frame_grab(1, Grip::Move));
    assert!(editor.frame_drag(5.0, 0.0, false, 2.0));
    assert!(editor.frame_release());
    assert!(close(editor.frames_mm()[1].x.get(), 165.0));
}
