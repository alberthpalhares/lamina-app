//! A tabela do registro, parte 2: os comandos dos menus Editar, Lâmina e Layout.

use super::{Command, Info, Menu};
use crate::keys::{Key, Shortcut};

impl Command {
    /// Nome estável, rótulo, menu e atalho dos comandos de Editar, Lâmina e Layout.
    pub(super) fn edit_info(self) -> Option<Info> {
        use Command as C;
        use Shortcut as S;
        let k = |key| Some(S::key(key));
        let shift = |key| Some(S::shift(key));
        Some(match self {
            C::Undo => (
                "editar.desfazer",
                "Desfazer",
                Menu::Edit,
                Some(S::ctrl('z')),
            ),
            C::Redo => ("editar.refazer", "Refazer", Menu::Edit, Some(S::ctrl('y'))),
            C::NextSpread => (
                "lamina.proxima",
                "Próxima lâmina",
                Menu::Spread,
                k(Key::PageDown),
            ),
            C::PreviousSpread => (
                "lamina.anterior",
                "Lâmina anterior",
                Menu::Spread,
                k(Key::PageUp),
            ),
            C::FirstSpread => (
                "lamina.primeira",
                "Primeira lâmina",
                Menu::Spread,
                k(Key::Home),
            ),
            C::LastSpread => ("lamina.ultima", "Última lâmina", Menu::Spread, k(Key::End)),
            C::NextLayout => (
                "layout.proximo",
                "Próximo layout",
                Menu::Layout,
                k(Key::Down),
            ),
            C::PreviousLayout => (
                "layout.anterior",
                "Layout anterior",
                Menu::Layout,
                k(Key::Up),
            ),
            C::Mirror => (
                "layout.espelhar",
                "Espelhar lâmina",
                Menu::Layout,
                k(Key::Char('e')),
            ),
            C::Favorite => (
                "layout.favoritar",
                "Favoritar layout",
                Menu::Layout,
                k(Key::Char('f')),
            ),
            C::Reject => (
                "layout.rejeitar",
                "Rejeitar layout",
                Menu::Layout,
                k(Key::Char('x')),
            ),
            C::ToggleReordered => (
                "layout.reordenadas",
                "Mostrar reordenadas",
                Menu::Layout,
                k(Key::Char('r')),
            ),
            C::ClearFrame => (
                "editar.tirar-foto",
                "Tirar foto ou apagar quadro",
                Menu::Edit,
                k(Key::Delete),
            ),
            C::RemoveFromAlbum => (
                "editar.remover-do-album",
                "Remover fotografia do álbum",
                Menu::Edit,
                None,
            ),
            C::Deselect => ("editar.desmarcar", "Desmarcar", Menu::Edit, k(Key::Escape)),
            C::BuildFromSelection => (
                "layout.montar",
                "Montar lâmina",
                Menu::Layout,
                k(Key::Enter),
            ),
            C::NewSpreadFromSelection => (
                "lamina.nova-com-selecao",
                "Nova lâmina com a seleção",
                Menu::Spread,
                shift(Key::Enter),
            ),
            C::NewSpread => (
                "lamina.nova",
                "Nova lâmina",
                Menu::Spread,
                Some(S::ctrl_shift('n')),
            ),
            C::NewBlankSpread => (
                "lamina.nova-em-branco",
                "Nova lâmina em branco",
                Menu::Spread,
                None,
            ),
            C::NewBlankSpreadBefore => (
                "lamina.nova-em-branco-antes",
                "Nova lâmina em branco antes",
                Menu::Spread,
                None,
            ),
            C::BlankPage => (
                "lamina.pagina-em-branco",
                "Página em branco",
                Menu::Spread,
                None,
            ),
            C::DeleteSpread => (
                "lamina.apagar",
                "Apagar lâmina",
                Menu::Spread,
                Some(S::ctrl_key(Key::Delete)),
            ),
            C::ClearAllPhotos => (
                "lamina.tirar-todas-as-fotos",
                "Tirar as fotos de todas as lâminas…",
                Menu::Spread,
                None,
            ),
            C::DeleteAllSpreads => (
                "lamina.apagar-todas",
                "Apagar todas as lâminas…",
                Menu::Spread,
                None,
            ),
            C::MoveSpreadLeft => (
                "lamina.mover-esquerda",
                "Mover lâmina para a esquerda",
                Menu::Spread,
                Some(S::ctrl_key(Key::Left)),
            ),
            C::MoveSpreadRight => (
                "lamina.mover-direita",
                "Mover lâmina para a direita",
                Menu::Spread,
                Some(S::ctrl_key(Key::Right)),
            ),
            C::CropMode => (
                "recorte.abrir",
                "Recortar foto",
                Menu::Edit,
                k(Key::Char('c')),
            ),
            C::ZoomIn => (
                "recorte.mais-zoom",
                "Mais zoom",
                Menu::Edit,
                k(Key::Char('+')),
            ),
            C::ZoomOut => (
                "recorte.menos-zoom",
                "Menos zoom",
                Menu::Edit,
                k(Key::Char('-')),
            ),
            C::ResetCrop => (
                "recorte.encaixar",
                "Encaixar a foto",
                Menu::Edit,
                k(Key::Char('0')),
            ),
            C::NewText => (
                "lamina.novo-texto",
                "Novo texto",
                Menu::Spread,
                k(Key::Char('t')),
            ),
            C::EditText => ("lamina.editar-texto", "Editar texto", Menu::Spread, None),
            C::NewPhotoFrame => (
                "lamina.novo-quadro",
                "Novo quadro de foto",
                Menu::Spread,
                None,
            ),
            C::BringForward => ("lamina.frente", "Trazer para a frente", Menu::Spread, None),
            C::SendBackward => ("lamina.tras", "Enviar para trás", Menu::Spread, None),
            C::SaveModel => (
                "lamina.salvar-modelo",
                "Salvar como modelo…",
                Menu::Spread,
                None,
            ),
            C::Models => ("lamina.modelos", "Meus modelos…", Menu::Spread, None),
            _ => return None,
        })
    }
}
