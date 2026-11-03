//! Favoritos e rejeitados do fotógrafo (spec TL, regras 6 a 8). Um **desenho** é identificado
//! pela família com a estrutura dela, pelo respiro e pela sequência de orientações das fotos;
//! o tamanho do álbum não entra (ADR 0003).

use std::collections::BTreeSet;

use lamina_core::{Orientation, PageSide};

use crate::layouts::{
    Align, Arrangement, Breathing, Family, FullBleed, LayoutCandidate, PageLayout, Scope,
    SupportSide,
};

/// Pontos que um favorito ganha na escolha do Auto Build (regra 8).
pub const FAVORITE_BONUS: f64 = 10.0;

/// Os desenhos marcados pelo fotógrafo, pelo identificador de [`design_key`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preferences {
    pub favorites: BTreeSet<String>,
    pub rejected: BTreeSet<String>,
}

impl Preferences {
    pub fn is_empty(&self) -> bool {
        self.favorites.is_empty() && self.rejected.is_empty()
    }

    /// Marca ou desmarca como favorito. Favoritar tira dos rejeitados.
    pub fn toggle_favorite(&mut self, key: &str) {
        if !self.favorites.remove(key) {
            self.rejected.remove(key);
            self.favorites.insert(key.to_owned());
        }
    }

    /// Rejeita (e tira dos favoritos).
    pub fn reject(&mut self, key: &str) {
        self.favorites.remove(key);
        self.rejected.insert(key.to_owned());
    }
}

/// Lista de sugestões com as preferências aplicadas.
#[derive(Debug, Clone, PartialEq)]
pub struct Preferred {
    pub list: Vec<LayoutCandidate>,
    /// Cada posição da lista é favorita?
    pub favorite: Vec<bool>,
    /// Todas as opções estavam rejeitadas: a lista veio inteira (regra 7).
    pub all_rejected: bool,
}

/// Identificador do desenho: "HHV|paginas:E=L2;D=L1|m0c".
pub fn design_key(candidate: &LayoutCandidate, aspects: &[f64]) -> String {
    design_key_of(
        &candidate.family,
        candidate.breathing,
        &candidate.order,
        aspects,
    )
}

/// O mesmo identificador, pelas partes de um candidato ainda não montado (spec AL, seção 11).
pub(crate) fn design_key_of(
    family: &Family,
    b: Breathing,
    order: &[usize],
    aspects: &[f64],
) -> String {
    // O modelo tem chave pelo nome: renomear leva as marcas junto (spec MD, regra 5).
    if let Family::Model { name, mirrored } = family {
        return model_key(name, *mirrored);
    }
    let orientations: String = order
        .iter()
        .map(
            |&i| match aspects.get(i).map(|a| Orientation::from_aspect_ratio(*a)) {
                Some(Orientation::Portrait) => 'V',
                Some(Orientation::Square) => 'Q',
                _ => 'H',
            },
        )
        .collect();
    let align = match b.align {
        Align::Center => 'c',
        Align::Start => 's',
        Align::End => 'e',
    };
    format!(
        "{orientations}|{}|m{}{align}",
        family_text(family),
        b.margin_level
    )
}

/// Chave do desenho de um modelo: "modelo:<nome>", com ":e" no espelho.
pub fn model_key(name: &str, mirrored: bool) -> String {
    let e = if mirrored { ":e" } else { "" };
    format!("modelo:{name}{e}")
}

fn layout_text(layout: &PageLayout) -> String {
    let kind = match layout.arrangement {
        Arrangement::Rows => 'L',
        Arrangement::Columns => 'C',
    };
    let groups: Vec<String> = layout.groups.iter().map(ToString::to_string).collect();
    format!("{kind}{}", groups.join("-"))
}

fn optional(layout: Option<&PageLayout>) -> String {
    layout.map_or_else(|| "-".to_owned(), layout_text)
}

fn side_text(side: PageSide) -> &'static str {
    match side {
        PageSide::Left => "E",
        PageSide::Right => "D",
    }
}

fn scope_text(scope: Scope) -> &'static str {
    match scope {
        Scope::Page(side) => side_text(side),
        Scope::Spread => "L",
    }
}

fn support_text(support: SupportSide) -> &'static str {
    match support {
        SupportSide::Left => "esquerda",
        SupportSide::Right => "direita",
        SupportSide::Below => "embaixo",
    }
}

fn full_bleed_text(format: &FullBleed) -> String {
    match format {
        FullBleed::Pages { left, right } => format!(
            "semborda:paginas:E={};D={}",
            optional(left.as_ref()),
            optional(right.as_ref())
        ),
        FullBleed::Spread(layout) => format!("semborda:lamina:{}", layout_text(layout)),
        FullBleed::Hero { support, layout } => format!(
            "semborda:destaque:{}:{}",
            support_text(*support),
            layout_text(layout)
        ),
    }
}

fn family_text(family: &Family) -> String {
    match family {
        Family::Across => "atravessada".to_owned(),
        Family::PerPage { left, right } => format!(
            "paginas:E={};D={}",
            optional(left.as_ref()),
            optional(right.as_ref())
        ),
        Family::WholeSpread(layout) => format!("lamina:{}", layout_text(layout)),
        Family::Hero {
            scope,
            support,
            layout,
        } => {
            let support = support_text(*support);
            let scope = scope_text(*scope);
            format!("destaque:{scope}:{support}:{}", layout_text(layout))
        }
        Family::Mirrored(layout) => format!("espelhado:{}", layout_text(layout)),
        Family::BleedPage { side, rest } => {
            format!("sangrada:{}:{}", side_text(*side), optional(rest.as_ref()))
        }
        Family::Mosaic { scope, layout } => {
            format!("mosaico:{}:{}", scope_text(*scope), layout_text(layout))
        }
        Family::FullBleed(format) => full_bleed_text(format),
        Family::Model { name, mirrored } => model_key(name, *mirrored),
    }
}

/// Favoritos no topo (pela nota), depois a ordem da variedade; rejeitados fora. Se todos forem
/// rejeitados, a lista vem inteira com `all_rejected` (regra 7).
pub fn apply_preferences(
    list: Vec<LayoutCandidate>,
    aspects: &[f64],
    prefs: &Preferences,
) -> Preferred {
    let keys: Vec<String> = list.iter().map(|c| design_key(c, aspects)).collect();
    let all_rejected = !list.is_empty() && keys.iter().all(|k| prefs.rejected.contains(k));
    let mut tagged: Vec<(bool, LayoutCandidate)> = list
        .into_iter()
        .zip(&keys)
        .filter(|(_, k)| all_rejected || !prefs.rejected.contains(*k))
        .map(|(c, k)| (prefs.favorites.contains(k), c))
        .collect();
    // Ordenação estável: entre favoritos manda a nota; o resto mantém a ordem da variedade.
    tagged.sort_by(|(fa, a), (fb, b)| match (fa, fb) {
        (true, true) => b.score.total.total_cmp(&a.score.total),
        _ => fb.cmp(fa),
    });
    let favorite = tagged.iter().map(|(f, _)| *f).collect();
    Preferred {
        list: tagged.into_iter().map(|(_, c)| c).collect(),
        favorite,
        all_rejected,
    }
}

#[cfg(test)]
#[path = "prefs_tests.rs"]
mod tests;
