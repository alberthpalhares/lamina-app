//! Modelos do fotógrafo pela IA (spec IA, `usar_modelo` e `salvar_modelo`; spec MD): aplicar a
//! opção "Modelo · <nome>" do painel e salvar a lâmina como modelo, como Lâmina → Salvar como
//! modelo.

use lamina_layout::Family;
use serde_json::json;

use super::{Agent, SaveModel, ToolError, ToolReply, UseModel, session, spread_index};
use crate::{Command, Editor, Part};

impl Agent {
    pub(super) fn use_model(
        &mut self,
        editor: &mut Editor,
        a: &UseModel,
    ) -> Result<ToolReply, ToolError> {
        let index = spread_index(session(editor)?.project(), a.lamina)?;
        let name = model_name(editor, &a.nome)?;
        editor.go_to_spread(index);
        // O modelo pode casar só com as fotos noutra ordem (spec MD, regra 3): na lista
        // reordenada, que a IA usa sem deixar a janela trocada.
        let mut option = model_option(editor, &name, a.espelhado)?;
        let toggled = option.is_none() && editor.has_reordered();
        if toggled {
            editor.run(Command::ToggleReordered);
            option = model_option(editor, &name, a.espelhado).unwrap_or(None);
        }
        let applied = option.map(|i| editor.apply_option(Part::Spread, i));
        if toggled {
            editor.run(Command::ToggleReordered);
        }
        match applied {
            None => {
                return Err(ToolError::ModelDoesNotFit {
                    name,
                    spread: a.lamina,
                });
            }
            Some(false) => {
                let why = editor.take_notice();
                return Err(ToolError::Refused(
                    why.unwrap_or_else(|| "Não deu para aplicar esse modelo.".into()),
                ));
            }
            Some(true) => {}
        }
        self.lists.retain(|(i, _), _| *i != index);
        let text = format!("Modelo {name} aplicado na lâmina {}.", a.lamina);
        self.edited(editor, index, &text)
    }

    pub(super) fn save_model(
        &mut self,
        editor: &mut Editor,
        a: &SaveModel,
    ) -> Result<ToolReply, ToolError> {
        let index = spread_index(session(editor)?.project(), a.lamina)?;
        let name = a.nome.trim();
        if editor.model_exists(name) && !a.substituir {
            return Err(ToolError::ModelExists(name.to_owned()));
        }
        editor.go_to_spread(index);
        editor.save_model(name)?;
        // O modelo novo entra nas listas do painel: as opções de antes mudam de número.
        self.lists.clear();
        let names: Vec<&str> = editor.models().iter().map(|m| m.name.as_str()).collect();
        Ok(ToolReply::new(
            format!("Lâmina {} salva como o modelo {name}.", a.lamina),
            json!({ "modelo": name, "modelos": names }),
        ))
    }
}

/// A posição da opção "Modelo · `name`" no painel da lâmina atual; sem `mirrored`, a normal ou,
/// se só ela casa, a espelhada.
fn model_option(
    editor: &Editor,
    name: &str,
    mirrored: Option<bool>,
) -> Result<Option<usize>, ToolError> {
    let panel = editor.panel(Part::Spread);
    if let Some(reason) = &panel.unavailable {
        return Err(ToolError::Refused(reason.to_string()));
    }
    let pick = |wanted: bool| {
        panel.options.iter().position(|c| {
            matches!(&c.family, Family::Model { name: n, mirrored: m, .. } if n == name && *m == wanted)
        })
    };
    Ok(match mirrored {
        Some(wanted) => pick(wanted),
        None => pick(false).or_else(|| pick(true)),
    })
}

/// O nome do modelo como o fotógrafo salvou (sem diferença de maiúsculas no pedido).
fn model_name(editor: &Editor, asked: &str) -> Result<String, ToolError> {
    let models = editor.models();
    if models.is_empty() {
        return Err(ToolError::NoModels);
    }
    let asked = asked.trim();
    models
        .iter()
        .find(|m| m.name == asked)
        .or_else(|| {
            models
                .iter()
                .find(|m| m.name.to_lowercase() == asked.to_lowercase())
        })
        .map(|m| m.name.clone())
        .ok_or_else(|| ToolError::NoModel {
            name: asked.to_owned(),
            known: models
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        })
}
