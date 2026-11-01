//! Lâminas e histórico pela IA (spec IA, regras 2 e 4; spec FQ, regras 5 a 7): montar, criar,
//! apagar e mover lâminas, desfazer e refazer.

use std::collections::HashSet;

use lamina_core::{PageSide, Project};
use serde_json::json;

use super::{
    Agent, BlankPageCall, BlankSide, BuildSpread, MoveSpread, NewSpreadCall, ToolError, ToolReply,
    edit_on, photo_id, session, session_mut, spread_index,
};
use crate::blank::{returned_text, set_blank};
use crate::edits::{delete_spread, insert_spread, layout_with, move_spread};
use crate::{Editor, EditorError};

impl Agent {
    pub(super) fn build_spread(
        &mut self,
        editor: &mut Editor,
        a: &BuildSpread,
    ) -> Result<ToolReply, ToolError> {
        if a.fotos.is_empty() {
            return Err(ToolError::NoPhotosChosen);
        }
        let p = session(editor)?.project();
        let mut seen = HashSet::new();
        let mut photos = Vec::new();
        for &n in &a.fotos {
            if !seen.insert(n) {
                return Err(ToolError::RepeatedPhoto(n));
            }
            photos.push(photo_id(p, n)?);
        }
        let at = position(p, a.posicao)?;
        let label = format!("Montar lâmina · lâmina {}", at + 1);
        edit_on(editor, at.saturating_sub(1), &label, |p| {
            let new = insert_at(p, at)?;
            layout_with(p, new, &photos)
        })?;
        editor.go_to_spread(at);
        self.edited(
            editor,
            at,
            &format!("Lâmina {} montada com {} fotos.", at + 1, photos.len()),
        )
    }

    pub(super) fn new_spread(
        &mut self,
        editor: &mut Editor,
        a: &NewSpreadCall,
    ) -> Result<ToolReply, ToolError> {
        let at = position(session(editor)?.project(), a.posicao)?;
        if a.em_branco {
            let label = format!("Nova lâmina em branco · lâmina {}", at + 1);
            edit_on(editor, at.saturating_sub(1), &label, |p| {
                crate::spreads::insert_blank(p, at)
            })?;
            editor.go_to_spread(at);
            return self.edited(editor, at, &format!("Lâmina {} criada, em branco.", at + 1));
        }
        let label = format!("Nova lâmina · lâmina {}", at + 1);
        edit_on(editor, at.saturating_sub(1), &label, |p| {
            insert_at(p, at).map(|_| ())
        })?;
        editor.go_to_spread(at);
        self.edited(editor, at, &format!("Lâmina {} criada, vazia.", at + 1))
    }

    /// A página em branco pela IA (spec PB, regra 12): faz o que o menu faz, num passo.
    pub(super) fn blank_page(
        &mut self,
        editor: &mut Editor,
        a: &BlankPageCall,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let active = p.spec.edges().active_pages(index, p.spreads.len());
        let sides: &[PageSide] = match a.lado {
            BlankSide::Esquerda => &[PageSide::Left],
            BlankSide::Direita => &[PageSide::Right],
            BlankSide::Ambas => &[PageSide::Left, PageSide::Right],
        };
        let exists = |side: &PageSide| match side {
            PageSide::Left => active.left,
            PageSide::Right => active.right,
        };
        if a.lado != BlankSide::Ambas && !sides.iter().all(exists) {
            let only = if active.right { "direita" } else { "esquerda" };
            return Err(ToolError::MissingPage {
                spread: a.lamina,
                verb: if index == 0 { "começa" } else { "termina" },
                only,
            });
        }
        let label = if a.em_branco {
            "Página em branco"
        } else {
            "Desmarcar página em branco"
        };
        let label = format!("{label} · lâmina {}", a.lamina);
        let mut returned = 0;
        edit_on(editor, index, &label, |p| {
            for side in sides.iter().filter(|s| exists(s)) {
                returned += set_blank(p, index, *side, a.em_branco)?;
            }
            Ok(())
        })?;
        let text = if returned > 0 {
            format!("{label}: {}.", returned_text(returned))
        } else {
            format!("{label}.")
        };
        self.edited(editor, index, &text)
    }

    pub(super) fn delete_spread(
        &mut self,
        editor: &mut Editor,
        lamina: usize,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, lamina)?;
        if p.spreads.len() <= 1 {
            return Err(ToolError::LastSpread);
        }
        edit_on(editor, index, "Apagar lâmina", |p| delete_spread(p, index))?;
        let shown = index.min(session(editor)?.spread_count() - 1);
        editor.go_to_spread(shown);
        self.edited(editor, shown, &format!("Lâmina {lamina} apagada."))
    }

    pub(super) fn move_spread(
        &mut self,
        editor: &mut Editor,
        a: &MoveSpread,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let count = p.spreads.len();
        let to = a
            .para
            .checked_sub(1)
            .filter(|t| *t < count)
            .ok_or(ToolError::BadPosition(count))?;
        if to == index {
            return Err(ToolError::Refused(format!(
                "A lâmina {} já está nessa posição.",
                a.lamina
            )));
        }
        editor.go_to_spread(index);
        if !editor.move_current_to(to) {
            return Err(ToolError::Refused(editor.take_notice().unwrap_or_default()));
        }
        self.edited(
            editor,
            to,
            &format!("Lâmina {} movida para a posição {}.", a.lamina, a.para),
        )
    }
}

/// Desfaz (ou refaz) `steps` passos, como Ctrl+Z e Ctrl+Y; para quando acabam.
pub(super) fn undo_redo(
    editor: &mut Editor,
    steps: Option<usize>,
    undo: bool,
) -> Result<ToolReply, ToolError> {
    let s = session_mut(editor)?;
    let mut done = Vec::new();
    for _ in 0..steps.unwrap_or(1).max(1) {
        let label = if undo { s.undo() } else { s.redo() };
        match label {
            Some(label) => done.push(label),
            None => break,
        }
    }
    if done.is_empty() {
        return Err(if undo {
            ToolError::NothingToUndo
        } else {
            ToolError::NothingToRedo
        });
    }
    let verb = if undo { "Desfeito" } else { "Refeito" };
    let text = format!("{verb}: {}.", done.join("; "));
    Ok(ToolReply::new(
        text,
        json!({ "passos": done, "alteracoes_nao_salvas": s.is_dirty() }),
    ))
}

/// A posição (a partir de 0) da lâmina nova: `posicao` a partir de 1, ou depois da última.
fn position(project: &Project, posicao: Option<usize>) -> Result<usize, ToolError> {
    let max = project.spreads.len() + 1;
    match posicao {
        None => Ok(max - 1),
        Some(p) if (1..=max).contains(&p) => Ok(p - 1),
        Some(_) => Err(ToolError::BadPosition(max)),
    }
}

/// Uma lâmina vazia na posição `at`, com as pontas rearranjadas como na janela.
fn insert_at(project: &mut Project, at: usize) -> Result<usize, EditorError> {
    if at > 0 {
        return Ok(insert_spread(project, at - 1));
    }
    let new = insert_spread(project, 0);
    // Num álbum em branco, a lâmina nova já é a primeira: não há o que mover (relato do teste
    // pelo MCP, IA-B25; mover para o mesmo lugar recusava com "Quadro ou lâmina que não existe").
    if new != 0 {
        move_spread(project, new, 0)?;
    }
    Ok(0)
}
