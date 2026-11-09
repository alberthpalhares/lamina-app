//! O popup dos menus Alinhar/Igualar/Distribuir da barra do organizar (bug da beta 10,
//! 2026-11-09): abria no canto esquerdo da janela — o painel seguia o popup gigante, não o botão.
//! Aqui a janela de verdade, sem tela, com a barra do modo e o álbum aberto.

use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Rgb8Pixel};

use crate::convert::model;
use crate::headless::{self, Screen};
use crate::{BarRow, OrganizeUi, PaletteUi};

const WIDE: (u32, u32) = (1366, 668);

/// A janela com o modo organizar ligado e o menu Alinhar apto: sem editor, só o desenho.
fn organize_window() -> Screen {
    let screen = headless::open(WIDE.0, WIDE.1).unwrap();
    crate::news::connect(&screen.app);
    screen.app.set_has_project(true);
    let ui = screen.app.global::<OrganizeUi>();
    ui.set_on(true);
    let palette = screen.app.global::<PaletteUi>();
    palette.set_can_align(true);
    let rows = vec![
        BarRow {
            icon: slint::Image::default(),
            label: "Alinhar à esquerda".into(),
            command: "organizar.alinhar-esquerda".into(),
            enabled: true,
        },
        BarRow {
            icon: slint::Image::default(),
            label: "Alinhar ao topo".into(),
            command: "organizar.alinhar-topo".into(),
            enabled: true,
        },
    ];
    ui.set_align_rows(model(rows));
    screen
}

fn click(screen: &Screen, x: f32, y: f32) {
    let position = LogicalPosition::new(x, y);
    let button = PointerEventButton::Left;
    screen
        .window
        .dispatch_event(WindowEvent::PointerMoved { position });
    screen
        .window
        .dispatch_event(WindowEvent::PointerPressed { position, button });
    screen
        .window
        .dispatch_event(WindowEvent::PointerReleased { position, button });
    screen.draw();
}

/// As colunas onde o desenho mudou em relação a `before`, na faixa de linhas dada.
fn changed_columns(screen: &Screen, before: &[Rgb8Pixel], after: &[Rgb8Pixel]) -> Vec<usize> {
    let width = screen.width() as usize;
    (0..width)
        .filter(|x| {
            (40..460usize).any(|y| {
                let a = before[y * width + x];
                let b = after[y * width + x];
                (a.r, a.g, a.b) != (b.r, b.g, b.b)
            })
        })
        .collect()
}

/// O botão Alinhar abre o painel **junto do botão** (colunas perto da dele, x 360 no desenho
/// 1366×668), não no canto esquerdo da janela (bug da beta 10: o painel ficava em x 0).
#[test]
fn o_menu_alinhar_abre_junto_do_botao() {
    let screen = organize_window();
    screen.draw();
    screen.draw();
    let before = screen.draw();
    // O centro do rótulo "Alinhar" na barra (medido no desenho 1366×668).
    let button_x = 360.0;
    click(&screen, button_x, 20.0);
    let after = screen.draw();
    if let Ok(folder) = std::env::var("LAMINA_CAPTURAS") {
        for (name, pixels) in [("antes", &before), ("depois", &after)] {
            let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            let out = image::RgbImage::from_raw(screen.width(), screen.height(), bytes).unwrap();
            out.save(format!("{folder}/organize-{name}.png")).unwrap();
        }
    }
    let cols = changed_columns(&screen, &before, &after);
    assert!(cols.len() > 50, "o painel do menu desenhou ({cols:?}…)");
    let first = *cols.first().unwrap();
    // Junto do botão: o começo fica até 120 px da coluna do botão, não encostado no canto (0).
    assert!(
        (button_x - first as f32).abs() < 120.0,
        "o painel começa perto do botão ({button_x}), não no canto ({first})"
    );
}
