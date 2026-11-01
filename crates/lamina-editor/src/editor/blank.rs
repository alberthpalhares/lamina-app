//! Página e lâmina em branco pela janela (spec PB): o item do menu de contexto, que vale para a
//! página clicada, e as lâminas novas em branco.

use lamina_core::PageSide;

use super::{ContextEntry, Editor};
use crate::blank::{returned_text, set_blank};
use crate::spreads::insert_blank;

impl Editor {
    /// Clique direito no fundo da página `side`: o menu de contexto, com o item "Página em branco"
    /// valendo para ela.
    pub fn context_menu_on_page(&mut self, side: PageSide) -> Vec<ContextEntry> {
        self.context_side = Some(side);
        self.context_menu(None)
    }

    /// A página do clique direito existe nesta lâmina? (regra 3)
    pub(super) fn can_toggle_blank(&self) -> bool {
        let (Some(s), Some(side)) = (self.session.as_ref(), self.context_side) else {
            return false;
        };
        let active = s
            .project()
            .spec
            .edges()
            .active_pages(s.current(), s.spread_count());
        match side {
            PageSide::Left => active.left,
            PageSide::Right => active.right,
        }
    }

    /// A página do clique direito já está em branco?
    pub(super) fn context_page_is_blank(&self) -> bool {
        let (Some(s), Some(side)) = (self.session.as_ref(), self.context_side) else {
            return false;
        };
        s.project()
            .spreads
            .get(s.current())
            .is_some_and(|spread| spread.blank.has(side))
    }

    /// Marca ou desmarca a página do clique direito (regras 4 e 5).
    pub(super) fn toggle_blank_page(&mut self) -> bool {
        let Some(side) = self.context_side else {
            return false;
        };
        let blank = !self.context_page_is_blank();
        let label = if blank {
            "Página em branco"
        } else {
            "Desmarcar página em branco"
        };
        let mut returned = 0;
        let done = self.change(label, |p, i| {
            returned = set_blank(p, i, side, blank)?;
            Ok(())
        });
        if done && returned > 0 {
            self.notice = Some(returned_text(returned));
        }
        done
    }

    /// Lâmina nova com as páginas em branco, depois da atual ou antes dela (regra 6).
    pub(super) fn new_blank_spread(&mut self, before: bool) -> bool {
        let Some(s) = self.session.as_ref() else {
            return false;
        };
        let at = if before {
            s.current()
        } else {
            (s.current() + 1).min(s.spread_count())
        };
        let label = format!("Nova lâmina em branco · lâmina {}", at + 1);
        let done = self.change(&label, |p, _| insert_blank(p, at));
        if done {
            self.go(at);
        }
        done
    }
}
