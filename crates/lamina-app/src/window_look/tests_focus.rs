//! O foco logo ao abrir, com a janela de verdade e sem tela (spec DD, regras 7 e 9). Um `focus()` no `init`
//! pode se perder dentro de um `if`: o diálogo abre, e Esc e Enter só respondem depois do primeiro Tab.

use lamina_editor::commands::Command;
use slint::ComponentHandle;
use slint::platform::Key;

use super::tests::{NOTEBOOK, SCREENS, example, press, remove_one_photo, session, settle};
use crate::NewProjectUi;

/// Esc logo depois de abrir, sem Tab nem clique, muda a janela.
#[test]
fn dd_b3_esc_funciona_em_cada_tela_logo_ao_abrir() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    // Reencontrar precisa de uma foto faltando.
    let missing_dir = tempfile::tempdir().unwrap();
    let missing = example(missing_dir.path());
    remove_one_photo(&missing);
    let mut deaf = Vec::new();
    for (name, open) in SCREENS {
        // Sem Esc de propósito (regra 9).
        if matches!(*name, "RecoverDialog" | "BusyOverlay") {
            continue;
        }
        let opened_album = if *name == "RelinkDialog" {
            &missing
        } else {
            &album
        };
        let (screen, app) = session(NOTEBOOK, dir.path(), Some(opened_album));
        settle(&screen);
        open(&screen, &app);
        settle(&screen);
        let opened = screen.draw();
        press(&screen, Key::Escape);
        settle(&screen);
        if screen.draw() == opened {
            deaf.push(*name);
        }
    }
    assert_eq!(deaf, Vec::<&str>::new(), "abriu sem foco");
}

/// A especificação abre em "Personalizado" (o `FirstComboBox` num `if`) e em "Tamanhos prontos".
#[test]
fn dd_b3_esc_fecha_a_especificacao_nas_duas_abas() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    for tab in [0, 1] {
        let (screen, app) = session(NOTEBOOK, dir.path(), Some(&album));
        crate::input::run_command(&app, Command::Specification);
        let ui = screen.app.global::<NewProjectUi>();
        ui.set_show(false);
        settle(&screen);
        ui.set_tab(tab);
        ui.set_show(true);
        settle(&screen);
        assert!(ui.get_show());
        press(&screen, Key::Escape);
        settle(&screen);
        assert!(!ui.get_show(), "aba {tab}: Esc não fechou");
    }
}

/// Trocar de aba com o diálogo aberto também não deixa ele surdo.
#[test]
fn dd_b3_esc_fecha_depois_de_trocar_de_aba() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session(NOTEBOOK, dir.path(), Some(&album));
    crate::input::run_command(&app, Command::Specification);
    let ui = screen.app.global::<NewProjectUi>();
    settle(&screen);
    ui.set_tab(1 - ui.get_tab());
    settle(&screen);
    press(&screen, Key::Escape);
    settle(&screen);
    assert!(!ui.get_show(), "Esc não fechou");
}
