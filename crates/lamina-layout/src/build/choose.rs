//! Escolha do layout de uma lâmina (spec AB, regra 3): ritmo, variedade e nota.

use crate::layouts::{Family, FamilyKind, LayoutCandidate};
use crate::plan::{SpreadPlan, SpreadRole};
use crate::prefs::FAVORITE_BONUS;

use super::Reuse;
use super::window::Boost;

/// A escolhida (posição na lista) e o motivo.
pub struct Pick {
    pub index: usize,
    pub reason: String,
}

/// Escolhe entre as sugestões `list` da lâmina, sabendo o que já foi usado no álbum (`used`, na
/// ordem das lâminas) e o que cada uma ganha do fotógrafo (`boost`, mesma ordem de `list`).
pub fn pick(
    list: &[LayoutCandidate],
    boost: &[Boost],
    spread: &SpreadPlan,
    used: &[Family],
    reuse: Reuse,
) -> Pick {
    // Ritmo: num destaque, só concorrem as famílias de foto grande (se houver alguma).
    let rhythm = wants_big(spread) && list.iter().any(|c| is_big(&c.family));
    // Um modelo do fotógrafo concorre sempre: é um desenho escolhido por ele (spec MD).
    let eligible =
        |c: &LayoutCandidate| !rhythm || is_big(&c.family) || c.family.kind() == FamilyKind::Model;
    let repeats_previous = |c: &LayoutCandidate| {
        reuse != Reuse::High && used.last().is_some_and(|p| p.same_outline(&c.family))
    };
    let is_favorite = |i: usize| boost.get(i).is_some_and(|b| b.favorite);
    let learned = |i: usize| boost.get(i).map_or(0.0, |b| b.learned);
    // O favorito fica com os pontos dele, sem somar o aprendido (spec AL, regra 4).
    let value = |i: usize, c: &LayoutCandidate| {
        let bonus = if is_favorite(i) {
            FAVORITE_BONUS
        } else {
            learned(i)
        };
        adjusted(c, uses(used, &c.family), reuse) + bonus
    };
    // Depois de duas lâminas até a borda, uma mais calma (spec SB, regra 8).
    let rest = needs_rest(used, reuse);
    let tiring = |c: &LayoutCandidate| rest && reaches_edge(&c.family);
    let free = best(
        list,
        |c| eligible(c) && !repeats_previous(c) && !tiring(c),
        value,
    );
    let overall = best(list, eligible, value);
    // Sem opção livre (todas repetem a anterior), fica a melhor de todas.
    let index = free.or(overall).unwrap_or(0);
    let avoided = overall.is_some_and(|o| o != index && repeats_previous(&list[o]));
    let rested = overall.is_some_and(|o| o != index && tiring(&list[o]));
    let mut parts = list.get(index).map_or_else(Vec::new, |c| {
        reasons(c, rhythm, uses(used, &c.family), avoided)
    });
    if rested {
        parts.push("respiro depois de duas lâminas até a borda".to_owned());
    }
    if is_favorite(index) {
        parts.insert(0, "desenho favorito".to_owned());
    } else if learned(index) > 0.0 {
        parts.insert(0, "desenho que você costuma usar".to_owned());
    }
    if parts.is_empty() {
        parts.push(if index == 0 {
            "melhor nota da lista".to_owned()
        } else {
            "melhor nota entre os desenhos menos usados no álbum".to_owned()
        });
    }
    Pick {
        index,
        reason: parts.join("; "),
    }
}

/// Posição da maior nota ajustada entre as sugestões aceitas; empate fica com a primeira.
fn best(
    list: &[LayoutCandidate],
    accept: impl Fn(&LayoutCandidate) -> bool,
    value: impl Fn(usize, &LayoutCandidate) -> f64,
) -> Option<usize> {
    let mut best: Option<(f64, usize)> = None;
    for (i, c) in list.iter().enumerate().filter(|(_, c)| accept(c)) {
        let v = value(i, c);
        if best.is_none_or(|(b, _)| v > b + 1e-9) {
            best = Some((v, i));
        }
    }
    best.map(|(_, i)| i)
}

/// Lâmina de destaque, ou única com até 2 fotos.
fn wants_big(spread: &SpreadPlan) -> bool {
    spread.role == SpreadRole::Highlight
        || (spread.role == SpreadRole::Single && spread.photos.len() <= 2)
}

