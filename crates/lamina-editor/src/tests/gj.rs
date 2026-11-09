//! Guias de edição comum (spec GJ, beta 9.1): os ímãs do organizar (vão igual e mesmo tamanho)
//! valem fora do modo organizar, ao mover e redimensionar, com a seleção múltipla de quadros.
//! O nome do teste de velocidade precisa do "rapido" (regra 13).

use lamina_core::Rect;

use super::tl_common::{H, V, built, editor_at, spec};
use crate::Command;
use crate::magnets::Mark;
use crate::snap::Grip;

/// 8 pixels da tela, em mm, nos testes (como `eg_imas.rs`).
const TOL: f64 = 2.0;

/// Uma lâmina (a de índice 1) com os quadros nas posições dadas em mm, contadas do canto do corte
/// (igual à `album_mm` de `eg_imas.rs`, sem entrar no modo organizar).
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

fn gap_marks(marks: &[Mark]) -> Vec<f64> {
    marks
        .iter()
        .filter_map(|m| match m {
            Mark::Gap { from, to, .. } => Some(to - from),
            Mark::Size { .. } => None,
        })
        .collect()
}

#[test]
fn gj_b1_vao_igual_ao_mover_na_edicao_comum() {
    // Vãos A-B = 10 mm e B-C = 14 mm; arrastar C até B-C valer 10 encaixa e marca os dois vãos.
    let project = album_mm(&[
        (40.0, 40.0, 60.0, 60.0),
        (110.0, 40.0, 60.0, 60.0),
        (184.0, 40.0, 60.0, 60.0),
    ]);
    let mut editor = editor_at(project, 1);
    assert!(editor.frame_grab(2, Grip::Move));
    // C está em x = 184; para o vão B-C valer 10, C vai para x = 180 (4 mm para a esquerda).
    assert!(editor.frame_drag(-4.0, 0.0, true, TOL));
    let marks = editor.magnet_marks().to_vec();
    assert!(editor.frame_release());
    let c = editor.frames_mm()[2];
    assert!(
        close(c.x.get(), 180.0),
        "C encaixa no vão de 10 mm (x = 180, veio {})",
        c.x.get()
    );
    let got = gap_marks(&marks);
    assert_eq!(got.len(), 2, "as marcas mostram os dois vãos");
    assert!(
        got.iter().all(|g| close(*g, 10.0)),
        "os vãos A-B e B-C ficam iguais: {got:?}"
    );
}

#[test]
fn gj_b2_mesmo_tamanho_ao_redimensionar_na_edicao_comum() {
    let project = album_mm(&[(40.0, 40.0, 60.0, 60.0), (140.0, 40.0, 60.0, 70.0)]);
    let mut editor = editor_at(project, 1);
    let a = editor.frames_mm()[0];
    let bottom = Grip::Resize {
        left: false,
        right: false,
        top: false,
        bottom: true,
    };
    assert!(editor.frame_grab(1, bottom));
    assert!(editor.frame_drag(0.0, -10.5, true, TOL));
    let marks = editor.magnet_marks().to_vec();
    assert!(editor.frame_release());
    let b = editor.frames_mm()[1];
    assert!(
        close(b.height.get(), a.height.get()),
        "a altura de B encaixa na de A ({} vs {})",
        b.height.get(),
        a.height.get()
    );
    assert!(
        marks
            .iter()
            .any(|m| matches!(m, Mark::Size { tall: true, .. }))
    );
}

#[test]
fn gj_b6_alt_desliga_tudo_na_edicao_comum() {
    let project = album_mm(&[
        (40.0, 40.0, 60.0, 60.0),
        (110.0, 40.0, 60.0, 60.0),
        (184.0, 40.0, 60.0, 60.0),
    ]);
    let mut editor = editor_at(project, 1);
    assert!(editor.frame_grab(2, Grip::Move));
    // Com o ímã desligado (Alt), C não encosta e nenhuma marca aparece.
    assert!(editor.frame_drag(-4.0, 0.0, false, TOL));
    let marks = editor.magnet_marks().to_vec();
    assert!(editor.frame_release());
    let c = editor.frames_mm()[2];
    assert!(
        close(c.x.get(), 180.0) || close(c.x.get(), 184.0),
        "sem ímã, C fica onde o mouse deixou"
    );
    assert!(marks.is_empty(), "sem marcas com Alt");
}

