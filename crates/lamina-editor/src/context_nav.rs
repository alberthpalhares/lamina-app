//! O menu de contexto pelo teclado e no lugar (spec DC): por onde as setas andam, o que cada tecla
//! faz, onde o menu e o submenu ficam dentro da janela e as linhas do menu da foto. A janela só
//! desenha e repassa as teclas.

use crate::leaving::remove_label;
use crate::reveal::System;

/// A altura de uma linha e de um separador, e a folga do painel (spec DC, regra 3).
pub const ROW_HEIGHT: f32 = 28.0;
pub const SEPARATOR_HEIGHT: f32 = 8.0;
pub const PANEL_PADDING: f32 = 4.0;
/// A folga até a borda da janela, e quanto o submenu encosta no menu.
const MARGIN: f32 = 4.0;
const OVERLAP: f32 = 4.0;

/// Uma linha, para as setas: um item que vale, um submenu, um item apagado ou um separador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLine {
    Item,
    Sub,
    Disabled,
    Separator,
}

impl MenuLine {
    fn walkable(self) -> bool {
        matches!(self, MenuLine::Item | MenuLine::Sub)
    }
}

/// A linha seguinte ou anterior que vale, dando a volta nas pontas. Sem nenhuma, fica sem atual.
pub fn next_line(lines: &[MenuLine], current: Option<usize>, forward: bool) -> Option<usize> {
    let n = lines.len();
    let order: Vec<usize> = match (current, forward) {
        (None, true) => (0..n).collect(),
        (None, false) => (0..n).rev().collect(),
        (Some(c), true) => (1..=n).map(|k| (c + k) % n).collect(),
        (Some(c), false) => (1..=n).map(|k| (c + n - k % n) % n).collect(),
    };
    order.into_iter().find(|i| lines[*i].walkable())
}

/// As teclas que o menu usa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Up,
    Down,
    Right,
    Left,
    Enter,
    Esc,
}

/// O que uma tecla pede à janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuStep {
    /// Só o atual mudou, ou nada.
    Stay,
    /// Abre o submenu da linha.
    OpenSub(usize),
    CloseSub,
    /// Roda a linha do menu.
    Run(usize),
    /// Roda a linha do submenu aberto.
    RunSub(usize),
    /// Fecha o menu; a seleção e o modo ficam como estavam.
    Close,
}

/// Onde o teclado está: o atual do menu e, com um submenu aberto, o dele.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuCursor {
    pub main: Option<usize>,
    pub open: Option<usize>,
    pub sub: Option<usize>,
}

impl MenuCursor {
    /// Ao abrir pelo teclado, o primeiro item que vale fica marcado (spec DC, regra 6).
    pub fn by_keyboard(main: &[MenuLine]) -> Self {
        MenuCursor {
            main: next_line(main, None, true),
            ..MenuCursor::default()
        }
    }

    /// Aplica uma tecla, com as linhas do menu e as do submenu aberto.
    pub fn press(&mut self, key: MenuKey, main: &[MenuLine], sub: &[MenuLine]) -> MenuStep {
        if self.open.is_some() {
            return self.press_in_sub(key, sub);
        }
        match key {
            MenuKey::Up => self.main = next_line(main, self.main, false),
            MenuKey::Down => self.main = next_line(main, self.main, true),
            MenuKey::Esc => return MenuStep::Close,
            MenuKey::Left => {}
            MenuKey::Right | MenuKey::Enter => {
                return self.enter(key, main);
            }
        }
        MenuStep::Stay
    }

    fn enter(&mut self, key: MenuKey, main: &[MenuLine]) -> MenuStep {
        let Some(i) = self.main else {
            return MenuStep::Stay;
        };
        match main.get(i) {
            Some(MenuLine::Sub) => MenuStep::OpenSub(i),
            Some(MenuLine::Item) if key == MenuKey::Enter => MenuStep::Run(i),
            _ => MenuStep::Stay,
        }
    }

    fn press_in_sub(&mut self, key: MenuKey, sub: &[MenuLine]) -> MenuStep {
        match key {
            MenuKey::Up => self.sub = next_line(sub, self.sub, false),
            MenuKey::Down => self.sub = next_line(sub, self.sub, true),
            MenuKey::Left | MenuKey::Esc => return MenuStep::CloseSub,
            MenuKey::Right => {}
            MenuKey::Enter => {
                if let Some(i) = self.sub {
                    return MenuStep::RunSub(i);
                }
            }
        }
        MenuStep::Stay
    }
}

/// Um painel do menu na janela: o canto e a altura; passando da janela, ele rola.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuBox {
    pub x: f32,
    pub y: f32,
    pub height: f32,
}

fn fit(start: f32, size: f32, room: f32) -> f32 {
    start.min(room - size - MARGIN).max(MARGIN)
}

/// Onde o menu fica: no ponto pedido e, se não couber, encostado nas bordas; mais alto que a janela,
/// ele fica com a altura dela menos as folgas (spec DC, regra 9).
pub fn menu_box(at: (f32, f32), size: (f32, f32), window: (f32, f32)) -> MenuBox {
    let height = size.1.min(window.1 - 2.0 * MARGIN);
    MenuBox {
        x: fit(at.0, size.0, window.0),
        y: fit(at.1, height, window.1),
        height,
    }
}

/// Onde o submenu fica: à direita do menu, na altura da linha, ou à esquerda quando não cabe (regra 8).
pub fn submenu_box(main: (f32, f32), row_y: f32, size: (f32, f32), window: (f32, f32)) -> MenuBox {
    let right = main.0 + main.1 - OVERLAP;
    let x = if right + size.0 + MARGIN <= window.0 {
        right
    } else {
        main.0 + OVERLAP - size.0
    };
    let placed = menu_box((x, row_y), size, window);
    MenuBox { x, ..placed }
}

/// As linhas do menu até `row`, em px, a partir do alto do painel.
pub fn row_top(lines: &[MenuLine], row: usize) -> f32 {
    lines.iter().take(row).fold(PANEL_PADDING, |y, line| {
        y + match line {
            MenuLine::Separator => SEPARATOR_HEIGHT,
            _ => ROW_HEIGHT,
        }
    })
}

/// O que a tecla Menu (ou Shift+F10) abre (spec DC, seção 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMenu {
    Frame(usize),
    Background,
    Photo,
}

/// Com a grade em foco e foto selecionada, o menu da foto; senão, o do quadro selecionado; sem
/// quadro, o do fundo. Sem álbum aberto, nada.
pub fn key_menu(has_album: bool, photo_in_focus: bool, frame: Option<usize>) -> Option<KeyMenu> {
    if !has_album {
        return None;
    }
    Some(match (photo_in_focus, frame) {
        (true, _) => KeyMenu::Photo,
        (false, Some(f)) => KeyMenu::Frame(f),
        (false, None) => KeyMenu::Background,
    })
}

/// As duas linhas do menu da foto (spec TF, regra 1): dependem de quantas estão selecionadas e do
/// sistema.
pub fn photo_menu_lines(selected: usize, system: System) -> [String; 2] {
    [
        remove_label(selected.max(1)),
        system.reveal_label().to_owned(),
    ]
}
