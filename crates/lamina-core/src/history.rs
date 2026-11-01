//! Desfazer/refazer por snapshots imutáveis (ADR 0005).
//!
//! Cada edição clona o estado atual, altera a cópia e guarda o estado anterior. Como o
//! documento é pequeno (alguns KB), isso é barato e sempre correto: não existe "comando
//! inverso" para escrever errado. A lista de rótulos alimenta o painel de histórico.

use std::sync::Arc;

#[derive(Debug, Clone)]
struct Entry<T> {
    label: String,
    state: Arc<T>,
}

/// Histórico de edições de um valor `T` (normalmente um [`crate::Project`]).
#[derive(Debug, Clone)]
pub struct History<T> {
    present: Arc<T>,
    /// Estados anteriores; o último é o mais recente. O rótulo descreve a edição que saiu dele.
    past: Vec<Entry<T>>,
    /// Estados desfeitos, prontos para refazer; o último é o próximo.
    future: Vec<Entry<T>>,
    limit: usize,
    /// Passos antigos já saíram por causa do limite (spec HI, regra 4).
    trimmed: bool,
}

impl<T: Clone> History<T> {
    /// Quantas edições ficam guardadas por padrão.
    pub const DEFAULT_LIMIT: usize = 200;

    pub fn new(initial: T) -> Self {
        Self::with_limit(initial, Self::DEFAULT_LIMIT)
    }

    pub fn with_limit(initial: T, limit: usize) -> Self {
        History {
            present: Arc::new(initial),
            past: Vec::new(),
            future: Vec::new(),
            limit: limit.max(1),
            trimmed: false,
        }
    }

    /// O estado atual.
    pub fn present(&self) -> &T {
        &self.present
    }

    /// O estado atual compartilhado: quem precisa guardá-lo não copia o valor inteiro (spec LV,
    /// regra 1). Dois `Arc` iguais por [`Arc::ptr_eq`] são o mesmo passo do histórico.
    pub fn present_shared(&self) -> Arc<T> {
        Arc::clone(&self.present)
    }

    /// Aplica uma edição que pode falhar. Se `edit` devolver `Err`, nada muda.
    pub fn try_apply<R, E>(
        &mut self,
        label: impl Into<String>,
        edit: impl FnOnce(&mut T) -> Result<R, E>,
    ) -> Result<R, E> {
        let mut next = T::clone(&self.present);
        let result = edit(&mut next)?;
        let previous = std::mem::replace(&mut self.present, Arc::new(next));
        self.past.push(Entry {
            label: label.into(),
            state: previous,
        });
        if self.past.len() > self.limit {
            self.past.remove(0);
            self.trimmed = true;
        }
        self.future.clear();
        Ok(result)
    }

    /// Emenda a última edição: muda o estado atual sem criar passo novo (digitar ou mover aos
    /// poucos vira uma edição só; spec TX, regra 10). Sem edição para emendar, nada muda e
    /// devolve `Ok(None)`.
    pub fn try_amend<R, E>(
        &mut self,
        edit: impl FnOnce(&mut T) -> Result<R, E>,
    ) -> Result<Option<R>, E> {
        if self.past.is_empty() {
            return Ok(None);
        }
        let mut next = T::clone(&self.present);
        let result = edit(&mut next)?;
        self.present = Arc::new(next);
        self.future.clear();
        Ok(Some(result))
    }

    /// Troca o rótulo da última edição (a sessão só sabe o rótulo inteiro depois de aplicar).
    pub fn relabel_last(&mut self, label: impl Into<String>) {
        if let Some(entry) = self.past.last_mut() {
            entry.label = label.into();
        }
    }

    /// Aplica uma edição que não falha.
    pub fn apply<R>(&mut self, label: impl Into<String>, edit: impl FnOnce(&mut T) -> R) -> R {
        match self.try_apply(label, |state| {
            Ok::<R, std::convert::Infallible>(edit(state))
        }) {
            Ok(result) => result,
            Err(never) => match never {},
        }
    }

    /// Desfaz a última edição. Devolve o rótulo dela, ou `None` se não há o que desfazer.
    pub fn undo(&mut self) -> Option<&str> {
        let entry = self.past.pop()?;
        let current = std::mem::replace(&mut self.present, entry.state);
        self.future.push(Entry {
            label: entry.label,
            state: current,
        });
        self.future.last().map(|e| e.label.as_str())
    }

    /// Refaz a última edição desfeita. Devolve o rótulo dela.
    pub fn redo(&mut self) -> Option<&str> {
        let entry = self.future.pop()?;
        let current = std::mem::replace(&mut self.present, entry.state);
        self.past.push(Entry {
            label: entry.label,
            state: current,
        });
        self.past.last().map(|e| e.label.as_str())
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Rótulos das edições que podem ser desfeitas, da mais antiga para a mais recente.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.past.iter().map(|e| e.label.as_str())
    }

    /// Rótulos das edições que podem ser refeitas, da próxima para a última.
    pub fn redo_labels(&self) -> impl Iterator<Item = &str> {
        self.future.iter().rev().map(|e| e.label.as_str())
    }

