//! Cenários da spec `design-menu-de-contexto` (DC) que são decisão do editor: o atalho de cada
//! linha, as setas, as teclas, o lugar do menu e do submenu e o menu da foto.

use crate::Command;
use crate::context_nav::{
    KeyMenu, MenuBox, MenuCursor, MenuKey, MenuLine, MenuStep, key_menu, menu_box, next_line,
    photo_menu_lines, row_top, submenu_box,
};
use crate::editor::ContextEntry;
use crate::reveal::System;

use super::eg_imas::album_mm;
use super::tl_common::editor_at;

fn shortcut_of(menu: &[ContextEntry], command: Command) -> Option<String> {
    menu.iter().find_map(|e| match e {
        ContextEntry::Item {
            command: c,
            shortcut,
            ..
        } if *c == command => Some(shortcut.clone()),
        _ => None,
    })
}

fn expected(command: Command) -> String {
    command.shortcut().map(|s| s.text()).unwrap_or_default()
}

fn menus() -> (Vec<ContextEntry>, Vec<ContextEntry>) {
    let project = album_mm(&[(20.0, 40.0, 40.0, 50.0), (80.0, 60.0, 50.0, 70.0)]);
    let mut editor = editor_at(project, 1);
    let background = editor.context_menu(None);
    let frame = editor.context_menu(Some(0));
    (frame, background)
}

#[test]
fn dc_b1_o_atalho_em_cada_linha_vem_do_registro() {
    let (frame, background) = menus();
    for command in [
        Command::CopyFrames,
        Command::PasteFrames,
        Command::DuplicateFrames,
    ] {
        assert_eq!(shortcut_of(&frame, command), Some(expected(command)));
        assert!(!expected(command).is_empty());
    }
    for command in [Command::Organize, Command::ToggleGuides] {
        assert_eq!(shortcut_of(&background, command), Some(expected(command)));
        assert!(!expected(command).is_empty());
    }
    assert_eq!(
        shortcut_of(&frame, Command::SendBackward),
        Some(String::new())
    );
}

#[test]
fn dc_b2_as_setas_andam_so_no_que_vale() {
    use MenuLine::{Disabled, Item, Separator, Sub};
    let lines = [Item, Separator, Disabled, Sub, Disabled];
    assert_eq!(next_line(&lines, None, true), Some(0));
    assert_eq!(next_line(&lines, None, false), Some(3));
    assert_eq!(
        next_line(&lines, Some(0), true),
        Some(3),
        "pula o separador e o apagado"
    );
    assert_eq!(next_line(&lines, Some(3), true), Some(0), "dá a volta");
    assert_eq!(
        next_line(&lines, Some(0), false),
        Some(3),
        "dá a volta para cima"
    );
    assert_eq!(next_line(&[Disabled, Separator], None, true), None);
    assert_eq!(next_line(&[Disabled, Separator], Some(0), true), None);
}

#[test]
fn dc_b3_enter_setas_e_esc() {
    use MenuLine::{Item, Sub};
    let main = [Item, Sub];
    let sub = [Item, Item];
    let mut cursor = MenuCursor::by_keyboard(&main);
    assert_eq!(cursor.main, Some(0));
    // Num comando, Enter roda; → não faz nada.
    assert_eq!(cursor.press(MenuKey::Right, &main, &[]), MenuStep::Stay);
    assert_eq!(cursor.press(MenuKey::Enter, &main, &[]), MenuStep::Run(0));
    // Num submenu, Enter e → abrem.
    assert_eq!(cursor.press(MenuKey::Down, &main, &[]), MenuStep::Stay);
    assert_eq!(
        cursor.press(MenuKey::Enter, &main, &[]),
        MenuStep::OpenSub(1)
    );
    assert_eq!(
        cursor.press(MenuKey::Right, &main, &[]),
        MenuStep::OpenSub(1)
    );
    cursor.open = Some(1);
    cursor.sub = next_line(&sub, None, true);
    // Dentro do submenu: Enter roda a linha dele; ← e Esc fecham só o submenu.
    assert_eq!(cursor.press(MenuKey::Down, &main, &sub), MenuStep::Stay);
    assert_eq!(cursor.sub, Some(1));
    assert_eq!(
        cursor.press(MenuKey::Enter, &main, &sub),
        MenuStep::RunSub(1)
    );
    assert_eq!(cursor.press(MenuKey::Left, &main, &sub), MenuStep::CloseSub);
    assert_eq!(cursor.press(MenuKey::Esc, &main, &sub), MenuStep::CloseSub);
    // Sem submenu aberto, Esc fecha o menu.
    cursor.open = None;
    assert_eq!(cursor.press(MenuKey::Esc, &main, &[]), MenuStep::Close);
}

