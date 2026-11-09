//! Cenários da spec `specs/reforma-dos-modelos.md` (RM): corte das variantes deslocadas e a
//! ordem por preenchimento nas listas da janela, com a linha de comando imune.

use std::collections::HashSet;

use lamina_core::{ActivePages, DesignMode, Dpi, Mm};

use super::*;
use crate::prefs::Preferences;

const H: f64 = 1.5;
const BOTH: ActivePages = ActivePages {
    left: true,
    right: true,
};

fn spec(mode: DesignMode) -> lamina_core::AlbumSpec {
    lamina_core::AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
        .with_mode(mode)
}

fn options(window: bool) -> LayoutOptions {
    LayoutOptions {
        window,
        ..LayoutOptions::default()
    }
}

/// A "chave de deslocamento": família + divisão + respiro (nível de margem) + sequência, sem o
/// alinhamento. Duas opções com a mesma chave e alinhamentos diferentes são variantes deslocadas.
fn displacement_key(c: &LayoutCandidate) -> (String, u8) {
    (
        crate::prefs::design_key_of(&c.family, c.breathing, &c.order, &[]),
        c.breathing.margin_level,
    )
}

#[test]
fn rm_b1_sem_variantes_deslocadas_nos_dois_eixos() {
    let l = layout_options(
        &spec(DesignMode::Spread),
        BOTH,
        &[H, H, H, H],
        &options(true),
    )
    .unwrap();
    for list in [&l.in_order, &l.reordered] {
        let mut seen = HashSet::new();
        for c in list {
            let (key, margin) = displacement_key(c);
            assert!(
                seen.insert((key, margin)),
                "variante deslocada na lista da janela: {:?} {:?}",
                c.family,
                c.breathing
            );
        }
    }
}

#[test]
fn rm_b1b_as_listas_da_linha_de_comando_continuam_com_as_variantes() {
    let l = layout_options(
        &spec(DesignMode::Spread),
        BOTH,
        &[H, H, H, H],
        &options(false),
    )
    .unwrap();
    // Sem o flag da janela, a geração continua como hoje (Start/End existem).
    assert!(l.in_order.len() > 1);
    let centers = l
        .in_order
        .iter()
        .any(|c| c.breathing.align != Align::Center);
    assert!(centers, "a linha de comando mantém os alinhamentos");
}

#[test]
fn rm_b2_preenchimento_na_frente() {
    let l = layout_options(
        &spec(DesignMode::Spread),
        BOTH,
        &[H, H, H, H],
        &options(true),
    )
    .unwrap();
    let prefs = Preferences::default();
    let preferred = crate::prefs::apply_preferences(l.in_order, &[H, H, H, H], &prefs);
    // Depois da 12ª (rodízio), a ordem é pelo preenchimento (coverage), não pela nota.
    let rest = &preferred.list[12..];
    for w in rest.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        assert!(
            b.coverage <= a.coverage + 1e-9,
            "a ordem cai: {:?} (cov {}) antes de {:?} (cov {})",
            a.family,
            a.coverage,
            b.family,
            b.coverage
        );
    }
}

#[test]
fn rm_b3_favorito_continua_no_topo() {
    let l = layout_options(
        &spec(DesignMode::Spread),
        BOTH,
        &[H, H, H, H],
        &options(true),
    )
    .unwrap();
    let mut prefs = Preferences::default();
    // O último da lista fica favorito: tem que subir para o topo.
    if let Some(last) = l.in_order.last() {
        let key = crate::prefs::design_key(last, &[H, H, H, H]);
        prefs.toggle_favorite(&key);
    }
    let preferred = crate::prefs::apply_preferences(l.in_order, &[H, H, H, H], &prefs);
    assert!(preferred.favorite[0], "o favorito vem primeiro");
}

#[test]
fn rm_b11_contagens_minimas_continam() {
    // 1 vertical e 4 horizontais: o mínimo da VL para 4 horizontais é 100 (nas duas listas).
    let l = layout_options(
        &spec(DesignMode::Spread),
        BOTH,
        &[H, H, H, H],
        &options(true),
    )
    .unwrap();
    let total = l.in_order.len() + l.reordered.len();
    assert!(
        total >= 100,
        "4 horizontais: {total} opções (mínimo 100 da VL, regra 8)"
    );
}
