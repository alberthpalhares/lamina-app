//! A tabela do registro, parte 1: os comandos dos menus Arquivo, Visualizar e Ajuda.

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    /// Nome estável, rótulo, menu e atalho dos comandos de Arquivo, Visualizar e Ajuda.
    pub(super) fn app_info(self) -> Option<Info> {
        use Command as C;
        use Shortcut as S;
        let k = |key| Some(S::key(key));
        Some(match self {
            C::NewAlbum => (
                "arquivo.novo",
                "Novo álbum…",
                Menu::File,
                Some(S::ctrl('n')),
            ),
            C::QuickStart => ("arquivo.inicio-rapido", "Início rápido", Menu::File, None),
            C::Open => ("arquivo.abrir", "Abrir…", Menu::File, Some(S::ctrl('o'))),
            C::Save => ("arquivo.salvar", "Salvar", Menu::File, Some(S::ctrl('s'))),
            C::SaveAs => (
                "arquivo.salvar-como",
                "Salvar como…",
                Menu::File,
                Some(S::ctrl_shift('s')),
            ),
            C::Export => (
                "arquivo.exportar",
                "Exportar…",
                Menu::File,
                Some(S::ctrl('e')),
            ),
            C::OpenExample => (
                "arquivo.exemplo",
                "Abrir o álbum de exemplo",
                Menu::File,
                None,
            ),
            C::RenameAlbum => ("arquivo.renomear", "Renomear álbum…", Menu::File, None),
            C::CloseAlbum => (
                "arquivo.fechar",
                "Fechar álbum",
                Menu::File,
                Some(S::ctrl('w')),
            ),
            C::Quit => ("arquivo.sair", "Sair", Menu::File, Some(S::ctrl('q'))),
            C::ClearRecents => (
                "arquivo.limpar-recentes",
                "Limpar recentes não fixados…",
                Menu::File,
                None,
            ),
            C::FindMissing => (
                "arquivo.procurar-fotos",
                "Reencontrar fotos…",
                Menu::File,
                None,
            ),
            C::LocatePhoto => (
                "arquivo.localizar-foto",
                "Localizar esta foto…",
                Menu::File,
                None,
            ),
            C::Specification => (
                "arquivo.especificacao",
                "Especificação do álbum…",
                Menu::File,
                None,
            ),
            C::Presets => ("arquivo.presets", "Presets…", Menu::File, None),
            C::ImportPhotos => (
                "arquivo.importar",
                "Importar fotos…",
                Menu::File,
                Some(S::ctrl('i')),
            ),
            C::ImportFolder => (
                "arquivo.importar-pasta",
                "Importar uma pasta de fotos…",
                Menu::File,
                None,
            ),
            C::MountAlbum => ("arquivo.montagem", "Montagem do álbum…", Menu::File, None),
            C::ToggleGuides => ("ver.guias", "Mostrar guias", Menu::View, k(Key::Char('g'))),
            C::ToggleSpreads => (
                "ver.faixa-laminas",
                "Faixa de lâminas",
                Menu::View,
                Some(S::alt('1')),
            ),
            C::ToggleTimeline => (
                "ver.linha-do-tempo",
                "Sequência",
                Menu::View,
                Some(S::alt('2')),
            ),
            C::TogglePhotos => ("ver.fotos", "Fotografias", Menu::View, Some(S::alt('3'))),
            C::ToggleStrips => (
                "ver.faixas",
                "Faixas de baixo",
                Menu::View,
                Some(S::alt('0')),
            ),
            C::ToggleReduceMotion => (
                "ver.reduzir-movimento",
                "Reduzir movimento",
                Menu::View,
                None,
            ),
            C::ToggleGutterShadow => ("ver.sombra-dobra", "Sombra da dobra", Menu::View, None),
            C::About => ("ajuda.sobre", "Sobre o Lamina", Menu::Help, None),
            C::CheckUpdates => (
                "ajuda.verificar-atualizacoes",
                "Verificar atualizações agora",
                Menu::Help,
                None,
            ),
            C::ToggleAutoUpdates => (
                "ajuda.atualizar-automaticamente",
                "Verificar atualizações automaticamente",
                Menu::Help,
                None,
            ),
            C::ToggleBeta => (
                "ajuda.versoes-de-teste",
                "Receber versões de teste (beta)",
                Menu::Help,
                None,
            ),
            C::SendFeedback => (
                "ajuda.enviar-sugestao",
                "Enviar sugestão ou problema…",
                Menu::Help,
                None,
            ),
            C::OpenLogs => (
                "ajuda.abrir-registros",
                "Abrir pasta de registros",
                Menu::Help,
                None,
            ),
            _ => return None,
        })
    }
}
