//! A janela Exportar com as duas abas, de verdade e sem tela (spec DD, regras 7 a 10): onde o foco
//! entra, o Tab que não sai dela, o Enter e o Esc.

use std::cell::Cell;
use std::rc::Rc;

use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString};

use crate::ApprovalUi;
use crate::headless::{self, Screen};

/// Mais paradas do que a janela Exportar e o resto da tela têm juntos.
const STOPS: usize = 40;
const PRINT: i32 = 0;
const APPROVAL: i32 = 1;

fn press(screen: &Screen, text: impl Into<SharedString>) {
    let text = text.into();
    screen
        .window
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    screen
        .window
        .dispatch_event(WindowEvent::KeyReleased { text });
}

/// A janela aberta na aba `tab`, contando as vezes que pedem para gerar.
fn open(tab: i32, ready: bool) -> (Screen, Rc<Cell<u32>>) {
    let screen = headless::open(1400, 900).unwrap();
    screen.app.set_has_project(true);
    let ui = screen.app.global::<ApprovalUi>();
    ui.set_html(true);
    ui.set_jpg(true);
    ui.set_all_spreads(true);
    ui.set_number(true);
    ui.set_ready(ready);
    ui.set_step(0);
    let asked = Rc::new(Cell::new(0));
    let count = asked.clone();
    ui.on_generate(move || count.set(count.get() + 1));
    screen.app.set_ex_tab(tab);
    screen.app.set_ex_step(0);
    screen.app.set_show_export(true);
    screen.draw();
    // O foco entra um instante depois de abrir (`ExportTab`).
    std::thread::sleep(std::time::Duration::from_millis(20));
    screen.draw();
    (screen, asked)
}

/// Antes das abas, o Exportar abria sem foco nenhum: Esc e Enter só respondiam depois do primeiro Tab.
#[test]
fn dd_b3_esc_fecha_o_exportar_logo_ao_abrir_nas_duas_abas() {
    for tab in [PRINT, APPROVAL] {
        let (screen, _) = open(tab, true);
        press(&screen, Key::Escape);
        screen.draw();
        assert!(!screen.app.get_show_export(), "aba {tab}");
    }
}

#[test]
fn dd_b4_o_primeiro_tab_da_aba_leva_ao_primeiro_campo() {
    let (screen, _) = open(APPROVAL, true);
    press(&screen, Key::Tab);
    screen.draw();
    press(&screen, " ");
    screen.draw();
    assert!(!screen.app.global::<ApprovalUi>().get_html());
}

#[test]
fn espaco_na_aba_troca_de_aba() {
    let (screen, _) = open(PRINT, true);
    press(&screen, Key::Tab);
    screen.draw();
    press(&screen, " ");
    screen.draw();
    assert_eq!(screen.app.get_ex_tab(), APPROVAL);
}

#[test]
fn dd_b10_enter_num_campo_aciona_o_principal_so_quando_ele_esta_ativo() {
    for (ready, expected) in [(true, 1), (false, 0)] {
        let (screen, asked) = open(APPROVAL, ready);
        press(&screen, Key::Tab);
        screen.draw();
        press(&screen, Key::Return);
        assert_eq!(asked.get(), expected, "pronto: {ready}");
        assert!(screen.app.get_show_export());
    }
}

#[test]
fn dd_b2_o_tab_nao_sai_do_exportar() {
    let (screen, _) = open(APPROVAL, true);
    let behind = Rc::new(Cell::new(0));
    let count = behind.clone();
    screen.app.on_command(move |_| count.set(count.get() + 1));
    for _ in 0..STOPS {
        press(&screen, Key::Tab);
        screen.draw();
        assert!(screen.app.get_show_export());
    }
    assert_eq!(behind.get(), 0, "o Tab alcançou um botão atrás da janela");
}