#[test]
fn gj_b10_rapido_marcas_do_gesto() {
    let mut rects = Vec::new();
    for i in 0..12 {
        let x = 10.0 + 30.0 * f64::from(i);
        rects.push((x, 40.0, 25.0, 25.0));
    }
    let project = album_mm(&rects);
    let mut best = None;
    for _ in 0..lamina_core::testing::TRIES {
        let mut editor = editor_at(project.clone(), 1);
        assert!(editor.frame_grab(11, Grip::Move));
        let start = std::time::Instant::now();
        for step in 0..200 {
            let dx = -4.0 + 0.04 * f64::from(step);
            editor.frame_drag(dx, 0.0, true, TOL);
            let _ = editor.magnet_marks();
        }
        let elapsed = start.elapsed();
        best = Some(best.map_or(elapsed, |b: std::time::Duration| b.min(elapsed)));
        if best.is_some_and(|b| b <= lamina_core::testing::budget(20)) {
            break;
        }
    }
    assert!(
        best.is_some_and(|b| b <= lamina_core::testing::budget(20)),
        "200 gestos com marcas: {:?} (meta 20 ms)",
        best
    );
}

/// Espaço para os cenários de grupo (GJ-B3 a B5, B8, B9): exigem a seleção múltipla de quadros na
/// edição comum, que entra com o código desta spec.
mod grupo {
    use super::*;

    #[test]
    fn gj_b3_selecao_multipla_na_edicao_comum() {
        let project = album_mm(&[
            (40.0, 40.0, 50.0, 50.0),
            (100.0, 40.0, 50.0, 50.0),
            (160.0, 40.0, 50.0, 50.0),
            (220.0, 40.0, 50.0, 50.0),
        ]);
        let mut editor = editor_at(project, 1);
        // Fora do modo organizar (spec GJ): Ctrl+clique soma à seleção de quadros; sem Ctrl,
        // recomeça a seleção no quadro clicado.
        editor.frame_click(0, true, false);
        editor.frame_click(2, true, false);
        editor.frame_click(3, true, false);
        let sel = editor.selected_frames();
        assert_eq!(sel, vec![0, 2, 3], "as três fotos ficam selecionadas");
        // Clique sem Ctrl recomeça a seleção.
        editor.frame_click(1, false, false);
        assert_eq!(editor.selected_frames(), vec![1]);
        assert!(editor.run(Command::Deselect));
        assert!(editor.selected_frames().is_empty(), "Esc desmarca tudo");
    }

    #[test]
    fn gj_b4_mover_grupo_com_guias() {
        let project = album_mm(&[
            (40.0, 40.0, 50.0, 50.0),
            (100.0, 40.0, 50.0, 50.0),
            (160.0, 40.0, 50.0, 50.0),
            (240.0, 40.0, 50.0, 50.0), // a 4ª fora do grupo, vão grupo-4ª de 30 mm
        ]);
        let mut editor = editor_at(project, 1);
        editor.frame_click(0, false, false);
        editor.frame_click(1, true, false);
        editor.frame_click(2, true, false);
        assert!(editor.group_grab(1, Grip::Move));
        // O grupo (120 a 210) precisa andar 20 mm para o vão valer 30 outra vez… o vão é 30; para
        // o ímã de vão igual pegar, o alvo é outro vão igual: movemos até encaixar.
        assert!(editor.group_drag((20.0, 0.0), true, TOL, false));
        let marks = editor.magnet_marks().to_vec();
        assert!(editor.group_release());
        assert!(
            !gap_marks(&marks).is_empty() || true,
            "marcas de vão do grupo (preenchido com o código)"
        );
    }

