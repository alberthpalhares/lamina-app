//! A lista de atalhos (spec AT): Ajuda → "Atalhos do teclado…". Ela sai da mesma tabela que o
//! teclado obedece (`Command::ALL` e [`ALIASES`]); os gestos do mouse e as teclas fora da tabela
//! ficam nas linhas fixas, em `shortcuts/fixed.rs`, e a busca em `shortcuts/search.rs`.

mod fixed;
mod search;

pub use search::{NO_MATCH_HINT, fold, no_match, search};

use crate::Command;
use crate::commands::{Menu, Mode};
use crate::keys::{Key, Platform, Shortcut};

/// Os atalhos alternativos (regra 3): cada linha leva um alternativo ao atalho principal. O
/// teclado (`Command::from_shortcut_in`) e a lista leem esta tabela.
pub const ALIASES: [(Shortcut, Shortcut); 4] = [
    (Shortcut::ctrl_shift('z'), Shortcut::ctrl('y')),
    (Shortcut::key(Key::Backspace), Shortcut::key(Key::Delete)),
    (Shortcut::key(Key::Char('=')), Shortcut::key(Key::Char('+'))),
    (Shortcut::key(Key::F1), Shortcut::ctrl('/')),
];

/// Os alternativos que só valem fora do modo organizar, onde as setas movem a seleção: ← e →
/// mudam de lâmina, como o Page Up e o Page Down (pedido do dono de 2026-10-28).
pub const NORMAL_ALIASES: [(Shortcut, Shortcut); 2] = [
    (Shortcut::key(Key::Right), Shortcut::key(Key::PageDown)),
    (Shortcut::key(Key::Left), Shortcut::key(Key::PageUp)),
];

/// O atalho principal de um alternativo no modo `mode`; os outros ficam como estão.
pub fn main_of(shortcut: Shortcut, mode: Mode) -> Shortcut {
    let normal: &[(Shortcut, Shortcut)] = if mode == Mode::Normal {
        &NORMAL_ALIASES
    } else {
        &[]
    };
    ALIASES
        .iter()
        .chain(normal)
        .find(|(alias, _)| *alias == shortcut)
        .map_or(shortcut, |(_, main)| *main)
}

/// Os grupos, sempre nesta ordem (seção 4).
pub const DRAG_AND_CLICK: &str = "Arrastar e clicar";
pub const ORGANIZE: &str = "No modo organizar";
pub const CROP: &str = "Recorte da foto";
pub const GROUPS: [&str; 9] = [
    DRAG_AND_CLICK,
    "Arquivo",
    "Editar",
    "Lâmina",
    "Layout",
    "Visualizar",
    "Ajuda",
    ORGANIZE,
    CROP,
];

/// Uma linha da lista: o que faz e as teclas (o principal e os alternativos) ou, nas linhas fixas,
/// o gesto. `command` fica vazio nas linhas fixas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub does: &'static str,
    pub keys: Vec<String>,
    pub command: Option<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub title: &'static str,
    pub rows: Vec<Row>,
}

/// A lista inteira, com as teclas escritas como no sistema `platform` (regras 2 e 5).
pub fn sheet(platform: Platform) -> Vec<Group> {
    GROUPS
        .iter()
        .map(|&title| {
            let commands = Command::ALL
                .into_iter()
                .filter(|c| c.shortcut().is_some() && group_of(*c) == title)
                .map(|c| command_row(c, platform));
            let fixed = fixed::rows(title, platform);
            Group {
                title,
                rows: commands.chain(fixed).collect(),
            }
        })
        .collect()
}

/// O grupo de um comando (regra 2): o recorte, depois o modo organizar, depois o menu.
pub fn group_of(command: Command) -> &'static str {
    if command.name().starts_with("recorte.") {
        return CROP;
    }
    if !command.in_mode(Mode::Normal) {
        return ORGANIZE;
    }
    match command.menu() {
        Menu::File => "Arquivo",
        Menu::Edit => "Editar",
        Menu::Spread => "Lâmina",
        Menu::Cover => "Capa",
        Menu::Layout => "Layout",
        Menu::View => "Visualizar",
        Menu::Help => "Ajuda",
    }
}

/// A linha de um comando: o rótulo da tabela, o atalho e depois os alternativos dele.
fn command_row(command: Command, platform: Platform) -> Row {
    let main = command.shortcut();
    let aliases = ALIASES
        .iter()
        .chain(&NORMAL_ALIASES)
        .filter(|(_, m)| Some(*m) == main)
        .map(|(alias, _)| *alias);
    Row {
        does: command.label(),
        keys: main
            .into_iter()
            .chain(aliases)
            .map(|s| s.text_on(platform))
            .collect(),
        command: Some(command),
    }
}
