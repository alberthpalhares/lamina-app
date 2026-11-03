//! Trocar layouts pelo editor (spec TL): o painel da lâmina atual, as setas, espelhar e as
//! preferências. As contas estão em [`crate::suggestions`]; aqui fica o estado.

use lamina_core::Project;
use lamina_layout::{LayoutCandidate, LayoutLists, add_models, apply_preferences, design_key};

use super::Editor;
use crate::suggestions::{
    List, Part, Selection, Unavailable, apply, can_mirror, is_in_use, lists_learned, mirror,
    selection,
};

/// O que o painel de layouts mostra para uma parte da lâmina atual.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub list: List,
    pub options: Vec<LayoutCandidate>,
    pub favorite: Vec<bool>,
    /// Posição do layout em uso em `options`.
    pub in_use: Option<usize>,
    pub all_rejected: bool,
    pub selection: Option<Selection>,
    /// Por que a parte não pode trocar de layout.
    pub unavailable: Option<Unavailable>,
}

impl Panel {
    fn unavailable(reason: Unavailable) -> Self {
        Panel {
            unavailable: Some(reason),
            ..Panel::default()
        }
    }
}

impl Editor {
    fn cached_lists(&self, project: &Project, sel: &Selection) -> LayoutLists {
        let mut bits: Vec<u64> = sel.aspects.iter().map(|a| a.to_bits()).collect();
        bits.push(sel.gap.get().to_bits());
        bits.push(u64::from(sel.whole));
        // A montagem e o lay-flat mudam as opções (spec NP, regra 11), também ao desfazer.
        let m = project.mounting;
        bits.push(m.crop as u64);
        bits.push(u64::from(m.bleed_page) | u64::from(m.mosaics) << 1);
        bits.push(u64::from(
            project.spec.mode() == lamina_core::DesignMode::Spread,
        ));
        let key = (bits, sel.active.left, sel.active.right);
        if let Some(found) = self.lists.borrow().get(&key) {
            return found.clone();
        }
        let mut computed = lists_learned(project, sel, self.learned()).unwrap_or_default();
        // Os modelos do fotógrafo valem para a lâmina inteira (spec MD; uma página só fica sem
        // fase). Sem lista do motor (erro), não há onde pôr.
        if sel.whole && !computed.in_order.is_empty() {
            let at = (sel.active, sel.gap);
            let models = self.models.all();
            let _ = add_models(&mut computed, &project.spec, at, &sel.aspects, models);
        }
        self.lists.borrow_mut().insert(key, computed.clone());
        computed
    }

    /// O painel da `part` da lâmina atual, com as preferências aplicadas. A lâmina inteira usa a
    /// lista ativa (na ordem ou reordenadas); uma página só, a lista na ordem.
    pub fn panel(&self, part: Part) -> Panel {
        let Some(session) = &self.session else {
            return Panel::unavailable(Unavailable::NoPhotos);
        };
        let (project, index) = (session.project(), session.current());
        // Com fotos selecionadas no navegador, as opções são para elas (spec FQ, regra 5).
        let chosen = !self.selection.is_empty() && part == Part::Spread;
        let sel = if chosen {
            self.photo_selection()
        } else {
            selection(project, index, part)
        };
        let sel = match sel {
            Ok(sel) => sel,
            Err(reason) => return Panel::unavailable(reason),
        };
        let all = self.cached_lists(project, &sel);
        let list = if part == Part::Spread && self.reordered {
            List::Reordered
        } else {
            List::InOrder
        };
        let raw = match list {
            List::InOrder => all.in_order,
            List::Reordered => all.reordered,
        };
        let preferred = apply_preferences(raw, &sel.aspects, &self.prefs.prefs);
        let in_use = preferred
            .list
            .iter()
            .position(|c| !chosen && is_in_use(project, index, &sel, c));
        Panel {
            list,
            options: preferred.list,
            favorite: preferred.favorite,
            in_use,
            all_rejected: preferred.all_rejected,
            selection: Some(sel),
            unavailable: None,
        }
    }

