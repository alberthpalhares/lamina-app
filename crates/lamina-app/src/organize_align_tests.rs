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
/// O centro vertical da linha dos de distribuir + seletor (medido).
const SELECTOR_ROW_Y: usize = 591;
/// O centro horizontal do seletor: a terceira célula da linha do distribuir (medido).
const SELECTOR_X: f32 = 1148.0;
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

/// As colunas com pixels claros (o ícone) numa faixa de linhas do desenho dado. O ícone de scan
/// tem linhas vazias no meio, então varre ±9 linhas e junta. Usa os pixels já desenhados: um
/// `draw` novo no meio do teste pode pegar outro quadro da animação de fundo.
fn columns_in(pixels: &[slint::Rgb8Pixel], width: usize, height: usize, y: usize) -> Vec<usize> {
    let mut cols = Vec::new();
    for dy in y.saturating_sub(9)..(y + 9).min(height) {
        for x in 0..width {
            let p = pixels[dy * width + x];
            if p.r > 150 && p.g > 150 && !cols.contains(&x) {
                cols.push(x);
            }
        }
    }
    // A ordem do vetor é a de descoberta (linha a linha), não a de x: ordena.
    cols.sort_unstable();
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

/// O seletor mora na mesma linha dos ícones de distribuir, como terceira célula — abaixo do
/// "Alinhar à base" (pedido do dono reafirmado na conferência de 2026-11-10): o ícone dele
/// fica na mesma faixa vertical dos de distribuir, à direita deles.
#[test]
fn o_seletor_fica_na_linha_do_distribuir() {
    let (screen, _seen) = align_window();
    screen.draw();
    screen.draw();
    let pixels = screen.draw();
    let (w, h) = (screen.width() as usize, screen.height() as usize);
    if let Ok(folder) = std::env::var("LAMINA_CAPTURAS") {
        let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
        let out = image::RgbImage::from_raw(screen.width(), screen.height(), bytes).unwrap();
        out.save(format!("{folder}/seletor.png")).unwrap();
    }
    let cols = columns_in(&pixels, w, h, SELECTOR_ROW_Y);
    if std::env::var("LAMINA_DEBUG").is_ok() {
        println!(
            "cols em {SELECTOR_ROW_Y}: primeiro {:?}, último {:?}",
            cols.first(),
            cols.last()
        );
        for dy in [585usize, 595] {
            let bright: Vec<usize> = (1140..w)
                .filter(|x| pixels[dy * w + x].r > 150 && pixels[dy * w + x].g > 150)
                .collect();
            println!("linha {dy}: claros à direita {bright:?}");
        }
    }
    let first = *cols.first().expect("a linha do distribuir tem ícones");
    let last = *cols.last().expect("a linha do distribuir tem ícones");
    // Os dois de distribuir ficam entre 1079 e 1140 (medido); o seletor, depois deles.
    assert!(
        first < 1140,
        "os de distribuir ficam à esquerda (primeiro pixel em {first})"
    );
    assert!(
        last > 1140,
        "o seletor fica depois dos de distribuir (último pixel em {last})"
    );
    // Três células na mesma faixa vertical: a distância total da linha cabe em ~110 px
    // (3 células de 30 px + 2 vãos de 4 px), a partir do primeiro ícone.
    assert!(
        last - first < 115,
        "três células na linha (de {first} a {last})"
    );
    // E abaixo da fileira de alinhar: o primeiro da linha de distribuir vem depois do fim da
    // fileira de alinhar em y (mesma coluna, outra faixa).
    let align_cols = columns_in(&pixels, w, h, ALIGN_ROW_Y);
    let align_last_x = *align_cols.last().expect("a fileira de alinhar tem ícones");
    assert!(
        align_last_x > 1250,
        "a fileira de alinhar vai até o sexto botão ({align_last_x})"
    );
}

// `AppWindow` importado pelo `ComponentHandle` usado no `window.window()` do diagnóstico.
#[allow(dead_code)]
fn _keep(window: &AppWindow) {
    let _ = window.window();
}
