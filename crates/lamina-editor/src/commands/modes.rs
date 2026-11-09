//! Atalhos por modo (spec EG, regra 2): cada comando diz em que modo da lâmina vale, e nenhum
//! atalho se repete dentro do mesmo modo. No modo organizar as setas movem a seleção e + e − dão
//! zoom na lâmina; fora dele, ↑ e ↓ trocam o layout, ← e → mudam de lâmina (alternativos do Page
//! Down e do Page Up, `NORMAL_ALIASES`) e + e − são o zoom do recorte.

use super::Command;
use crate::keys::{Key, Shortcut};

/// O modo da lâmina grande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Organize,
}

impl Command {
    /// O comando vale no modo `mode`?
    pub fn in_mode(self, mode: Mode) -> bool {
        use Command as C;
        match self {
            // Só fora do modo: ↑ e ↓ trocam o layout; Enter, F, X e R; o zoom do recorte e o 0 que
            // encaixa a foto (no modo, o 0 é Ajustar; spec ZR); L, a
            // faixa de layouts; Ctrl+A, as fotos mostradas (no modo, o Ctrl+A é dos quadros).
            C::NextLayout
            | C::PreviousLayout
            | C::Favorite
            | C::Reject
            | C::ToggleReordered
            | C::BuildFromSelection
            | C::NewSpreadFromSelection
            | C::ZoomIn
            | C::ZoomOut
            | C::ResetCrop
            | C::ToggleLayoutBand
            | C::SelectAllPhotos => mode == Mode::Normal,
            // Só no modo: mover a seleção, selecionar todos, o zoom da lâmina; Z, a lupa; 1, o
            // tamanho real (spec ZR). Alinhar, Igualar e Distribuir passam a valer também na
            // edição comum com a seleção de quadros (spec GJ, conferência do dono 2026-11-04).
            C::NudgeLeft
            | C::NudgeRight
            | C::NudgeUp
            | C::NudgeDown
            | C::NudgeLeftMore
            | C::NudgeRightMore
            | C::NudgeUpMore
            | C::NudgeDownMore
            | C::SelectAllFrames
            | C::ViewZoomIn
            | C::ViewZoomOut
            | C::ViewFit
            | C::Loupe
            | C::ActualSize => mode == Mode::Organize,
            _ => true,
        }
    }

    /// O comando do atalho no modo `mode`. Os alternativos (Ctrl+Shift+Z refaz, Backspace tira a
    /// foto, = vale o +, F1 abre os atalhos) vêm da tabela `ALIASES` (spec AT, regra 3), e os que
    /// só valem fora do modo organizar (← e → mudam de lâmina) de `NORMAL_ALIASES`.
    pub fn from_shortcut_in(shortcut: Shortcut, mode: Mode) -> Option<Command> {
        // Sinais e números: o Shift depende do teclado (o + pede Shift no ABNT2), então não conta.
        let shortcut = match shortcut.key {
            Key::Char(c) if !c.is_alphabetic() => Shortcut {
                shift: false,
                ..shortcut
            },
            _ => shortcut,
        };
        let shortcut = crate::shortcuts::main_of(shortcut, mode);
        Command::ALL
            .into_iter()
            .find(|c| c.in_mode(mode) && c.shortcut() == Some(shortcut))
    }

    /// O comando do atalho fora do modo organizar.
    pub fn from_shortcut(shortcut: Shortcut) -> Option<Command> {
        Command::from_shortcut_in(shortcut, Mode::Normal)
    }
}