    /// A lâmina atual tem lista reordenada (fotos de orientações misturadas)?
    pub fn has_reordered(&self) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let (project, index) = (session.project(), session.current());
        selection(project, index, Part::Spread)
            .is_ok_and(|sel| !self.cached_lists(project, &sel).reordered.is_empty())
    }

    /// Aplica a opção `i` do painel da `part` (clique num cartão da faixa). `true` = trocou.
    pub fn apply_option(&mut self, part: Part, i: usize) -> bool {
        // A lâmina fica igual à prévia: ela é limpa e nada desliza de novo (spec FL, regra 6).
        self.clear_hover();
        if part == Part::Spread && !self.selection.is_empty() {
            return self.build_from_selection(i);
        }
        let panel = self.panel(part);
        let (Some(sel), Some(c)) = (panel.selection, panel.options.get(i).cloned()) else {
            return false;
        };
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        match session.edit("Trocar layout", |p| apply(p, index, &sel, &c)) {
            Ok(()) => true,
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }

    pub(super) fn can_step(&self) -> bool {
        let panel = self.panel(Part::Spread);
        panel.unavailable.is_none() && !panel.options.is_empty()
    }

    /// ↓ e ↑: a opção seguinte (ou anterior) da lâmina inteira; no fim volta ao começo (regra 3).
    pub(super) fn step(&mut self, forward: bool) -> bool {
        let panel = self.panel(Part::Spread);
        let n = panel.options.len();
        if n == 0 {
            return false;
        }
        let target = match (panel.in_use, forward) {
            (Some(p), true) => (p + 1) % n,
            (Some(p), false) => (p + n - 1) % n,
            (None, true) => 0,
            (None, false) => n - 1,
        };
        self.choose_option(Part::Spread, target)
    }

    pub(super) fn can_mirror(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| can_mirror(s.project(), s.current()).is_ok())
    }

    pub(super) fn mirror(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        session
            .edit("Espelhar lâmina", |p| mirror(p, index))
            .is_ok()
    }

    /// Identificador do desenho em uso na lâmina inteira, e a posição dele no painel.
    fn in_use_design(&self) -> Option<(String, usize)> {
        let panel = self.panel(Part::Spread);
        let (sel, p) = (panel.selection.as_ref()?, panel.in_use?);
        Some((design_key(&panel.options[p], &sel.aspects), p))
    }

    fn save_prefs(&mut self) {
        if let Err(message) = self.prefs.save() {
            self.notice = Some(message);
        }
    }

    /// Estrela ou "×" num cartão da faixa: favorita ou rejeita a opção `i` da lâmina
    /// inteira. Rejeitar a que está em uso passa para a seguinte.
    pub fn mark_option(&mut self, i: usize, favorite: bool) -> bool {
        // A lista de cartões muda: a prévia é limpa (spec FL, regra 9).
        self.clear_hover();
        let panel = self.panel(Part::Spread);
        let (Some(sel), Some(c)) = (panel.selection.as_ref(), panel.options.get(i)) else {
            return false;
        };
        if !favorite && panel.in_use == Some(i) {
            return self.reject();
        }
        let key = design_key(c, &sel.aspects);
        if favorite {
            self.prefs.prefs.toggle_favorite(&key);
        } else {
            self.prefs.prefs.reject(&key);
            self.forget_learned(&key);
        }
        self.save_prefs();
        true
    }

    pub(super) fn favorite(&mut self) -> bool {
        let Some((key, _)) = self.in_use_design() else {
            return false;
        };
        self.prefs.prefs.toggle_favorite(&key);
        self.save_prefs();
        true
    }

    /// Rejeita o desenho em uso e aplica a opção que ficou no lugar dele (regra 7).
    pub(super) fn reject(&mut self) -> bool {
        let Some((key, position)) = self.in_use_design() else {
            return false;
        };
        self.prefs.prefs.reject(&key);
        self.forget_learned(&key);
        self.save_prefs();
        let panel = self.panel(Part::Spread);
        let n = panel.options.len();
        if n == 0 {
            return true;
        }
        let next = if panel.all_rejected {
            (position + 1) % n
        } else {
            position.min(n - 1)
        };
        self.apply_option(Part::Spread, next);
        true
    }
}
