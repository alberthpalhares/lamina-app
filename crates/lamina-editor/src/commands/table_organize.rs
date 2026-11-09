//! A tabela do registro, parte 3: os comandos do modo organizar a lâmina (spec EG).

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    /// Nome estável, rótulo, menu e atalho dos comandos do modo organizar.
    pub(super) fn organize_info(self) -> Option<Info> {
        use Command as C;
        use Shortcut as S;
        let k = |key| Some(S::key(key));
        let shift = |key| Some(S::shift(key));
        Some(match self {
            C::Organize => (
                "lamina.organizar",
                "Organizar lâmina",
                Menu::Spread,
                k(Key::Char('o')),
            ),
            C::SelectAllFrames => (
                "editar.selecionar-quadros",
                "Selecionar todos os quadros",
                Menu::Edit,
                Some(S::ctrl('a')),
            ),
            C::NudgeLeft => (
                "organizar.esquerda",
                "Mover 1 mm para a esquerda",
                Menu::Edit,
                k(Key::Left),
            ),
            C::NudgeRight => (
                "organizar.direita",
                "Mover 1 mm para a direita",
                Menu::Edit,
                k(Key::Right),
            ),
            C::NudgeUp => (
                "organizar.cima",
                "Mover 1 mm para cima",
                Menu::Edit,
                k(Key::Up),
            ),
            C::NudgeDown => (
                "organizar.baixo",
                "Mover 1 mm para baixo",
                Menu::Edit,
                k(Key::Down),
            ),
            C::NudgeLeftMore => (
                "organizar.esquerda-10",
                "Mover 10 mm para a esquerda",
                Menu::Edit,
                shift(Key::Left),
            ),
            C::NudgeRightMore => (
                "organizar.direita-10",
                "Mover 10 mm para a direita",
                Menu::Edit,
                shift(Key::Right),
            ),
            C::NudgeUpMore => (
                "organizar.cima-10",
                "Mover 10 mm para cima",
                Menu::Edit,
                shift(Key::Up),
            ),
            C::NudgeDownMore => (
                "organizar.baixo-10",
                "Mover 10 mm para baixo",
                Menu::Edit,
                shift(Key::Down),
            ),
            C::ViewZoomIn => (
                "organizar.mais-zoom",
                "Aproximar a lâmina",
                Menu::View,
                k(Key::Char('+')),
            ),
            C::ViewZoomOut => (
                "organizar.menos-zoom",
                "Afastar a lâmina",
                Menu::View,
                k(Key::Char('-')),
            ),
            C::ViewFit => (
                "organizar.ajustar",
                "Ajustar a lâmina",
                Menu::View,
                k(Key::Char('0')),
            ),
            C::AlignLeft => (
                "organizar.alinhar-esquerda",
                "Alinhar à esquerda",
                Menu::Spread,
                None,
            ),
            C::AlignCenter => (
                "organizar.alinhar-centro",
                "Alinhar ao centro",
                Menu::Spread,
                None,
            ),
            C::AlignRight => (
                "organizar.alinhar-direita",
                "Alinhar à direita",
                Menu::Spread,
                None,
            ),
            C::AlignTop => (
                "organizar.alinhar-topo",
                "Alinhar ao topo",
                Menu::Spread,
                None,
            ),
            C::AlignMiddle => (
                "organizar.alinhar-meio",
                "Alinhar ao meio",
                Menu::Spread,
                None,
            ),
            C::AlignBottom => (
                "organizar.alinhar-base",
                "Alinhar à base",
                Menu::Spread,
                None,
            ),
            C::EqualWidth => (
                "organizar.igualar-largura",
                "Igualar largura",
                Menu::Spread,
                None,
            ),
            C::EqualHeight => (
                "organizar.igualar-altura",
                "Igualar altura",
                Menu::Spread,
                None,
            ),
            C::EqualSize => (
                "organizar.igualar-tamanho",
                "Igualar tamanho",
                Menu::Spread,
                None,
            ),
            C::DistributeAcross => (
                "organizar.distribuir-horizontal",
                "Distribuir na horizontal",
                Menu::Spread,
                None,
            ),
            C::DistributeDown => (
                "organizar.distribuir-vertical",
                "Distribuir na vertical",
                Menu::Spread,
                None,
            ),
            // Com a lâmina inteira como referência (spec GJ, conferência do dono 2026-11-04).
            C::AlignLeftToSpread => (
                "organizar.alinhar-lamina-esquerda",
                "Alinhar à área da lâmina: esquerda",
                Menu::Spread,
                None,
            ),
            C::AlignCenterToSpread => (
                "organizar.alinhar-lamina-centro",
                "Alinhar à área da lâmina: centro",
                Menu::Spread,
                None,
            ),
            C::AlignRightToSpread => (
                "organizar.alinhar-lamina-direita",
                "Alinhar à área da lâmina: direita",
                Menu::Spread,
                None,
            ),
            C::AlignTopToSpread => (
                "organizar.alinhar-lamina-topo",
                "Alinhar à área da lâmina: topo",
                Menu::Spread,
                None,
            ),
            C::AlignMiddleToSpread => (
                "organizar.alinhar-lamina-meio",
                "Alinhar à área da lâmina: meio",
                Menu::Spread,
                None,
            ),
            C::AlignBottomToSpread => (
                "organizar.alinhar-lamina-base",
                "Alinhar à área da lâmina: base",
                Menu::Spread,
                None,
            ),
            C::DistributeAcrossToSpread => (
                "organizar.distribuir-lamina-horizontal",
                "Distribuir na área da lâmina: horizontal",
                Menu::Spread,
                None,
            ),
            C::DistributeDownToSpread => (
                "organizar.distribuir-lamina-vertical",
                "Distribuir na área da lâmina: vertical",
                Menu::Spread,
                None,
            ),
            C::CopyFrames => ("editar.copiar", "Copiar", Menu::Edit, Some(S::ctrl('c'))),
            C::PasteFrames => ("editar.colar", "Colar", Menu::Edit, Some(S::ctrl('v'))),
            C::DuplicateFrames => (
                "editar.duplicar",
                "Duplicar",
                Menu::Edit,
                Some(S::ctrl('d')),
            ),
            _ => return None,
        })
    }
}
