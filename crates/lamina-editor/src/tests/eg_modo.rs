//! O modo organizar (spec EG): entrar e sair, atalhos por modo, a seleção de vários quadros, o
//! laço e o painel (EG-B1 a EG-B6, EG-B17, EG-B21 e EG-B23).

use std::collections::HashSet;

use lamina_core::{FrameContent, Project};

use super::tl_common::{H, V, built_whole, editor_at, spec};
use crate::commands::Mode;
use crate::editor::{DoubleClick, SidePanel};
use crate::keys::{Key, Shortcut};
use crate::view::{Area, spread_view};
use crate::{Command, Editor};

fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    // Fotos inteiras: as setas precisam de espaço até as bordas do corte.
    built_whole(spec(30.0, 30.0), &photos, spreads)
}

/// No modo organizar, numa lâmina com pelo menos quatro quadros (`index` é só o ponto de partida).
fn organizing(index: usize) -> Editor {
    let project = album(4);
    let index = (index..project.spreads.len())
        .chain(0..index)
        .find(|i| project.spreads[*i].frames.len() >= 4)
        .unwrap();
    let mut editor = editor_at(project, index);
    assert!(editor.run(Command::Organize));
    editor
}

#[test]
fn eg_b1_entrar_e_sair() {
    let mut editor = editor_at(album(4), 1);
    let history = editor.session().unwrap().history_list().steps.len();
    let dirty = editor.session().unwrap().is_dirty();
    // Duplo clique no fundo (fora dos quadros) entra no modo.
    assert_eq!(editor.double_click(None), DoubleClick::Organize);
    assert!(editor.organizing());
    editor.organize_click(0, false, false);
    assert_eq!(editor.selected_frames(), vec![0]);
    // O primeiro Esc desmarca; o segundo sai.
    assert!(editor.run(Command::Deselect));
    assert!(editor.organizing());
    assert!(editor.selected_frames().is_empty());
    assert!(editor.run(Command::Deselect));
    assert!(!editor.organizing());
    // O alterna.
    assert_eq!(
        Command::Organize.shortcut(),
        Some(Shortcut::key(Key::Char('o')))
    );
    assert!(editor.run(Command::Organize));
    assert!(editor.organizing());
    assert!(editor.run(Command::Organize));
    assert!(!editor.organizing());
    // Nada entra no histórico nem no arquivo.
    let s = editor.session().unwrap();
    assert_eq!(s.history_list().steps.len(), history);
    assert_eq!(s.is_dirty(), dirty);
}

#[test]
fn eg_b2_lamina_toda_sangrada() {
    let mut editor = editor_at(album(4), 1);
    // As fotos de teste não têm arquivo: aqui elas contam como presentes. Numa foto ausente, o
    // duplo clique leva a "Localizar esta foto…" (spec RF, regra 8; RF-B7).
    editor.session_mut().unwrap().checked_missing(&[]);
    // Duplo clique na margem fora da lâmina (nenhum quadro): entra no modo.
    assert_eq!(editor.double_click(None), DoubleClick::Organize);
    // Duplo clique na foto: abre o recorte, também dentro do modo.
    assert_eq!(editor.double_click(Some(0)), DoubleClick::Crop);
    assert_eq!(editor.crop_mode(), Some(0));
}

#[test]
fn eg_b3_fora_do_modo_nada_muda() {
    let mut editor = editor_at(album(4), 1);
    assert!(!editor.organizing());
    let right = Shortcut::key(Key::Right);
    assert_eq!(
        Command::from_shortcut_in(right, editor.mode()),
        Some(Command::NextSpread)
    );
    // Arrastar o corpo de uma foto para outro quadro troca as fotos (FQ-B2).
    let photo =
        |e: &Editor, f: usize| match &e.session().unwrap().project().spreads[1].frames[f].content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        };
    let (a, b) = (photo(&editor, 0), photo(&editor, 1));
    assert!(editor.drop_frame(0, 1));
    assert_eq!((photo(&editor, 0), photo(&editor, 1)), (b, a));
    // Ctrl+clique fora do modo só seleciona um quadro.
    editor.organize_click(2, true, false);
    assert_eq!(editor.selected_frames(), vec![2]);
    assert_eq!(editor.selected_frame(), Some(2));
}

#[test]
fn eg_b4_atalhos_por_modo() {
    for mode in [Mode::Normal, Mode::Organize] {
        let mut seen = HashSet::new();
        for c in Command::ALL.into_iter().filter(|c| c.in_mode(mode)) {
            if let Some(s) = c.shortcut() {
                assert!(seen.insert(s), "{s:?} repetido no modo {mode:?} ({c:?})");
            }
        }
    }
    let right = Shortcut::key(Key::Right);
    assert_eq!(
        Command::from_shortcut_in(right, Mode::Organize),
        Some(Command::NudgeRight)
    );
    assert_eq!(
        Command::from_shortcut_in(right, Mode::Normal),
        Some(Command::NextSpread)
    );
    assert_eq!(
        Command::from_shortcut_in(Shortcut::shift(Key::Down), Mode::Organize),
        Some(Command::NudgeDownMore)
    );
    let mut editor = organizing(1);
    for c in [
        Command::BuildFromSelection,
        Command::Favorite,
        Command::Reject,
        Command::ToggleReordered,
        Command::NextLayout,
    ] {
        assert!(!editor.available(c), "{c:?} no modo organizar");
    }
    // → move 1 mm no modo organizar.
    editor.organize_click(0, false, false);
    let before = editor.frames_mm()[0];
    assert!(editor.run(Command::NudgeRight));
    let after = editor.frames_mm()[0];
    assert!((after.x.get() - before.x.get() - 1.0).abs() < 1e-9);
}

