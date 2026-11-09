//! "Início rápido" na janela (spec IQ): o botão "Começar" e o item do menu criam o álbum vazio com
//! o tamanho padrão, sem assistente. As decisões ficam no `lamina-editor` (`quick_start`).

use lamina_editor::Human;

use crate::files::show_error;
use crate::state::App;

/// Cria e abre o álbum do início rápido, ainda sem arquivo: o primeiro Salvar pede onde gravar
/// (regra 3). A tela entra no convite do álbum em branco (spec BR), como o "Começar sem fotos".
pub fn open(app: &mut App) {
    match app.editor.quick_start() {
        Ok(()) => {
            app.project_changed();
            crate::recents::refresh(app);
        }
        Err(e) => show_error(app, &e.human()),
    }
}
