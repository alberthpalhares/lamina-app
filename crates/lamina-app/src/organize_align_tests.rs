//! O painel de alinhar do modo organizar (correções da conferência de 2026-11-09): o seletor
//! "área da lâmina" mora na própria fileira, à direita, abaixo da linha do Distribuir, e
//! marcado ele troca os comandos — "à lâmina" chega ao Rust, não o entre-selecionados. Janela
//! de verdade sem tela, como `organize_tests.rs`. Posições medidas no desenho 1366×920 (tema
//! escuro, painel direito a partir de x 1061): os 6 ícones de alinhar em y≈538, primeiro
//! centrado em x≈1093; o seletor em y≈625, centrado em x≈1338.

use std::cell::RefCell;
use std::rc::Rc;

use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition};

use crate::AppWindow;
use crate::headless::{self, Screen};

const TALL: (u32, u32) = (1366, 920);
/// O centro vertical da fileira dos ícones de alinhar (medido).
const ALIGN_ROW_Y: usize = 538;
/// O centro vertical da fileira própria do seletor (medido).
const SELECTOR_ROW_Y: usize = 625;
/// O centro horizontal do seletor (medido): a célula encostada à direita do painel.
const SELECTOR_X: f32 = 1338.0;
/// O centro horizontal da primeira célula de alinhar (medido).
const ALIGN_FIRST_X: f32 = 1093.0;

/// A janela com o modo organizar ligado, gravando os comandos que chegam.
fn align_window() -> (Screen, Rc<RefCell<Vec<String>>>) {
    let screen = headless::open(TALL.0, TALL.1).unwrap();
    crate::news::connect(&screen.app);
    screen.app.set_has_project(true);
    let ui = screen.app.global::<crate::OrganizeUi>();
    ui.set_on(true);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    screen.app.on_command(move |name| {
        sink.borrow_mut().push(name.as_str().to_owned());
    });
    (screen, seen)
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

/// As colunas com pixels claros (o ícone) numa linha do desenho. O ícone de scan tem linhas
/// vazias no meio, então varre uma faixa de ±6 linhas e junta.
fn bright_columns(screen: &Screen, y: usize) -> Vec<usize> {
    let pixels = screen.draw();
    let width = screen.width() as usize;
    let height = screen.height() as usize;
    let mut cols = Vec::new();
    for dy in y.saturating_sub(6)..(y + 6).min(height) {
        for x in 0..width {
            let p = pixels[dy * width + x];
            if p.r > 150 && p.g > 150 && !cols.contains(&x) {
                cols.push(x);
            }
        }
    }
    cols
}

/// Marcado o seletor, os botões de alinhar mandam os comandos "à lâmina", que existem no
/// registro — não ficam mudos (bug da conferência de 2026-11-09).
#[test]
fn o_alinhar_marcado_manda_o_comando_a_lamina() {
    let (screen, seen) = align_window();
    screen.draw();
    screen.draw();
    // Marca o seletor "área da lâmina" na fileira própria, à direita.
    click(&screen, SELECTOR_X, SELECTOR_ROW_Y as f32);
    seen.borrow_mut().clear();
    // O primeiro botão de alinhar.
    click(&screen, ALIGN_FIRST_X, ALIGN_ROW_Y as f32);
    let got = seen.borrow().clone();
    assert_eq!(
        got,
        vec!["organizar.alinhar-lamina-esquerda".to_owned()],
        "com o seletor marcado, o comando é o à lâmina (chegou {got:?})"
    );
}

/// Desmarcado, o mesmo botão manda o comando entre os selecionados.
#[test]
fn o_alinhar_desmarcado_manda_o_comando_entre_selecionados() {
    let (screen, seen) = align_window();
    screen.draw();
    screen.draw();
    seen.borrow_mut().clear();
    click(&screen, ALIGN_FIRST_X, ALIGN_ROW_Y as f32);
    let got = seen.borrow().clone();
    assert_eq!(
        got,
        vec!["organizar.alinhar-esquerda".to_owned()],
        "sem o seletor, o comando é o entre os selecionados (chegou {got:?})"
    );
}

/// O seletor mora na própria fileira, encostado à direita do painel (pedido do dono
/// 2026-11-09): o ícone dele fica no fim da janela, longe dos ícones de distribuir (que
/// ficam na esquerda) e abaixo da linha deles.
#[test]
fn o_seletor_fica_a_direita_do_painel() {
    let (screen, _seen) = align_window();
    screen.draw();
    screen.draw();
    if let Ok(folder) = std::env::var("LAMINA_CAPTURAS") {
        let pixels = screen.draw();
        let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
        let out = image::RgbImage::from_raw(screen.width(), screen.height(), bytes).unwrap();
        out.save(format!("{folder}/seletor.png")).unwrap();
    }
    let cols = bright_columns(&screen, SELECTOR_ROW_Y);
    let first = *cols.first().expect("a fileira do seletor tem ícone");
    let last = *cols.last().expect("a fileira do seletor tem ícone");
    let width = screen.width() as usize;
    // Encostado à direita: o último pixel fica a menos de 60 px da borda.
    assert!(
        width - last < 60,
        "o seletor encosta na direita (último pixel em {last}, janela {width})"
    );
    // Abaixo da linha de distribuir e longe dela: o seletor não divide a fileira com os
    // ícones de distribuir, que ficam à esquerda (x < 1140).
    assert!(
        first > 1140,
        "o seletor não fica junto dos ícones de distribuir (primeiro pixel em {first})"
    );
    // A fileira de alinhar (acima) não passa de x 1280: o seletor começa depois dela.
    let align_cols = bright_columns(&screen, ALIGN_ROW_Y);
    let align_last = *align_cols.last().expect("a fileira de alinhar tem ícones");
    assert!(
        first > align_last,
        "o seletor fica abaixo e além do último botão de alinhar (seletor em {first}, alinhar termina em {align_last})"
    );
}

// `AppWindow` importado pelo `ComponentHandle` usado no `window.window()` do diagnóstico.
#[allow(dead_code)]
fn _keep(window: &AppWindow) {
    let _ = window.window();
}