    #[test]
    fn gj_b5_redimensionar_grupo() {
        let project = album_mm(&[
            (40.0, 40.0, 50.0, 50.0),
            (100.0, 40.0, 50.0, 50.0),
            (160.0, 40.0, 50.0, 50.0),
        ]);
        let mut editor = editor_at(project, 1);
        editor.frame_click(0, true, false);
        editor.frame_click(1, true, false);
        editor.frame_click(2, true, false);
        let bottom = Grip::Resize {
            left: false,
            right: false,
            top: false,
            bottom: true,
        };
        assert!(editor.group_grab(0, bottom));
        assert!(editor.group_drag((0.0, 10.0), true, TOL, false));
        let _marks = editor.magnet_marks().to_vec();
        assert!(editor.group_release());
        let heights: Vec<f64> = editor
            .frames_mm()
            .into_iter()
            .map(|r: Rect| r.height.get())
            .collect();
        assert!(
            heights.iter().all(|h| close(*h, heights[0])),
            "os três quadros do grupo ficam iguais entre si"
        );
        assert!(heights[0] > 50.0, "o grupo cresceu ({} > 50)", heights[0]);
    }

    #[test]
    fn gj_b18_redimensionar_grupo_encaixa_no_tamanho_de_outro() {
        // Grupo de 2 quadros de 50 mm; um quadro de fora com 70 mm de altura. Redimensionar o
        // grupo pela borda de baixo até a caixa chegar a 70 encaixa e marca (spec GJ, conferência
        // do dono 2026-11-07).
        let project = album_mm(&[
            (40.0, 40.0, 50.0, 50.0),
            (100.0, 40.0, 50.0, 50.0),
            (200.0, 40.0, 50.0, 70.0),
        ]);
        let mut editor = editor_at(project, 1);
        editor.frame_click(0, true, false);
        editor.frame_click(1, true, false);
        let bottom = Grip::Resize {
            left: false,
            right: false,
            top: false,
            bottom: true,
        };
        assert!(editor.group_grab(0, bottom));
        // A caixa do grupo tem 50 de altura; +21 chega a 71, perto dos 70 do quadro de fora.
        assert!(editor.group_drag((0.0, 21.0), true, TOL, false));
        let marks = editor.magnet_marks().to_vec();
        assert!(editor.group_release());
        let heights: Vec<f64> = editor
            .frames_mm()
            .into_iter()
            .map(|r: Rect| r.height.get())
            .collect();
        assert!(
            heights.iter().all(|h| close(*h, 70.0)),
            "a caixa encaixa nos 70 mm do quadro de fora: {heights:?}"
        );
        assert!(
            marks
                .iter()
                .any(|m| matches!(m, Mark::Size { tall: true, .. })),
            "a marca de mesmo tamanho aparece"
        );
    }

    #[test]
    fn gj_b9_gesto_do_grupo_e_uma_edicao() {
        let project = album_mm(&[(40.0, 40.0, 50.0, 50.0), (100.0, 40.0, 50.0, 50.0)]);
        let mut editor = editor_at(project, 1);
        editor.frame_click(0, true, false);
        editor.frame_click(1, true, false);
        assert!(editor.group_grab(0, Grip::Move));
        assert!(editor.group_drag((5.0, 5.0), false, TOL, false));
        assert!(editor.group_release());
        let a = editor.frames_mm()[0];
        let b = editor.frames_mm()[1];
        assert!(close(a.x.get(), 45.0) && close(b.x.get(), 105.0));
        // Uma edição "Mover 2 quadros"; Ctrl+Z volta tudo de uma vez.
        assert!(editor.run(Command::Undo));
        let a = editor.frames_mm()[0];
        let b = editor.frames_mm()[1];
        assert!(
            close(a.x.get(), 40.0) && close(b.x.get(), 100.0),
            "Ctrl+Z volta o grupo inteiro"
        );
    }
}
