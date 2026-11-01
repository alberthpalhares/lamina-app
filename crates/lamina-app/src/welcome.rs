//! A tela de boas-vindas e o álbum de exemplo (spec PC): acha o exemplo que veio com o programa,
//! prepara a cópia do fotógrafo e a abre pelo mesmo caminho do "Abrir". A cópia é do editor.

use std::path::PathBuf;

use lamina_editor::{EXAMPLE_DIR, EXAMPLE_FILE, Human, example_target, prepare_example};

use crate::files::{open_file, show_error};
use crate::state::App;

/// Onde o exemplo pode estar: ao lado do programa (Windows, Linux e dentro do `.app`), em
/// `Contents/Resources` no Mac, e no repositório quando a equipe roda pelo `cargo run`.
fn candidates() -> Vec<PathBuf> {
    // Os testes apontam outra origem (PC-B8).
    if let Some(dir) = std::env::var_os("LAMINA_EXEMPLO") {
        return vec![PathBuf::from(dir)];
    }
    let mut list = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        list.push(dir.join(EXAMPLE_DIR));
        list.push(dir.join("..").join("Resources").join(EXAMPLE_DIR));
    }
    list.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(EXAMPLE_DIR));
    list
}

/// A pasta do exemplo que veio com o programa (a primeira que tem o álbum).
pub fn example_source() -> PathBuf {
    let list = candidates();
    list.iter()
        .find(|dir| dir.join(EXAMPLE_FILE).is_file())
        .or(list.first())
        .cloned()
        .unwrap_or_else(|| PathBuf::from(EXAMPLE_DIR))
}

/// "Álbum de exemplo" na tela inicial e Arquivo → "Abrir o álbum de exemplo", depois da
/// pergunta de salvar (regras 3 e 4).
pub fn open_example(app: &mut App) {
    match prepare_example(&example_source(), &example_target()) {
        Ok(file) => open_file(app, &file),
        Err(e) => show_error(app, &e.human()),
    }
}
