//! A barra da lâmina (spec RL, regras 15 a 17; R0-26): o selo e os atalhos que aparecem ao
//! passar o mouse sobre uma lâmina do rolo. Os botões chamam os comandos do registro, com a
//! disponibilidade do menu.

use crate::{Command, Editor};

use super::roll_timeline::spread_badge;

/// Os botões da barra, na ordem da tela: o lápis, Layouts, Espelhar e Texto.
pub const SPREAD_BAR_COMMANDS: [Command; 4] = [
    Command::Organize,
    Command::ToggleLayoutBand,
    Command::Mirror,
    Command::NewText,
];

/// Um botão da barra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarButton {
    pub command: Command,
    /// A dica: o nome do comando e o atalho ("Espelhar lâmina (E)").
    pub tip: String,
    /// A tecla do atalho ("E").
    pub key: String,
    pub enabled: bool,
}

/// A barra da lâmina `index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadBar {
    pub badge: String,
    pub buttons: Vec<BarButton>,
}

/// O rolo aparece: há álbum com lâminas e não está no modo organizar nem no recorte (regra 24).
pub fn roll_shown(editor: &Editor) -> bool {
    let has_spreads = editor.session().is_some_and(|s| s.spread_count() > 0);
    has_spreads && !editor.organizing() && editor.crop_mode().is_none() && !editor.on_cover()
}

/// O atalho para a capa ao lado da lâmina 1, no rolo (spec CA, seção 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollPill {
    None,
    GoToCover,
    CreateCover,
}

/// A barra da lâmina `index`, ou nada quando o rolo não aparece ou a lâmina não existe (regra
/// 15). A disponibilidade é a do menu na lâmina atual: numa vizinha, ela vira a atual antes.
pub fn spread_bar(editor: &Editor, index: usize) -> Option<SpreadBar> {
    let session = editor.session()?;
    let count = session.spread_count();
    if !roll_shown(editor) || index >= count {
        return None;
    }
    let buttons = SPREAD_BAR_COMMANDS
        .into_iter()
        .map(|command| {
            let key = command.shortcut().map(|s| s.text()).unwrap_or_default();
            BarButton {
                command,
                tip: format!("{} ({key})", command.label()),
                key,
                enabled: editor.available(command),
            }
        })
        .collect();
    let edges = session.album().spec.edges();
    Some(SpreadBar {
        badge: spread_badge(edges, index, count),
        buttons,
    })
}