fn is_big(family: &Family) -> bool {
    matches!(
        family.kind(),
        FamilyKind::Across
            | FamilyKind::BleedPage
            | FamilyKind::FullBleed
            | FamilyKind::Hero
            | FamilyKind::WholeSpread
    )
}

/// A foto chega ao corte em pelo menos uma página inteira?
fn reaches_edge(family: &Family) -> bool {
    matches!(
        family.kind(),
        FamilyKind::Across | FamilyKind::BleedPage | FamilyKind::FullBleed
    )
}

/// As duas últimas lâminas foram até a borda (salvo com a reutilização alta)?
fn needs_rest(used: &[Family], reuse: Reuse) -> bool {
    reuse != Reuse::High && used.len() >= 2 && used[used.len() - 2..].iter().all(reaches_edge)
}

/// Quantas vezes o mesmo desenho geral já foi usado no álbum.
fn uses(used: &[Family], family: &Family) -> usize {
    used.iter().filter(|u| u.same_outline(family)).count()
}

fn penalty(reuse: Reuse) -> f64 {
    match reuse {
        Reuse::Low => 15.0,
        Reuse::Medium => 8.0,
        Reuse::High => 0.0,
    }
}

/// Nota menos a penalidade de repetição do desenho geral.
fn adjusted(c: &LayoutCandidate, uses: usize, reuse: Reuse) -> f64 {
    let count = f64::from(u32::try_from(uses).unwrap_or(u32::MAX));
    c.score.total - penalty(reuse) * count
}

