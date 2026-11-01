//! A barra única do modo organizar (spec BO): o que cabe na largura da área, o que acende com a
//! seleção, as dicas dos menus apagados e os comandos de cada menu. A tela só desenha.

use crate::Command;

/// Voltar, as ferramentas, o zoom (−, controle, +, valor), o ⋯ e as bordas: nunca saem da barra
/// (regra 3), em px lógicos (spec DG, regra 3).
pub const BAR_FIXED: f64 = 590.0;
/// Ajustar e 1:1.
pub const BAR_FIT: f64 = 72.0;
/// Alinhar, Igualar e Distribuir.
pub const BAR_MENUS: f64 = 270.0;
/// X, Y, L, A e o cadeado.
pub const BAR_FIELDS: f64 = 360.0;

/// Os grupos que cabem na barra; o resto vai para o ⋯.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarFit {
    pub fit: bool,
    pub menus: bool,
    pub fields: bool,
}

/// O que cabe numa área de `width` px (regra 3): saem primeiro os campos, depois os menus e depois
/// Ajustar e 1:1.
pub fn bar_fit(width: f64) -> BarFit {
    let fit = width >= BAR_FIXED + BAR_FIT;
    let menus = fit && width >= BAR_FIXED + BAR_FIT + BAR_MENUS;
    let fields = menus && width >= BAR_FIXED + BAR_FIT + BAR_MENUS + BAR_FIELDS;
    BarFit { fit, menus, fields }
}

/// O que vale com a seleção de agora (regra 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarGroups {
    /// X, Y, L e A: um quadro ou mais.
    pub fields: bool,
    /// Alinhar e Igualar: dois ou mais.
    pub align: bool,
    /// Distribuir: três ou mais.
    pub distribute: bool,
}

/// O que acende com `selected` quadros selecionados.
pub fn bar_groups(selected: usize) -> BarGroups {
    BarGroups {
        fields: selected >= 1,
        align: selected >= 2,
        distribute: selected >= 3,
    }
}

/// Os três menus da barra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarMenu {
    Align,
    Equal,
    Distribute,
}

/// Os menus na ordem da barra.
pub const BAR_MENUS_ORDER: [BarMenu; 3] = [BarMenu::Align, BarMenu::Equal, BarMenu::Distribute];

impl BarMenu {
    /// O nome no botão.
    pub fn name(self) -> &'static str {
        match self {
            BarMenu::Align => "Alinhar",
            BarMenu::Equal => "Igualar",
            BarMenu::Distribute => "Distribuir",
        }
    }

    /// Os comandos do menu, na ordem das linhas (regra 1, item 4).
    pub fn commands(self) -> &'static [Command] {
        match self {
            BarMenu::Align => &[
                Command::AlignLeft,
                Command::AlignCenter,
                Command::AlignRight,
                Command::AlignTop,
                Command::AlignMiddle,
                Command::AlignBottom,
            ],
            BarMenu::Equal => &[
                Command::EqualWidth,
                Command::EqualHeight,
                Command::EqualSize,
            ],
            BarMenu::Distribute => &[Command::DistributeAcross, Command::DistributeDown],
        }
    }

    /// Quantos quadros o menu pede.
    fn needs(self) -> usize {
        match self {
            BarMenu::Align | BarMenu::Equal => 2,
            BarMenu::Distribute => 3,
        }
    }

    /// A dica do botão: o nome, e o porquê quando está apagado (regra 2).
    pub fn tip(self, selected: usize) -> String {
        let need = self.needs();
        if selected >= need {
            self.name().to_owned()
        } else {
            format!("{} (selecione {need} quadros ou mais)", self.name())
        }
    }
}