#[test]
fn eg_b5_ctrl_e_shift() {
    let mut editor = organizing(1);

    editor.organize_click(0, false, false);
    editor.organize_click(2, true, false);
    editor.organize_click(3, false, true);
    editor.organize_click(2, true, false);
    assert_eq!(editor.selected_frames(), vec![0, 3]);
    assert_eq!(editor.selection_text(), "2 quadros");
}

/// Os quadros da lâmina `index` em frações da largura, como a janela desenha.
fn rects(editor: &Editor) -> Vec<crate::view::PxRect> {
    let s = editor.session().unwrap();
    let canvas = s.project().spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    spread_view(s.project(), s.current(), area, &[])
        .unwrap()
        .frames
        .iter()
        .map(|f| f.rect)
        .collect()
}

#[test]
fn eg_b6_tudo_e_laco() {
    let mut editor = organizing(1);
    assert!(editor.run(Command::SelectAllFrames));
    let all: Vec<usize> = (0..editor.frames_mm().len()).collect();
    assert_eq!(editor.selected_frames(), all);
    assert!(editor.run(Command::Deselect));
    assert!(editor.selected_frames().is_empty());
    // Um laço que começa fora da lâmina e pega só os quadros inteiros dentro dele.
    let r = rects(&editor);
    let first = r[0];
    editor.lasso(
        (-0.1, -0.1),
        (
            first.x + first.width + 0.001,
            first.y + first.height + 0.001,
        ),
        false,
    );
    assert_eq!(editor.selected_frames(), vec![0]);
    // Com Shift, soma.
    let second = r[1];
    editor.lasso(
        (second.x - 0.001, second.y - 0.001),
        (
            second.x + second.width + 0.001,
            second.y + second.height + 0.001,
        ),
        true,
    );
    assert_eq!(editor.selected_frames(), vec![0, 1]);
    // Um laço que pega só um pedaço de um quadro não pega o quadro.
    editor.lasso(
        (first.x, first.y),
        (first.x + first.width / 2.0, first.y + 0.01),
        false,
    );
    assert!(editor.selected_frames().is_empty());
}

#[test]
fn eg_b17_setas_no_modo() {
    let mut editor = organizing(1);
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    let history = editor.session().unwrap().history_list().steps.len();
    let before = editor.frames_mm();
    for _ in 0..3 {
        assert!(editor.run(Command::NudgeRight));
    }
    assert!(editor.run(Command::NudgeDownMore));
    let after = editor.frames_mm();
    for f in [0, 1] {
        assert!((after[f].x.get() - before[f].x.get() - 3.0).abs() < 1e-9);
        assert!((after[f].y.get() - before[f].y.get() - 10.0).abs() < 1e-9);
    }
    // Uma edição só.
    let s = editor.session().unwrap();
    assert_eq!(s.history_list().steps.len(), history + 1);
    let last = s.history_list().steps.last().unwrap().label.clone();
    assert!(last.contains("Mover 2 quadros"), "{last}");
}

#[test]
fn eg_b21_selecao_estavel() {
    let mut editor = organizing(1);
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    assert!(editor.run(Command::NudgeRight));
    // Depois do desfazer, a seleção continua pelos quadros que ainda existem.
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.selected_frames(), vec![0, 1]);
    // Trocar de lâmina limpa a seleção e mantém o modo.
    assert!(editor.run(Command::NextSpread));
    assert!(editor.organizing());
    assert!(editor.selected_frames().is_empty());
}

#[test]
fn eg_b23_painel_por_modo() {
    let mut editor = editor_at(album(4), 1);
    assert_eq!(editor.side_panel(), SidePanel::Tabs);
    assert!(editor.run(Command::Organize));
    assert_eq!(editor.side_panel(), SidePanel::Organize);
    assert!(editor.run(Command::Organize));
    assert_eq!(editor.side_panel(), SidePanel::Tabs);
}

#[test]
fn eg_b1_titulo_e_clique_fora() {
    let mut editor = organizing(1);
    let index = editor.session().unwrap().current() + 1;
    assert_eq!(
        editor.organize_title(),
        format!("Organizar a lâmina {index}")
    );
    editor.organize_click(0, false, false);
    editor.click_nothing();
    assert!(editor.selected_frames().is_empty());
    assert!(editor.organizing());
    assert!(editor.run(Command::Organize));
    assert_eq!(editor.organize_title(), "");
}