    /// Os passos do histórico (spec HI): o passo 0 é o começo; o passo `k` é o estado logo
    /// depois da `k`-ésima edição guardada. Inclui os desfeitos, que ainda podem ser refeitos.
    pub fn len(&self) -> usize {
        self.past.len() + self.future.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// O passo atual.
    pub fn position(&self) -> usize {
        self.past.len()
    }

    /// Rótulo da edição que leva ao passo `step` (de 1 a [`History::len`]).
    pub fn label(&self, step: usize) -> Option<&str> {
        let p = self.past.len();
        let entry = if step == 0 {
            return None;
        } else if step <= p {
            self.past.get(step - 1)
        } else {
            self.future
                .len()
                .checked_sub(step - p)
                .and_then(|i| self.future.get(i))
        };
        entry.map(|e| e.label.as_str())
    }

    /// O estado no passo `step`.
    pub fn state_at(&self, step: usize) -> Option<&T> {
        let p = self.past.len();
        if step < p {
            self.past.get(step).map(|e| &*e.state)
        } else if step == p {
            Some(&self.present)
        } else {
            let i = self.future.len().checked_sub(step - p)?;
            self.future.get(i).map(|e| &*e.state)
        }
    }

    /// Vai direto ao passo `step`, desfazendo ou refazendo o quanto for preciso. `false` se o
    /// passo não existe ou já é o atual.
    pub fn go_to(&mut self, step: usize) -> bool {
        if step > self.len() || step == self.position() {
            return false;
        }
        while self.position() > step && self.undo().is_some() {}
        while self.position() < step && self.redo().is_some() {}
        true
    }

    /// Passos antigos já saíram por causa do limite.
    pub fn trimmed(&self) -> bool {
        self.trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emendar_muda_o_ultimo_passo_sem_criar_outro() {
        let mut history = History::new(String::new());
        assert_eq!(
            history.try_amend(|s| {
                s.push('x');
                Ok::<(), ()>(())
            }),
            Ok(None)
        );
        assert_eq!(history.present(), "");
        history.apply("digitar", |s| s.push('a'));
        history
            .try_amend(|s| {
                s.push('b');
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(history.present(), "ab");
        assert_eq!(history.len(), 1);
        assert_eq!(history.undo(), Some("digitar"));
        assert_eq!(history.present(), "");
    }

    #[test]
    fn desfaz_e_refaz() {
        let mut history = History::new(vec![1]);
        history.apply("adicionar 2", |v| v.push(2));
        history.apply("adicionar 3", |v| v.push(3));
        assert_eq!(history.present(), &vec![1, 2, 3]);

        assert_eq!(history.undo(), Some("adicionar 3"));
        assert_eq!(history.present(), &vec![1, 2]);
        assert_eq!(history.undo(), Some("adicionar 2"));
        assert_eq!(history.undo(), None);
        assert_eq!(history.present(), &vec![1]);

        assert_eq!(history.redo(), Some("adicionar 2"));
        assert_eq!(history.present(), &vec![1, 2]);
    }

    #[test]
    fn passos_e_ir_a_um_passo() {
        let mut history = History::new(0);
        for i in 1..=3 {
            history.apply(format!("+{i}"), |n| *n += i);
        }
        assert_eq!((history.len(), history.position()), (3, 3));
        assert!(history.go_to(1));
        assert_eq!((*history.present(), history.position()), (1, 1));
        assert_eq!(history.label(3), Some("+3"));
        assert_eq!(history.state_at(3), Some(&6));
        assert_eq!(history.state_at(0), Some(&0));
        assert_eq!(history.redo_labels().collect::<Vec<_>>(), ["+2", "+3"]);
        assert!(history.go_to(3));
        assert_eq!(*history.present(), 6);
        assert!(!history.go_to(3) && !history.go_to(9));
    }

    #[test]
    fn limite_corta_o_mais_antigo() {
        let mut history = History::with_limit(0, 2);
        for i in 1..=3 {
            history.apply(format!("+{i}"), |n| *n += i);
        }
        assert!(history.trimmed());
        assert_eq!(history.len(), 2);
        assert_eq!(
            history.state_at(0),
            Some(&1),
            "começo guardado: depois da 1ª"
        );
    }

    #[test]
    fn nova_edicao_apaga_o_refazer() {
        let mut history = History::new(0);
        history.apply("+1", |n| *n += 1);
        history.undo();
        assert!(history.can_redo());
        history.apply("+10", |n| *n += 10);
        assert!(!history.can_redo());
        assert_eq!(*history.present(), 10);
    }

    #[test]
    fn edicao_que_falha_nao_entra_no_historico() {
        let mut history = History::new(5);
        let result: Result<(), &str> = history.try_apply("falha", |n| {
            *n = 99;
            Err("não pode")
        });
        assert_eq!(result, Err("não pode"));
        assert_eq!(*history.present(), 5);
        assert!(!history.can_undo());
    }

    #[test]
    fn respeita_o_limite() {
        let mut history = History::with_limit(0, 3);
        for i in 1..=5 {
            history.apply(format!("passo {i}"), |n| *n = i);
        }
        let labels: Vec<_> = history.undo_labels().collect();
        assert_eq!(labels, ["passo 3", "passo 4", "passo 5"]);
    }
}
