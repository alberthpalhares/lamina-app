//! A barra de cima enxuta (spec JE, regras 8 a 10): os comandos dos botões, na ordem da tela, e a
//! dica de cada um, com a ação que o clique faz e o atalho tirado do registro de comandos (nunca
//! escrito à mão no Slint).

use crate::Command;

/// Os comandos que a barra de cima chama, da esquerda para a direita (JE-B9).
pub const BAR_COMMANDS: [Command; 5] = [
    Command::CloseAlbum,
    Command::ToggleLayoutBand,
    Command::ToggleStrips,
    Command::TogglePanel,
    Command::Export,
];

/// A dica do botão da barra que roda `command`. `open` diz se o que o botão mostra e esconde está
/// aberto agora (Fotos e Painel): a dica diz o que o clique vai fazer.
pub fn bar_tip(command: Command, open: bool) -> String {
    let action = match command {
        Command::CloseAlbum => "Início: fechar o álbum",
        Command::TogglePhotos if open => "Esconder fotografias",
        Command::TogglePhotos => "Mostrar fotografias",
        // O botão das faixas de baixo (spec FX, regra 8).
        Command::ToggleStrips if open => "Esconder as faixas de baixo",
        Command::ToggleStrips => "Mostrar as faixas de baixo",
        Command::TogglePanel if open => "Esconder painel",
        Command::TogglePanel => "Mostrar painel",
        // O botão da faixa de modelos (conferência do dono 2026-11-08).
        Command::ToggleLayoutBand if open => "Esconder a faixa de modelos",
        Command::ToggleLayoutBand => "Modelos  (L)",
        Command::Export => "Exportar",
        other => other.label().trim_end_matches('…'),
    };
    match command.shortcut() {
        Some(key) => format!("{action} ({})", key.text()),
        None => action.to_owned(),
    }
}