fn reasons(c: &LayoutCandidate, rhythm: bool, uses: usize, avoided_previous: bool) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    if rhythm && is_big(&c.family) {
        parts.push("destaque: foto grande".to_owned());
    }
    if avoided_previous {
        parts.push("evita repetir o desenho da lâmina anterior".to_owned());
    }
    if uses > 0 {
        parts.push(format!("desenho já usado {uses} vez(es) no álbum"));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::{Arrangement, Breathing, PageLayout, Scope, Score, SupportSide};

    fn rows(groups: &[usize]) -> PageLayout {
        PageLayout {
            arrangement: Arrangement::Rows,
            groups: groups.to_vec(),
        }
    }

    fn per_page() -> Family {
        Family::PerPage {
            left: Some(rows(&[1])),
            right: Some(rows(&[1])),
        }
    }

    fn hero() -> Family {
        Family::Hero {
            scope: Scope::Spread,
            support: SupportSide::Right,
            layout: rows(&[1]),
        }
    }

    fn whole() -> Family {
        Family::WholeSpread(rows(&[2]))
    }

    fn option(family: Family, total: f64) -> LayoutCandidate {
        LayoutCandidate {
            family,
            breathing: Breathing::default(),
            frames: Vec::new(),
            order: Vec::new(),
            crop: Vec::new(),
            coverage: 0.0,
            score: Score {
                total,
                ..Score::default()
            },
            model: None,
        }
    }

    fn spread(role: SpreadRole) -> SpreadPlan {
        SpreadPlan {
            moment: 0,
            role,
            photos: Vec::new(),
        }
    }

    fn learned(value: f64) -> Boost {
        Boost {
            favorite: false,
            learned: value,
        }
    }

    #[test]
    fn al_b7_o_bonus_muda_a_escolha_e_o_motivo() {
        let list = [option(per_page(), 90.0), option(whole(), 85.0)];
        let dense = spread(SpreadRole::Dense);
        let boost = [learned(0.0), learned(6.4)];
        let p = pick(&list, &boost, &dense, &[], Reuse::Medium);
        assert_eq!(p.index, 1);
        assert!(
            p.reason.contains("desenho que você costuma usar"),
            "{}",
            p.reason
        );
        // Mais de 6,4 pontos atrás, ele não passa.
        let far = [option(per_page(), 90.0), option(whole(), 80.0)];
        assert_eq!(pick(&far, &boost, &dense, &[], Reuse::Medium).index, 0);
        // Repetir a lâmina anterior ainda pesa mais que o bônus.
        let used = [whole()];
        assert_eq!(pick(&list, &boost, &dense, &used, Reuse::Medium).index, 0);
    }

    #[test]
    fn al_b8_favorito_nao_soma_com_o_aprendizado() {
        let list = [option(per_page(), 80.0), option(whole(), 93.0)];
        let both = Boost {
            favorite: true,
            learned: 6.4,
        };
        let boost = [both, learned(0.0)];
        // 80 + 10 fica atrás de 93; com o bônus somado, passaria.
        let dense = spread(SpreadRole::Dense);
        assert_eq!(pick(&list, &boost, &dense, &[], Reuse::Medium).index, 1);
        let near = [option(per_page(), 80.0), option(whole(), 89.0)];
        let p = pick(&near, &boost, &dense, &[], Reuse::Medium);
        assert_eq!(p.index, 0);
        assert!(p.reason.contains("desenho favorito"), "{}", p.reason);
        assert!(!p.reason.contains("costuma usar"), "{}", p.reason);
    }

    #[test]
    fn empate_fica_com_a_primeira_da_lista() {
        let list = [option(per_page(), 80.0), option(hero(), 80.0)];
        let p = pick(&list, &[], &spread(SpreadRole::Dense), &[], Reuse::Medium);
        assert_eq!(p.index, 0);
        assert_eq!(p.reason, "melhor nota da lista");
    }

    #[test]
    fn destaque_so_aceita_foto_grande() {
        let list = [option(per_page(), 90.0), option(hero(), 70.0)];
        let p = pick(
            &list,
            &[],
            &spread(SpreadRole::Highlight),
            &[],
            Reuse::Medium,
        );
        assert_eq!(p.index, 1);
        assert!(p.reason.contains("destaque: foto grande"), "{}", p.reason);
    }

    #[test]
    fn destaque_sem_foto_grande_fica_com_a_melhor_nota() {
        let list = [
            option(per_page(), 90.0),
            option(Family::Mirrored(rows(&[1])), 70.0),
        ];
        let p = pick(
            &list,
            &[],
            &spread(SpreadRole::Highlight),
            &[],
            Reuse::Medium,
        );
        assert_eq!(p.index, 0);
    }

    #[test]
    fn vizinha_nao_repete_salvo_no_nivel_alto() {
        let list = [option(per_page(), 90.0), option(whole(), 70.0)];
        let used = [per_page()];
        let medium = pick(&list, &[], &spread(SpreadRole::Dense), &used, Reuse::Medium);
        assert_eq!(medium.index, 1);
        assert!(medium.reason.contains("evita repetir"), "{}", medium.reason);
        let high = pick(&list, &[], &spread(SpreadRole::Dense), &used, Reuse::High);
        assert_eq!(high.index, 0);
    }

    #[test]
    fn todas_repetem_a_anterior_fica_a_melhor() {
        let list = [option(per_page(), 70.0), option(per_page(), 90.0)];
        let p = pick(
            &list,
            &[],
            &spread(SpreadRole::Dense),
            &[per_page()],
            Reuse::Low,
        );
        assert_eq!(p.index, 1);
        assert!(p.reason.contains("já usado 1 vez"), "{}", p.reason);
    }

    #[test]
    fn ritmo_vem_antes_da_vizinha() {
        // O único de foto grande repete a anterior; mesmo assim o destaque fica com ele.
        let list = [option(per_page(), 90.0), option(hero(), 70.0)];
        let p = pick(
            &list,
            &[],
            &spread(SpreadRole::Highlight),
            &[hero()],
            Reuse::Medium,
        );
        assert_eq!(p.index, 1);
    }

    #[test]
    fn penalidade_muda_o_vencedor_entre_os_niveis() {
        // Hero já usado duas vezes (não na anterior): −16 no médio, −30 no baixo.
        let list = [option(hero(), 80.0), option(whole(), 66.0)];
        let used = [hero(), hero(), per_page()];
        let dense = spread(SpreadRole::Dense);
        assert_eq!(pick(&list, &[], &dense, &used, Reuse::Medium).index, 1);
        assert_eq!(pick(&list, &[], &dense, &used, Reuse::High).index, 0);
        let near = [option(hero(), 80.0), option(whole(), 60.0)];
        assert_eq!(pick(&near, &[], &dense, &used, Reuse::Medium).index, 0);
        assert_eq!(pick(&near, &[], &dense, &used, Reuse::Low).index, 1);
    }
}
