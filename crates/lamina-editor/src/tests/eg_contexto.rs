//! O menu de contexto (spec EG, regra 13): EG-B20.

use super::eg_imas::album_mm;
use super::tl_common::editor_at;
use crate::editor::ContextEntry;
use crate::{Command, Editor};

/// Todos os itens (os dos submenus também), com o comando e se valem.
fn items(menu: &[ContextEntry]) -> Vec<(Command, String, bool)> {
    let mut out = Vec::new();
    for e in menu {
        match e {
            ContextEntry::Item {
                command,
                label,
                enabled,
                ..
            } => out.push((*command, label.clone(), *enabled)),
            ContextEntry::Sub { items: sub, .. } => out.extend(items(sub)),
            ContextEntry::Separator => {}
        }
    }
    out
}

fn organizing_two() -> Editor {
    let project = album_mm(&[
        (20.0, 40.0, 40.0, 50.0),
        (80.0, 60.0, 50.0, 70.0),
        (150.0, 30.0, 60.0, 20.0),
    ]);
    let mut editor = editor_at(project, 1);
    assert!(editor.run(Command::Organize));
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    editor
}

#[test]
fn eg_b20_menu_de_contexto() {
    let mut editor = organizing_two();
    // Num quadro de dentro, a seleção se mantém.
    let menu = editor.context_menu(Some(1));
    assert_eq!(editor.selected_frames(), vec![0, 1]);
    let all = items(&menu);
    for (command, _, enabled) in &all {
        assert_eq!(
            *enabled,
            editor.available(*command),
            "{command:?} vale como no menu"
        );
    }
    let has = |c: Command| all.iter().any(|(x, _, _)| *x == c);
    for c in [
        Command::CropMode,
        Command::RemoveFromAlbum,
        Command::CopyFrames,
        Command::PasteFrames,
        Command::DuplicateFrames,
        Command::BringForward,
        Command::SendBackward,
        Command::AlignLeft,
        Command::EqualSize,
        Command::DistributeAcross,
        Command::SelectAllFrames,
    ] {
        assert!(has(c), "o menu do quadro tem {c:?}");
    }
    let titles: Vec<&str> = menu
        .iter()
        .filter_map(|e| match e {
            ContextEntry::Sub { title, .. } => Some(*title),
            _ => None,
        })
        .collect();
    assert_eq!(titles, ["Alinhar", "Igualar", "Distribuir"]);
    // Com foto nos quadros, o item tira as fotos do álbum (spec TF-B11).
    assert!(
        all.iter()
            .any(|(_, l, _)| l == "Remover 2 fotografias do álbum")
    );
    assert!(!all.iter().any(|(_, l, _)| l == "Tirar foto"));

    // Num quadro de fora, só ele fica selecionado.
    editor.context_menu(Some(2));
    assert_eq!(editor.selected_frames(), vec![2]);

    // No fundo: os itens da lâmina, e Organizar vira "Voltar" no modo (spec ZR).
    let menu = editor.context_menu(None);
    assert_eq!(
        editor.selected_frames(),
        vec![2],
        "o fundo não mexe na seleção"
    );
    let all = items(&menu);
    for c in [
        Command::PasteFrames,
        Command::SelectAllFrames,
        Command::NewText,
        Command::NewPhotoFrame,
        Command::Mirror,
        Command::NewSpread,
        Command::DeleteSpread,
        Command::SaveModel,
        Command::ToggleGuides,
        Command::Organize,
    ] {
        assert!(
            all.iter().any(|(x, _, _)| *x == c),
            "o menu do fundo tem {c:?}"
        );
    }
    assert!(
        all.iter()
            .any(|(c, l, _)| *c == Command::Organize && l == "Voltar")
    );
    for (command, _, enabled) in &all {
        assert_eq!(*enabled, editor.available(*command), "{command:?}");
    }
}

#[test]
fn eg_b20_fora_do_modo_o_clique_direito_seleciona_o_quadro() {
    let project = album_mm(&[(20.0, 40.0, 40.0, 50.0), (80.0, 60.0, 50.0, 70.0)]);
    let mut editor = editor_at(project, 1);
    let menu = editor.context_menu(Some(1));
    assert_eq!(editor.selected_frames(), vec![1]);
    let all = items(&menu);
    let align = all
        .iter()
        .find(|(c, _, _)| *c == Command::AlignLeft)
        .unwrap();
    assert!(!align.2, "Alinhar só vale no modo organizar");
    let menu = editor.context_menu(None);
    let organize = items(&menu)
        .into_iter()
        .find(|(c, _, _)| *c == Command::Organize)
        .unwrap();
    assert_eq!(organize.1, "Organizar lâmina");
    assert!(organize.2);
}
