//! Cenários da spec `historico` (HI-B1 a B9).

use std::time::{Duration, Instant};

use lamina_core::Mm;
use lamina_core::testing::{TRIES, budget};

use super::common::isolated;
use super::tl_common::{H, V, built, editor_at, spec, three_spreads};
use crate::edits::spread_photos;
use crate::{Command, Editor, Part};

fn labels(editor: &Editor) -> Vec<String> {
    let list = editor.session().unwrap().history_list();
    std::iter::once(list.start.clone())
        .chain(list.steps.iter().map(|s| s.label.clone()))
        .collect()
}

/// Troca o layout da lâmina 3, coloca uma foto na lâmina 1 e aplica a aparência a todas.
fn three_edits() -> Editor {
    let mut editor = editor_at(three_spreads(), 2);
    assert!(editor.apply_option(Part::Spread, 2));
    editor.session_mut().unwrap().go_to(0);
    let other = spread_photos(editor.session().unwrap().project(), 2)[0];
    assert!(editor.drop_photo(0, other));
    assert!(editor.style_all());
    editor
}

#[test]
fn hi_b1_lista_com_rotulo_e_lamina() {
    let editor = three_edits();
    assert_eq!(
        labels(&editor),
        [
            "Álbum criado",
            "Trocar modelo · lâmina 3",
            "Colocar foto · lâmina 1",
            "Aplicar aparência a todas · todas as lâminas"
        ]
    );
    assert_eq!(editor.session().unwrap().history_list().position, 3);
}

#[test]
fn hi_b2_voltar_a_um_passo() {
    let mut editor = three_edits();
    let after_first = editor.session().unwrap().history_list();
    // O álbum logo depois do primeiro passo, por outro caminho: desfazer os dois últimos.
    let mut probe = three_edits();
    let s = probe.session_mut().unwrap();
    assert!(s.undo().is_some() && s.undo().is_some());
    let expected = s.project().clone();
    assert!(editor.go_to_step(1));
    assert_eq!(
        editor.session().unwrap().project(),
        &expected,
        "o álbum de logo depois do passo"
    );
    let s = editor.session().unwrap();
    assert_eq!(s.current(), 2, "a lâmina do passo: a 3");
    let list = s.history_list();
    assert_eq!(list.position, 1);
    assert_eq!(
        list.steps.iter().map(|x| x.undone).collect::<Vec<_>>(),
        [false, true, true]
    );
    assert_eq!(list.steps.len(), after_first.steps.len());
}

#[test]
fn hi_b3_refazer_pelo_clique() {
    let mut editor = three_edits();
    let end = editor.session().unwrap().project().clone();
    assert!(editor.go_to_step(1));
    assert!(editor.go_to_step(3));
    let s = editor.session().unwrap();
    assert_eq!(s.project(), &end);
    assert!(s.history_list().steps.iter().all(|x| !x.undone));
}

#[test]
fn hi_b4_voltar_ao_comeco_e_nova_edicao() {
    let start = three_spreads();
    let mut editor = three_edits();
    assert!(editor.go_to_step(0));
    assert_eq!(editor.session().unwrap().project(), &start);
    editor.session_mut().unwrap().go_to(1);
    assert!(editor.run(Command::Mirror));
    assert_eq!(
        labels(&editor),
        ["Álbum criado", "Espelhar lâmina · lâmina 2"]
    );
}

#[test]
fn hi_b5_marcador_de_salvo() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.run(Command::NextLayout));
    assert!(editor.run(Command::Mirror));
    editor
        .session_mut()
        .unwrap()
        .save_as(&dir.path().join("a.lamina"))
        .unwrap();
    assert!(editor.run(Command::Mirror));
    let list = editor.session().unwrap().history_list();
    assert_eq!(
        list.steps.iter().map(|x| x.saved).collect::<Vec<_>>(),
        [false, true, false]
    );
    assert!(editor.session().unwrap().is_dirty());
    assert!(editor.go_to_step(2));
    assert!(!editor.session().unwrap().is_dirty(), "de volta ao salvo");
}

#[test]
fn hi_b6_menu_editar() {
    let mut editor = three_edits();
    assert!(editor.go_to_step(1));
    let s = editor.session().unwrap();
    assert_eq!(s.undo_label(), Some("Trocar modelo"));
    assert_eq!(s.redo_label(), Some("Colocar foto"));
}

#[test]
fn hi_b7_limite_de_200() {
    let mut editor = editor_at(three_spreads(), 1);
    for i in 0..205 {
        let gap = Mm(3.0 + f64::from(i % 2));
        assert!(editor.set_gap(gap).is_none());
    }
    let list = editor.session().unwrap().history_list();
    assert_eq!(list.steps.len(), 200);
    assert_eq!(list.start, "Início do histórico guardado");
    assert!(editor.go_to_step(0));
    let gap = editor.session().unwrap().project().spreads[1].gap;
    assert_eq!(
        gap,
        Some(Mm(3.0 + f64::from(4 % 2))),
        "o estado depois da 5ª edição"
    );
}

#[test]
fn hi_b8_abrir_outro_album() {
    let dir = tempfile::tempdir().unwrap();
    let mut other = editor_at(three_spreads(), 0);
    let file = other
        .session_mut()
        .unwrap()
        .save_as(&dir.path().join("b.lamina"))
        .unwrap();
    let mut editor = three_edits();
    editor.open(&file).unwrap();
    assert_eq!(labels(&editor), ["Álbum aberto"]);
}

#[test]
fn hi_b9_rapido() {
    let photos: Vec<(u32, u32)> = (0..40).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut editor = editor_at(built(spec(30.0, 30.0), &photos, 10), 1);
    for i in 0..200 {
        editor.session_mut().unwrap().go_to(i % 10);
        assert!(editor.run(Command::Mirror) || editor.run(Command::NextLayout));
    }
    // A pior ida de cada rodada; vale a melhor rodada.
    let mut worst = Duration::MAX;
    for _ in 0..TRIES {
        let round = isolated(|| {
            let mut round = Duration::ZERO;
            for step in [0, 200, 0, 200] {
                let started = Instant::now();
                assert!(editor.go_to_step(step));
                round = round.max(started.elapsed());
            }
            round
        });
        worst = worst.min(round);
        if worst < budget(50) {
            break;
        }
    }
    assert!(worst < budget(50), "ir a um passo levou {worst:?}");
}