#[test]
fn dc_b4_tecla_menu_abre_o_que_esta_selecionado() {
    assert_eq!(key_menu(true, false, Some(2)), Some(KeyMenu::Frame(2)));
    assert_eq!(key_menu(true, false, None), Some(KeyMenu::Background));
    assert_eq!(key_menu(true, true, Some(2)), Some(KeyMenu::Photo));
    assert_eq!(key_menu(false, false, None), None, "sem álbum, nada");
    // Pelo teclado, o primeiro item que vale fica marcado.
    let lines = [MenuLine::Disabled, MenuLine::Separator, MenuLine::Item];
    assert_eq!(MenuCursor::by_keyboard(&lines).main, Some(2));
}

#[test]
fn dc_b6_o_menu_da_foto_segue_a_selecao() {
    let one = photo_menu_lines(1, System::Windows);
    assert_eq!(one[0], "Remover fotografia do álbum");
    assert_eq!(one[1], "Mostrar no Explorador");
    let three = photo_menu_lines(3, System::Windows);
    assert_eq!(three[0], "Remover 3 fotografias do álbum");
    // Sem nada na seleção, a foto clicada conta como uma.
    assert_eq!(photo_menu_lines(0, System::Windows)[0], one[0]);
}

#[test]
fn dc_b8_o_menu_fica_dentro_da_janela() {
    let placed = menu_box((1356.0, 658.0), (300.0, 500.0), (1366.0, 668.0));
    assert_eq!(
        placed,
        MenuBox {
            x: 1062.0,
            y: 164.0,
            height: 500.0
        }
    );
    // Na janela mais baixa que o menu: a altura dela menos as folgas, e rola.
    let low = menu_box((790.0, 390.0), (300.0, 500.0), (800.0, 400.0));
    assert_eq!(
        low,
        MenuBox {
            x: 496.0,
            y: 4.0,
            height: 392.0
        }
    );
    // Onde couber, fica no ponto pedido.
    let free = menu_box((100.0, 80.0), (300.0, 200.0), (1366.0, 668.0));
    assert_eq!((free.x, free.y), (100.0, 80.0));
}

#[test]
fn dc_b9_o_submenu_abre_ao_lado_sem_mexer_no_resto() {
    use MenuLine::{Item, Separator, Sub};
    let (frame, _) = menus();
    // As linhas do menu são as mesmas com o submenu fechado ou aberto: ele não entra na lista.
    assert!(frame.iter().any(|e| matches!(
        e,
        ContextEntry::Sub {
            title: "Alinhar",
            ..
        }
    )));
    // A altura da linha conta as de cima.
    assert_eq!(row_top(&[Item, Separator, Sub], 2), 4.0 + 28.0 + 8.0);
    // À direita, encostando; sem espaço, à esquerda.
    let right = submenu_box((100.0, 290.0), 200.0, (150.0, 176.0), (1366.0, 668.0));
    assert_eq!((right.x, right.y), (386.0, 200.0));
    let left = submenu_box((1100.0, 260.0), 200.0, (150.0, 176.0), (1366.0, 668.0));
    assert_eq!((left.x, left.y), (954.0, 200.0));
    // Perto do pé da janela, sobe para caber.
    let low = submenu_box((100.0, 290.0), 600.0, (150.0, 176.0), (1366.0, 668.0));
    assert_eq!(low.y, 488.0);
}
