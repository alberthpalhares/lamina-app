//! Apoio aos testes da RM (beta 9.1): títulos da faixa, nome do primeiro de fábrica e quadros.

use crate::Editor;

/// Abre a faixa e devolve os títulos das opções.
pub(super) fn titles(editor: &mut Editor) -> Vec<String> {
    let titles = editor.panel(crate::Part::Spread).options;
    titles
        .iter()
        .map(|c| crate::text::option_title(&c.family))
        .collect()
}

/// O nome do primeiro modelo de fábrica que casa com a lâmina atual.
pub(super) fn first_factory_name(editor: &Editor) -> String {
    let panel = editor.panel(crate::Part::Spread);
    for c in &panel.options {
        if let lamina_layout::Family::Model {
            name,
            factory: true,
            ..
        } = &c.family
        {
            return name.clone();
        }
    }
    String::new()
}
