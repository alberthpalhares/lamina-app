//! A tabela do registro, parte 5: os comandos da janela nova (Rodada 0b). Entram já com nome,
//! rótulo, menu e atalho, para as sessões da rodada não disputarem a tabela, mas ficam
//! indisponíveis até a spec de cada um ligar o seu (FL, AT, NV, ZR, BR e PF) e ainda não estão no
//! menu. Nunca Ctrl+número: o dono usa o ZoomIt.

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    /// Nome estável, rótulo, menu e atalho dos comandos da janela nova.
    pub(super) fn window_info(self) -> Option<Info> {
        use Command as C;
        let k = |c| Some(Shortcut::key(Key::Char(c)));
        Some(match self {
            C::ToggleLayoutBand => ("ver.faixa-layouts", "Faixa de layouts", Menu::View, k('l')),
            C::ShowShortcuts => (
                "ajuda.atalhos",
                "Atalhos do teclado…",
                Menu::Help,
                Some(Shortcut::ctrl('/')),
            ),
            C::ShowNews => ("ajuda.novidades", "O que há de novo…", Menu::Help, None),
            C::Preferences => (
                "editar.preferencias",
                "Preferências…",
                Menu::Edit,
                Some(Shortcut::ctrl(',')),
            ),
            C::Loupe => ("organizar.lupa", "Lupa", Menu::View, k('z')),
            // 1 e 0 sem Ctrl, só no modo organizar (spec ZR, escolha 17).
            C::ActualSize => (
                "organizar.tamanho-real",
                "Tamanho real (1:1)",
                Menu::View,
                k('1'),
            ),
            C::CalibrateSize => (
                "organizar.calibrar",
                "Calibrar o tamanho real…",
                Menu::View,
                None,
            ),
            C::BuildAlbum => ("lamina.montar-album", "Montar o álbum…", Menu::Spread, None),
            // Ctrl+A só fora do modo organizar: no modo, ele seleciona os quadros (spec EG).
            C::SelectAllPhotos => (
                "editar.selecionar-fotos",
                "Selecionar todas as fotos mostradas",
                Menu::Edit,
                Some(Shortcut::ctrl('a')),
            ),
            _ => return None,
        })
    }
}
