//! As sugestões que o Auto Build considera em cada lâmina, com as preferências do fotógrafo
//! (spec TL, regra 8): as primeiras da variedade mais os favoritos que vierem depois delas, sem
//! os rejeitados.

use crate::layouts::{FamilyKind, LayoutCandidate};
use crate::learned::Learned;
use crate::prefs::{Preferences, design_key};

/// Sugestões consideradas por lâmina: as que passaram pelo rodízio (spec VL, regra 6).
pub const WINDOW: usize = 12;

/// O que o fotógrafo acrescenta ao valor de uma sugestão na escolha.
#[derive(Debug, Clone, Copy, Default)]
pub struct Boost {
    pub favorite: bool,
    /// O bônus aprendido (spec AL, regra 5).
    pub learned: f64,
}

/// As sugestões de uma lâmina e o que cada uma ganha do fotógrafo.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub list: Vec<LayoutCandidate>,
    pub boost: Vec<Boost>,
}

/// O bônus aprendido da sugestão; um modelo nunca tem (spec AL, regra 7).
fn learned_of(c: &LayoutCandidate, aspects: &[f64], learned: &Learned) -> f64 {
    if learned.is_empty() || c.family.kind() == FamilyKind::Model {
        return 0.0;
    }
    learned.bonus(&design_key(c, aspects))
}

/// Recorta a lista da variedade. Sem preferências, são as `WINDOW` primeiras, como sempre.
pub fn window(
    list: Vec<LayoutCandidate>,
    aspects: &[f64],
    prefs: &Preferences,
    learned: &Learned,
) -> Window {
    if prefs.is_empty() {
        let list: Vec<_> = list.into_iter().take(WINDOW).collect();
        let boost = list
            .iter()
            .map(|c| Boost {
                favorite: false,
                learned: learned_of(c, aspects, learned),
            })
            .collect();
        return Window { list, boost };
    }
    let keys: Vec<String> = list.iter().map(|c| design_key(c, aspects)).collect();
    // Todos rejeitados: vale a escolha sem preferências (regra 8).
    let all_rejected = keys.iter().all(|k| prefs.rejected.contains(k));
    let mut out = Window::default();
    let mut kept = 0;
    for (candidate, key) in list.into_iter().zip(&keys) {
        if !all_rejected && prefs.rejected.contains(key) {
            continue;
        }
        let favorite = prefs.favorites.contains(key);
        if kept < WINDOW || favorite {
            out.boost.push(Boost {
                favorite,
                learned: learned_of(&candidate, aspects, learned),
            });
            out.list.push(candidate);
        }
        kept += 1;
    }
    out
}
