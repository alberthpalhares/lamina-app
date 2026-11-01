//! O aprendizado de layouts no editor (spec AL, regras 8 a 14 e 18): a visita da lâmina, a contagem
//! ao sair e o apagar. Contar não é editar: nada entra no histórico nem no `.lamina`.

use std::collections::HashSet;

use lamina_core::SpreadId;
use lamina_layout::{FamilyKind, LayoutCandidate, Learned, design_key};

use super::Editor;
use crate::human::HumanMessage;
use crate::learned_store::{LearnedStore, start_learned_worker};
use crate::suggestions::{Part, Selection, is_in_use};

/// A resposta do item do menu sem nada aprendido (regra 18).
pub const NOTHING_LEARNED: &str = "Nada aprendido ainda.";
/// A pergunta antes de apagar (regra 18).
pub const LEARNED_QUESTION: &str = "Apagar o que o Lamina aprendeu sobre os layouts que você usa?";
pub const LEARNED_QUESTION_DETAIL: &str =
    "As listas voltam à ordem original. Favoritos, rejeitados e álbuns não mudam.";
const LEARNED_DELETED: &str = "Aprendizado de layouts apagado.";

/// A escolha guardada enquanto a lâmina fica aberta (regra 8). A lâmina é reconhecida pelo
/// `SpreadId`, nunca pela posição: mover a lâmina não encerra a visita.
#[derive(Debug)]
struct LayoutVisit {
    spread: SpreadId,
    key: String,
    selection: Selection,
    option: LayoutCandidate,
}

#[derive(Debug, Default)]
pub(super) struct Learning {
    pub(super) store: LearnedStore,
    visit: Option<LayoutVisit>,
    /// O que já contou com este álbum aberto: ir e voltar não infla o número (regra 11).
    counted: HashSet<(SpreadId, String)>,
}

impl Editor {
    /// O que o fotógrafo mais escolhe (spec AL).
    pub fn learned(&self) -> &Learned {
        &self.learning.store.learned
    }

    /// Passa a usar `learned`: as listas guardadas tinham a ordem anterior.
    pub fn set_learned(&mut self, learned: Learned) {
        self.learning.store.learned = learned;
        self.lists.borrow_mut().clear();
    }

    /// Passa a aprender no arquivo de `store`. Arquivo ilegível só vai para o registro (seção 6).
    pub fn set_learned_store(&mut self, store: LearnedStore) {
        if let (true, Some(log)) = (store.unreadable(), &self.log) {
            log.note("não foi possível ler o aprendizado de layouts; começa do zero");
        }
        self.learning.store = store;
        self.lists.borrow_mut().clear();
    }

    /// Liga o trabalhador que grava fora da linha da janela (regra 19).
    pub fn start_learned_worker(&mut self) {
        let queue = start_learned_worker(self.log.clone());
        self.learning.store.set_queue(queue);
    }

    /// Não há nada para apagar? O item do menu responde com [`NOTHING_LEARNED`] (regra 18).
    pub fn nothing_learned(&self) -> bool {
        !self.learning.store.has_anything()
    }

    /// A escolha do fotógrafo na faixa ou pelas setas: aplica a opção e abre a visita (regra 8).
    /// A IA e o que o programa refaz sozinho usam [`Editor::apply_option`], que não conta.
    pub fn choose_option(&mut self, part: Part, i: usize) -> bool {
        self.settle_visit();
        let visit = self.visit_for(part, i);
        let done = self.apply_option(part, i);
        if done {
            self.learning.visit = visit;
        }
        done
    }

    /// A visita da opção `i`, com os dados de antes de aplicar: é com eles que ela foi aplicada.
    /// Uma página só, a capa e os modelos não contam (regras 7 e 12).
    fn visit_for(&self, part: Part, i: usize) -> Option<LayoutVisit> {
        let session = self.session.as_ref()?;
        if part != Part::Spread || session.on_cover() || !self.learning.store.can_learn() {
            return None;
        }
        let panel = self.panel(part);
        let (selection, option) = (panel.selection?, panel.options.get(i)?.clone());
        if option.family.kind() == FamilyKind::Model {
            return None;
        }
        Some(LayoutVisit {
            spread: session.album().spreads.get(session.current())?.id,
            key: design_key(&option, &selection.aspects),
            selection,
            option,
        })
    }

    /// A lâmina aberta mudou? Então a visita terminou (regra 10). Com a capa aberta, nenhuma lâmina
    /// está aberta, embora `current()` devolva 0.
    pub(crate) fn settle_visit(&mut self) {
        let Some(visit) = &self.learning.visit else {
            return;
        };
        let open = self
            .session
            .as_ref()
            .filter(|s| !s.on_cover())
            .and_then(|s| s.album().spreads.get(s.current()))
            .map(|s| s.id);
        if open != Some(visit.spread) {
            self.close_visit();
        }
    }

    /// Encerra a visita: conta se a opção ainda está em uso, uma vez por lâmina (regras 9 e 11).
    /// Lâmina apagada não conta.
    pub(super) fn close_visit(&mut self) {
        let Some(visit) = self.learning.visit.take() else {
            return;
        };
        let in_use = self.session.as_ref().is_some_and(|s| {
            let album = s.album();
            album
                .spreads
                .iter()
                .position(|spread| spread.id == visit.spread)
                .is_some_and(|i| is_in_use(album, i, &visit.selection, &visit.option))
        });
        if !in_use
            || !self
                .learning
                .counted
                .insert((visit.spread, visit.key.clone()))
        {
            return;
        }
        self.learning.store.learned.record(&visit.key);
        self.learning.store.save();
        // A lista só muda aqui, com a visita terminada (regra 13).
        self.lists.borrow_mut().clear();
    }

    /// "Não salvar": a escolha da lâmina aberta foi descartada junto com o trabalho (regra 10).
    pub(super) fn discard_visit(&mut self) {
        self.learning.visit = None;
    }

    /// Outro álbum: a contagem de "uma vez por lâmina" recomeça (regra 11).
    pub(super) fn forget_visits(&mut self) {
        self.learning.visit = None;
        self.learning.counted.clear();
    }

    /// Sair do Lamina: conta a visita aberta e espera a fila acabar (regra 19).
    pub fn flush_learned(&mut self) {
        self.close_visit();
        self.learning.store.flush();
    }

    /// Rejeitar um desenho apaga o peso dele (regra 4), e a escolha dele não conta mais.
    pub(super) fn forget_learned(&mut self, key: &str) {
        if self.learning.visit.as_ref().is_some_and(|v| v.key == key) {
            self.learning.visit = None;
        }
        if self.learned().weight(key) > 0 {
            self.learning.store.learned.forget(key);
            self.learning.store.save();
            self.lists.borrow_mut().clear();
        }
    }

    /// Apaga o aprendizado (regra 18). `true` sempre: há aviso novo para mostrar. Não dá para
    /// desfazer com Ctrl+Z; se o arquivo não sair, nada muda.
    pub(super) fn delete_learned(&mut self) -> bool {
        match self.learning.store.delete() {
            Ok(()) => {
                self.forget_visits();
                self.lists.borrow_mut().clear();
                self.notice = Some(LEARNED_DELETED.to_owned());
            }
            Err(reason) => {
                let message = HumanMessage::new(
                    "Não foi possível apagar o aprendizado de layouts.",
                    "Confira se o arquivo não está aberto em outro programa e tente de novo.",
                    reason,
                );
                self.record(&message);
                self.notice = Some(message.line());
            }
        }
        true
    }
}
