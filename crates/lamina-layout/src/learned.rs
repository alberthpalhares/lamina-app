//! O aprendizado de layouts (spec AL): quantas vezes o fotógrafo escolheu cada desenho. É só uma
//! conta sobre os pesos guardados: a mesma entrada dá sempre o mesmo bônus.

use std::collections::BTreeMap;

use crate::prefs::FAVORITE_BONUS;

/// O teto do bônus aprendido, em pontos da nota (regra 2); nunca é alcançado.
pub const MAX_LEARNED_BONUS: f64 = 8.0;
/// Quanto um uso vale no peso (seção 4).
pub const WEIGHT_PER_USE: u32 = 1000;
/// Com este número de usos, o bônus chega à metade do teto.
const HALF_BONUS_USES: f64 = 3.0;
/// Peso abaixo disto sai do arquivo (regra 16).
pub const MIN_WEIGHT: u32 = 100;
/// Quantos desenhos o arquivo guarda no máximo (regra 16).
pub const MAX_DESIGNS: usize = 500;

// Um favorito sempre vale mais que o aprendido (regra 2).
const _: () = assert!(MAX_LEARNED_BONUS < FAVORITE_BONUS);

/// O bônus de um desenho com este peso: `8 × usos ÷ (usos + 3)` (regra 2).
pub fn learned_bonus(weight: u32) -> f64 {
    let uses = f64::from(weight) / f64::from(WEIGHT_PER_USE);
    MAX_LEARNED_BONUS * uses / (uses + HALF_BONUS_USES)
}

/// Os pesos dos desenhos, pelo identificador de [`crate::prefs::design_key`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Learned {
    weights: BTreeMap<String, u32>,
}

impl Learned {
    pub fn from_weights(weights: impl IntoIterator<Item = (String, u32)>) -> Self {
        Learned {
            weights: weights.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }

    pub fn weight(&self, key: &str) -> u32 {
        self.weights.get(key).copied().unwrap_or(0)
    }

    /// O bônus do desenho `key`; zero para o que nunca foi escolhido.
    pub fn bonus(&self, key: &str) -> f64 {
        learned_bonus(self.weight(key))
    }

    /// Esquece o desenho (rejeitar apaga o peso dele, regra 4).
    pub fn forget(&mut self, key: &str) {
        self.weights.remove(key);
    }

    /// Todos os pesos, na ordem das chaves: é o que vai para o arquivo.
    pub fn weights(&self) -> &BTreeMap<String, u32> {
        &self.weights
    }

    /// Conta uma escolha de `key`: os outros desenhos perdem um pouco, ele ganha um uso, e o que
    /// ficou leve demais ou passou do limite sai (regras 15 e 16). O relógio não entra.
    pub fn record(&mut self, key: &str) {
        for (other, weight) in &mut self.weights {
            if other != key {
                *weight = decayed(*weight);
            }
        }
        let chosen = self.weights.entry(key.to_owned()).or_insert(0);
        *chosen = chosen.saturating_add(WEIGHT_PER_USE);
        self.weights.retain(|_, weight| *weight >= MIN_WEIGHT);
        self.trim(key);
    }

    /// Acima do limite, saem os de menor peso e, no empate, os primeiros na ordem das chaves. O
    /// recém-escolhido fica, senão um desenho novo nunca entraria num arquivo cheio de pesos altos.
    fn trim(&mut self, chosen: &str) {
        let extra = self.weights.len().saturating_sub(MAX_DESIGNS);
        if extra == 0 {
            return;
        }
        let mut others: Vec<(u32, String)> = self
            .weights
            .iter()
            .filter(|(key, _)| key.as_str() != chosen)
            .map(|(key, weight)| (*weight, key.clone()))
            .collect();
        others.sort_unstable();
        for (_, key) in others.into_iter().take(extra) {
            self.weights.remove(&key);
        }
    }
}

/// `peso × 997 ÷ 1000`, em conta inteira: a mesma sequência dá sempre o mesmo arquivo (regra 15).
fn decayed(weight: u32) -> u32 {
    u32::try_from(u64::from(weight) * 997 / 1000).unwrap_or(weight)
}
