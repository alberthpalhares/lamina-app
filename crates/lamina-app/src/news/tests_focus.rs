//! PV-B20 com a janela de verdade, sem tela: o foco fica na página de novidades. Antes da correção, o
//! Tab saía dela e alcançava os botões da janela, escondidos atrás; ali, Enter rodava o comando deles
//! e Esc não fechava a página.

use std::cell::RefCell;
use std::rc::Rc;

use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, SharedString};

use super::tests::{WIDE, notes_with, open_notes, open_window};
use crate::NewsUi;
use crate::headless::Screen;

/// O único comando que os botões da própria página pedem à janela.
const FROM_THE_PAGE: &str = "ajuda.enviar-sugestao";
/// Mais paradas do que a página e a janela têm juntas: duas voltas inteiras, se ficar presa.
const STOPS: usize = 30;

fn press(screen: &Screen, text: impl Into<SharedString>) {
    let text = text.into();
    screen
        .window
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    screen
        .window
        .dispatch_event(WindowEvent::KeyReleased { text });
}

/// A janela com um álbum aberto, para os botões de trás existirem, anotando os comandos que chegam.
fn window_with_buttons_behind() -> (Screen, Rc<RefCell<Vec<String>>>) {
    let screen = open_window(WIDE);
    screen.app.set_has_project(true);
    let commands = Rc::new(RefCell::new(Vec::new()));
    let seen = commands.clone();
    screen
        .app
        .on_command(move |name| seen.borrow_mut().push(name.as_str().to_owned()));
    (screen, commands)
}

fn open(screen: &Screen) {
    open_notes(screen, &notes_with(2), true);
    screen.draw();
}

fn is_open(screen: &Screen) -> bool {
    screen.app.global::<NewsUi>().get_show()
}

/// Em cada parada: `Enter` não roda nada atrás da página, e `Esc` sempre fecha.
fn walk(step: Key) {
    let (screen, commands) = window_with_buttons_behind();
    for stops in 0..STOPS {
        open(&screen);
        for _ in 0..stops {
            press(&screen, step);
        }
        press(&screen, Key::Return);
        screen.draw();
        if is_open(&screen) {
            press(&screen, Key::Escape);
            screen.draw();
            assert!(!is_open(&screen), "Esc não fechou depois de {stops} passos");
        }
    }
    let behind: Vec<String> = commands
        .borrow()
        .iter()
        .filter(|name| *name != FROM_THE_PAGE)
        .cloned()
        .collect();
    assert_eq!(behind, Vec::<String>::new(), "rodou atrás da página");
}

#[test]
fn pv_b20_tab_da_a_volta_na_pagina_e_nada_atras_reage() {
    walk(Key::Tab);
}

#[test]
fn pv_b20_shift_tab_da_a_volta_na_pagina_e_nada_atras_reage() {
    walk(Key::Backtab);
}

#[test]
fn pv_b20_esc_fecha_mesmo_se_a_janela_pedir_o_foco_de_volta() {
    let (screen, _) = window_with_buttons_behind();
    open(&screen);
    // Um trabalho que termina com a página aberta pede o foco para a janela.
    screen.app.invoke_focus_main();
    screen.draw();
    press(&screen, Key::Escape);
    screen.draw();
    assert!(!is_open(&screen));
}

/// "Experimente agora" fecha a página e abre um diálogo na mesma volta. O popup que fecha devolve o
/// foco a quem o tinha, e a janela não pode ficar com ele por baixo do diálogo novo.
#[test]
fn pv_b20_o_dialogo_aberto_ao_fechar_a_pagina_fica_com_o_foco() {
    let (screen, _) = window_with_buttons_behind();
    let answers = Rc::new(RefCell::new(Vec::new()));
    let seen = answers.clone();
    screen
        .app
        .on_confirm(move |choice| seen.borrow_mut().push(choice));
    open(&screen);
    // A pergunta vem antes da página na árvore: o caso em que a ordem atrapalha.
    screen.app.global::<NewsUi>().set_show(false);
    screen.app.set_show_confirm(true);
    screen.draw();
    press(&screen, Key::Escape);
    screen.draw();
    assert_eq!(*answers.borrow(), vec![2], "o Esc chega na pergunta");
}

#[test]
fn pv_b20_esc_fecha_depois_de_rolar_com_o_foco_numa_linha_que_saiu_de_vista() {
    let (screen, _) = window_with_buttons_behind();
    open_notes(&screen, &notes_with(30), true);
    screen.draw();
    // O foco desce até uma linha da lista, e a lista rola até ela sair da tela.
    for _ in 0..6 {
        press(&screen, Key::Tab);
    }
    let position = LogicalPosition::new(600.0, 400.0);
    for _ in 0..40 {
        screen.window.dispatch_event(WindowEvent::PointerScrolled {
            position,
            delta_x: 0.0,
            delta_y: -200.0,
        });
        screen.draw();
    }
    press(&screen, Key::Escape);
    screen.draw();
    assert!(!is_open(&screen));
}
