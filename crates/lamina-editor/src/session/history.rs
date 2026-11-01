//! O painel de histórico (spec HI): a lista dos passos, ir a um passo e os rótulos do menu.

use super::Session;

/// Uma linha da lista do histórico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryStep {
    /// "Trocar layout · lâmina 3".
    pub label: String,
    /// Desfeito (ainda dá para refazer): aparece apagado.
    pub undone: bool,
    /// É o estado gravado no arquivo.
    pub saved: bool,
}

/// A lista inteira: o começo, os passos e o passo atual (0 = o começo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryList {
    /// "Álbum aberto", "Álbum criado" ou "Início do histórico guardado".
    pub start: String,
    pub start_saved: bool,
    pub steps: Vec<HistoryStep>,
    pub position: usize,
}

/// A lâmina (a partir de 0) que o rótulo do passo cita: "· lâmina 3" → 2; "· 2 → 5" → 4; "· todas
/// as lâminas" → nenhuma.
fn spread_of(label: &str) -> Option<usize> {
    let tail = label.split(" · ").nth(1)?;
    let number = match tail.split_once("→ ") {
        Some((_, to)) => to,
        None => tail.strip_prefix("lâmina ")?,
    };
    number.trim().parse::<usize>().ok()?.checked_sub(1)
}

/// Separa o rótulo da lâmina: "Trocar layout · lâmina 3" → "Trocar layout".
fn base(label: &str) -> &str {
    label.split(" · ").next().unwrap_or(label)
}

impl Session {
    /// A lista do histórico para o painel (regras 1 a 5).
    pub fn history_list(&self) -> HistoryList {
        let h = &self.history;
        let saved = |step: usize| {
            self.saved.as_ref().is_some_and(|s| {
                h.state_at(step)
                    .is_some_and(|at| at.export == s.export && at.same_except_export(s))
            })
        };
        let start = if h.trimmed() {
            "Início do histórico guardado"
        } else if self.opened {
            "Álbum aberto"
        } else {
            "Álbum criado"
        };
        let steps = (1..=h.len())
            .map(|step| HistoryStep {
                label: h.label(step).unwrap_or_default().to_owned(),
                undone: step > h.position(),
                saved: saved(step),
            })
            .collect();
        HistoryList {
            start: start.to_owned(),
            start_saved: saved(0),
            steps,
            position: h.position(),
        }
    }

    /// Vai ao passo `step` (0 = o começo); a lâmina atual vai para a lâmina do passo, ou para a
    /// que mudou quando o passo não cita uma (regra 2).
    pub fn go_to_step(&mut self, step: usize) -> bool {
        let before = self.album_shared();
        if !self.history.go_to(step) {
            return false;
        }
        self.after_step(&before);
        let cited = self.history.label(step).and_then(spread_of);
        if let Some(index) = cited.filter(|i| *i < self.spread_count()) {
            self.current = index;
        }
        true
    }

    /// "Trocar layout": o que Ctrl+Z vai desfazer (menu Editar).
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_labels().last().map(base)
    }

    /// O que Ctrl+Y vai refazer.
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_labels().next().map(base)
    }
}
