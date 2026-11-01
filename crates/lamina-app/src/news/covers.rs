//! As capas na memória (spec PV, regra 10): só as das linhas que aparecem, no máximo 16. A linha que
//! voltar à tela pede de novo.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use lamina_editor::updates::news::cover;
use slint::{ComponentHandle, Image, Model, Timer};

use super::{SHOWN, Shown, rows};
use crate::AppWindow;
use crate::news_player;

/// Mais que isto a tela nunca mostra: com o limite acima das linhas visíveis, soltar a capa mais
/// distante da última pedida nunca solta uma da tela.
pub const MAX_COVERS: usize = 16;

/// As capas prontas, por índice do item, e a fila das pedidas.
#[derive(Default)]
pub struct Covers {
    loaded: BTreeMap<usize, Image>,
    queue: VecDeque<usize>,
    /// Há um temporizador agendado para trazer a próxima capa.
    pumping: bool,
}

impl Covers {
    pub fn get(&self, index: usize) -> Option<Image> {
        self.loaded.get(&index).cloned()
    }

    /// Pede a capa do item; `true` quando quem pediu precisa agendar a leitura da fila.
    pub fn ask(&mut self, index: usize) -> bool {
        if self.loaded.contains_key(&index) || self.queue.contains(&index) {
            return false;
        }
        self.queue.push_back(index);
        !std::mem::replace(&mut self.pumping, true)
    }

    /// O próximo item da fila.
    pub fn next(&mut self) -> Option<usize> {
        let next = self.queue.pop_front();
        if next.is_none() {
            self.pumping = false;
        }
        next
    }

    /// Ainda há capa na fila; sem nenhuma, o próximo pedido agenda a leitura de novo.
    pub fn has_more(&mut self) -> bool {
        self.pumping = !self.queue.is_empty();
        self.pumping
    }

    pub fn keep(&mut self, index: usize, image: Image) {
        self.loaded.insert(index, image);
    }

    /// Passou do limite: solta a capa mais distante de `near` na lista, começando pelas que nem estão
    /// nela, e devolve o item dela.
    pub fn over_limit(&mut self, near: usize, listed: &[usize]) -> Option<usize> {
        if self.loaded.len() <= MAX_COVERS {
            return None;
        }
        let row = |item: usize| listed.iter().position(|i| *i == item);
        let from = row(near).unwrap_or(0);
        let far = self
            .loaded
            .keys()
            .copied()
            .max_by_key(|item| row(*item).map_or(usize::MAX, |at| at.abs_diff(from)))?;
        self.loaded.remove(&far);
        Some(far)
    }

    /// Solta tudo; um pedido na fila não vira capa depois de fechar.
    pub fn clear(&mut self) {
        self.loaded.clear();
        self.queue.clear();
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.loaded.len()
    }

    /// Quanto as capas ocupam na memória, em bytes.
    #[cfg(test)]
    pub fn bytes(&self) -> usize {
        self.loaded
            .values()
            .map(|image| {
                let size = image.size();
                size.width as usize * size.height as usize * 4
            })
            .sum()
    }
}

/// Uma linha ou um cartão nasceu: a capa entra na fila. Uma por vez e fora do desenho, para rolar a
/// lista não engasgar (regra 10).
pub fn request(window: &AppWindow, index: usize) {
    let start = SHOWN.with_borrow_mut(|s| s.covers.ask(index));
    if start {
        schedule(window.as_weak());
    }
}

fn schedule(window: slint::Weak<AppWindow>) {
    Timer::single_shot(Duration::ZERO, move || {
        if let Some(w) = window.upgrade() {
            load_next(&w);
        }
    });
}

/// Traz a próxima capa da fila, põe nas linhas dela e solta a mais distante, se passou do limite.
fn load_next(window: &AppWindow) {
    let more = SHOWN.with_borrow_mut(|s| {
        let Some(index) = s.covers.next() else {
            return false;
        };
        let picture = s.all.get(index).and_then(rows::bytes_of).and_then(cover);
        if let Some(picture) = picture {
            let image = news_player::image(&picture);
            s.covers.keep(index, image.clone());
            put_cover(s, index, Some(image));
            if let Some(far) = s.covers.over_limit(index, &s.listed) {
                put_cover(s, far, None);
            }
        }
        s.covers.has_more()
    });
    if more {
        schedule(window.as_weak());
    }
}

/// Grava a capa só nas linhas do item (`set_row_data`), sem refazer a lista.
fn put_cover(s: &Shown, index: usize, image: Option<Image>) {
    let target = i32::try_from(index).unwrap_or(-1);
    if let Some(at) = s.listed.iter().position(|i| *i == index)
        && let Some(mut row) = s.rows.row_data(at)
    {
        row.has_cover = image.is_some();
        row.cover = image.clone().unwrap_or_default();
        s.rows.set_row_data(at, row);
    }
    for at in 0..s.cards.row_count() {
        if let Some(mut card) = s.cards.row_data(at)
            && card.index == target
        {
            card.has_cover = image.is_some();
            card.cover = image.clone().unwrap_or_default();
            s.cards.set_row_data(at, card);
        }
    }
}
