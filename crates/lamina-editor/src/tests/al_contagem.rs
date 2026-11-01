//! Cenários AL-B6 (o que depende da contagem), AL-B12 a AL-B18 e AL-B28 da spec
//! `specs/aprendizado-de-layouts.md`: a visita da lâmina e o que conta ao sair.

use lamina_core::{CoverMeasures, Mm, Project, RelRect};
use lamina_layout::{FamilyKind, LayoutCandidate, design_key};
use tempfile::TempDir;

use super::tl_common::{H, V, built, spec};
use crate::learned_store::LearnedStore;
use crate::{Command, Editor, Leave, Part};

/// Um caso com nome: o que o fotógrafo faz.
type Case = (&'static str, fn(&mut Editor));

/// Quatro lâminas com 4 fotos cada, para ter para onde ir nos dois sentidos.
fn album() -> Project {
    let photos: Vec<(u32, u32)> = (0..16).map(|i| if i % 4 == 3 { V } else { H }).collect();
    built(spec(30.0, 30.0), &photos, 4)
}

/// Editor que aprende num arquivo temporário, na lâmina `index`.
pub(super) fn learning_at(project: Project, index: usize) -> (Editor, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = Editor::new();
    editor.set_learned_store(LearnedStore::load(&dir.path().join("aprendizado.json")));
    editor.adopt(project);
    assert!(editor.go_to_spread(index));
    (editor, dir)
}

/// Escolhe na faixa uma opção que não está em uso e devolve a chave dela.
pub(super) fn choose(editor: &mut Editor) -> String {
    choose_past(editor, 0)
}

/// Como [`choose`], a partir da opção `from`.
fn choose_past(editor: &mut Editor, from: usize) -> String {
    let panel = editor.panel(Part::Spread);
    let i = (from..panel.options.len())
        .find(|i| panel.in_use != Some(*i) && !is_model(&panel.options[*i]))
        .unwrap();
    let key = design_key(&panel.options[i], &panel.selection.unwrap().aspects);
    assert!(editor.choose_option(Part::Spread, i));
    key
}

fn is_model(c: &LayoutCandidate) -> bool {
    c.family.kind() == FamilyKind::Model
}

/// A chave do desenho em uso na lâmina atual.
fn in_use_key(editor: &Editor) -> String {
    let panel = editor.panel(Part::Spread);
    let i = panel.in_use.unwrap();
    design_key(&panel.options[i], &panel.selection.unwrap().aspects)
}

fn keys(editor: &Editor) -> Vec<String> {
    let panel = editor.panel(Part::Spread);
    let sel = panel.selection.unwrap();
    panel
        .options
        .iter()
        .map(|c| design_key(c, &sel.aspects))
        .collect()
}

#[test]
fn al_b12_escolher_e_sair_conta_uma_vez() {
    let (mut editor, dir) = learning_at(album(), 2);
    let key = choose(&mut editor);
    assert_eq!(editor.learned().weight(&key), 0, "só conta ao sair");
    let file = dir.path().join("album.lamina");
    editor.save_to(Some(&file)).unwrap();
    let steps = editor.session().unwrap().history_list().steps.len();
    let saved = std::fs::read(&file).unwrap();

    assert!(editor.go_to_spread(3));

    assert_eq!(editor.learned().weight(&key), 1000);
    let session = editor.session().unwrap();
    assert_eq!(session.history_list().steps.len(), steps, "sem passo novo");
    assert!(!session.is_dirty(), "contar não marca o álbum");
    assert_eq!(std::fs::read(&file).unwrap(), saved, "o .lamina não muda");
}

#[test]
fn al_b13_pelas_setas_so_a_ultima() {
    let (mut editor, _dir) = learning_at(album(), 2);
    for _ in 0..4 {
        assert!(editor.run(Command::NextLayout));
    }
    let last = in_use_key(&editor);
    assert!(editor.run(Command::NextSpread));
    assert_eq!(editor.learned().weight(&last), 1000);
    assert_eq!(editor.learned().weights().len(), 1, "só a última conta");
}

/// A escolha é seguida de `change` e de sair da lâmina: conta?
fn counted_after(change: impl FnOnce(&mut Editor)) -> bool {
    let (mut editor, _dir) = learning_at(album(), 2);
    let key = choose(&mut editor);
    change(&mut editor);
    assert!(editor.go_to_spread(3));
    editor.learned().weight(&key) > 0
}

#[test]
fn al_b14_o_que_deixou_de_estar_em_uso_nao_conta() {
    assert!(counted_after(|_| ()), "sem mudança, conta");
    let undone = |e: &mut Editor| assert!(e.run(Command::Undo));
    assert!(!counted_after(undone), "Ctrl+Z");
    assert!(
        counted_after(|e| {
            assert!(e.run(Command::Undo));
            assert!(e.run(Command::Redo));
        }),
        "Ctrl+Z e refazer"
    );
    let moved = |e: &mut Editor| {
        let edit = e.session_mut().unwrap().edit("Mover", |p: &mut Project| {
            let frame = &mut p.spreads[2].frames[0];
            let r = frame.rect;
            frame.rect = RelRect::new(r.x(), r.y(), r.width() * 0.8, r.height() * 0.8)?;
            Ok::<(), lamina_core::CoreError>(())
        });
        edit.unwrap();
    };
    assert!(!counted_after(moved), "quadro movido à mão");
    let swapped = |e: &mut Editor| {
        let edit = e.session_mut().unwrap().edit("Trocar", |p: &mut Project| {
            let frames = &mut p.spreads[2].frames;
            let first = frames[0].content.clone();
            frames[0].content = std::mem::replace(&mut frames[1].content, first);
            Ok::<(), ()>(())
        });
        edit.unwrap();
    };
    assert!(!counted_after(swapped), "fotos trocadas");
    let spaced = |e: &mut Editor| assert_eq!(e.set_gap(Mm(9.0)), None);
    assert!(!counted_after(spaced), "espaço entre fotos");
}

#[test]
fn al_b15_o_que_nao_conta() {
    let cases: [Case; 5] = [
        ("prévia", |e| {
            assert!(e.toggle_band());
            e.hover_option(Some(3));
            e.hover_option(None);
        }),
        ("Enter com seleção", |e| {
            let album = e.session().unwrap().album();
            let photos = album.photos[..3].iter().map(|p| p.id).collect();
            e.set_selection(photos);
            assert!(e.run(Command::BuildFromSelection));
        }),
        ("rejeitar o layout em uso", |e| {
            assert!(e.run(Command::Reject));
        }),
        ("espelhar", |e| assert!(e.run(Command::Mirror))),
        ("o que a IA faz", |e| {
            assert!(e.apply_option(Part::Spread, 5));
        }),
    ];
    for (name, act) in cases {
        let (mut editor, _dir) = learning_at(album(), 1);
        act(&mut editor);
        assert!(editor.go_to_spread(2));
        assert!(editor.learned().is_empty(), "{name}");
    }
    // Sem arquivo, como na IA e na linha de comando, nem as setas contam.
    let mut editor = Editor::new();
    editor.adopt(album());
    assert!(editor.run(Command::NextLayout));
    assert!(editor.run(Command::NextSpread));
    assert!(editor.learned().is_empty(), "sem arquivo");
}

/// Escolhe na lâmina 2 e sai por `leave`: quanto pesou?
fn weight_after(leave: impl FnOnce(&mut Editor)) -> u32 {
    let (mut editor, _dir) = learning_at(album(), 1);
    let key = choose(&mut editor);
    leave(&mut editor);
    editor.learned().weight(&key)
}

#[test]
fn al_b16_toda_saida_fecha_a_visita() {
    let exits: [Case; 6] = [
        ("faixa e rolo", |e| assert!(e.go_to_spread(3))),
        ("Page Down", |e| assert!(e.run(Command::NextSpread))),
        ("Home", |e| assert!(e.run(Command::FirstSpread))),
        ("fechar o álbum", |e| assert!(e.close_album())),
        ("outro álbum", |e| e.adopt(album())),
        ("sair do Lamina", Editor::remember_current),
    ];
    for (name, leave) in exits {
        assert_eq!(weight_after(leave), 1000, "{name}");
    }
    let dont_save = |e: &mut Editor| {
        assert!(e.leave(Leave::DontSave, None).unwrap());
        e.close_album();
    };
    assert_eq!(weight_after(dont_save), 0, "Não salvar");
    assert_eq!(
        weight_after(|e| assert!(e.run(Command::DeleteSpread))),
        0,
        "lâmina apagada"
    );
}

#[test]
fn al_b16_abrir_outro_arquivo_fecha_a_visita() {
    let (mut editor, dir) = learning_at(album(), 1);
    let other = dir.path().join("outro.lamina");
    crate::Session::new_unsaved(album())
        .save_as(&other)
        .unwrap();
    let key = choose(&mut editor);
    editor.open(&other).unwrap();
    assert_eq!(editor.learned().weight(&key), 1000);
}

#[test]
fn al_b16_ctrl_z_que_muda_de_lamina_fecha_a_visita() {
    let (mut editor, _dir) = learning_at(album(), 0);
    assert!(editor.run(Command::Mirror));
    assert!(editor.go_to_spread(1));
    // Escolher a opção em uso não muda os quadros: os Ctrl+Z voltam até o espelhar.
    let panel = editor.panel(Part::Spread);
    let in_use = panel.in_use.unwrap();
    let key = in_use_key(&editor);
    assert!(editor.choose_option(Part::Spread, in_use));
    while editor.session().unwrap().current() == 1 {
        assert_eq!(editor.learned().weight(&key), 0);
        assert!(editor.run(Command::Undo));
    }
    assert_eq!(editor.learned().weight(&key), 1000);
}

#[test]
fn al_b16_a_capa_nao_e_lamina() {
    let (mut editor, _dir) = learning_at(album(), 0);
    let measures = CoverMeasures::defaults(&editor.session().unwrap().album().spec);
    assert!(editor.create_cover(measures, None, &[]));
    assert!(editor.go_to_spread(0));
    let key = choose(&mut editor);
    // Da lâmina 1 para a capa: `current()` continua 0, mas a lâmina foi deixada.
    assert!(editor.run(Command::PreviousSpread));
    assert!(editor.on_cover());
    assert_eq!(editor.learned().weight(&key), 1000, "ir para a capa conta");

    // Com a capa aberta, nada conta, nem aplicando direto.
    let before = editor.learned().clone();
    if !editor.panel(Part::Spread).options.is_empty() {
        editor.choose_option(Part::Spread, 0);
    }
    assert!(editor.run(Command::NextSpread));
    assert!(!editor.on_cover());
    assert_eq!(editor.learned(), &before, "a capa não conta");
    assert!(editor.go_to_spread(1));
    assert_eq!(editor.learned(), &before, "voltar da capa não conta");
}

#[test]
fn al_b16_mover_a_lamina_nao_encerra_a_visita() {
    let (mut editor, _dir) = learning_at(album(), 1);
    let key = choose(&mut editor);
    assert!(editor.run(Command::MoveSpreadRight));
    assert_eq!(editor.learned().weight(&key), 0, "continua na mesma lâmina");
    assert_eq!(editor.session().unwrap().current(), 2);
    assert!(editor.run(Command::NextSpread));
    assert_eq!(editor.learned().weight(&key), 1000);
}

#[test]
fn al_b17_ir_e_voltar_nao_infla() {
    let (mut editor, _dir) = learning_at(album(), 2);
    let key = choose(&mut editor);
    assert!(editor.go_to_spread(3));
    assert_eq!(editor.learned().weight(&key), 1000);
    for _ in 0..3 {
        assert!(editor.go_to_spread(2));
        let at = keys(&editor).iter().position(|k| *k == key).unwrap();
        assert!(editor.choose_option(Part::Spread, at));
        assert!(editor.go_to_spread(3));
    }
    assert_eq!(editor.learned().weight(&key), 1000);
    // Num álbum novo, a contagem recomeça.
    editor.adopt(album());
    assert!(editor.go_to_spread(2));
    let at = keys(&editor).iter().position(|k| *k == key);
    assert!(editor.choose_option(Part::Spread, at.unwrap()));
    assert!(editor.go_to_spread(3));
    assert_eq!(editor.learned().weight(&key), 2000);
}

#[test]
fn al_b18_a_lista_so_muda_depois_da_visita() {
    let (mut editor, _dir) = learning_at(album(), 2);
    let panel = editor.panel(Part::Spread);
    // Depois das 12 do rodízio, a lista segue a nota: uma opção a menos de 2 pontos da anterior.
    let close = |i: &usize| {
        let (a, b) = (&panel.options[*i - 1], &panel.options[*i]);
        a.score.total - b.score.total < 1.9 && !is_model(b) && panel.in_use != Some(*i)
    };
    let at = (13..panel.options.len()).find(close).unwrap();
    let before = keys(&editor);
    assert!(editor.choose_option(Part::Spread, at));
    assert_eq!(
        keys(&editor),
        before,
        "com a lâmina aberta, a ordem é a mesma"
    );
    assert!(editor.go_to_spread(3));
    assert!(editor.go_to_spread(2));
    assert_ne!(keys(&editor), before, "ao voltar, é a nova");
}

#[test]
fn al_b28_a_faixa_nao_mostra_a_nota_e_avisa_o_que_sobe() {
    let (mut editor, _dir) = learning_at(album(), 2);
    let panel = editor.panel(Part::Spread);
    let hint = "Os layouts que você usa com frequência sobem na lista.";
    assert_eq!(panel.hint(), hint);
    for card in panel.cards() {
        let text = format!("{card:?}").to_lowercase();
        assert!(!text.contains("nota"), "{text}");
    }
    // Os cartões da janela também não têm campo para a nota.
    let band = include_str!("../../../lamina-app/ui/band.slint");
    assert!(!band.contains("score"), "a faixa da janela mostra a nota");

    // Com um aviso da lista, vem o aviso no lugar da frase.
    for _ in 0..panel.options.len() {
        if !editor.mark_option(0, false) {
            break;
        }
    }
    let rejected = editor.panel(Part::Spread);
    assert!(rejected.all_rejected);
    assert_eq!(rejected.notice(), crate::text::ALL_REJECTED);
    assert_eq!(rejected.hint(), "");

    // A nota continua ordenando e a IA continua recebendo a explicação.
    assert!(crate::score_text(&panel.options[0].score).starts_with("Nota "));
}

#[test]
fn al_b6_escolher_um_modelo_nao_conta() {
    let project = super::md::album(&[super::md::HHV, super::md::HHV]);
    let (mut editor, dir) = learning_at(project, 0);
    super::md::abertura(&mut editor);
    assert!(editor.go_to_spread(1));
    let panel = editor.panel(Part::Spread);
    let at = panel.options.iter().position(is_model).unwrap();
    assert!(editor.choose_option(Part::Spread, at));
    assert!(editor.go_to_spread(0));
    assert!(editor.learned().is_empty());
    assert!(
        !dir.path().join("aprendizado.json").exists(),
        "nada gravado"
    );
}
