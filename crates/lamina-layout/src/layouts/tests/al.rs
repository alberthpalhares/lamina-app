//! Cenários AL-B1 a AL-B4, AL-B6 e AL-B11 da spec `specs/aprendizado-de-layouts.md`: a ordem das
//! listas.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{ActivePages, AlbumSpec, Border, DesignMode, Dpi, Mm, RelRect};

use super::rotation::rotation_slot;
use super::*;
use crate::learned::{Learned, MAX_LEARNED_BONUS, learned_bonus};
use crate::prefs::design_key;

const H: f64 = 1.5;
const V: f64 = 2.0 / 3.0;
const BOTH: ActivePages = ActivePages {
    left: true,
    right: true,
};

fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
        .with_mode(DesignMode::Spread)
}

/// A lista inteira: o teto de 200 não pode esconder uma opção do teste.
fn everything() -> LayoutOptions {
    LayoutOptions {
        max_candidates: usize::MAX,
        ..LayoutOptions::default()
    }
}

fn plain(aspects: &[f64]) -> Vec<LayoutCandidate> {
    layout_options(&spec(), BOTH, aspects, &everything())
        .unwrap()
        .in_order
}

fn taught(aspects: &[f64], learned: &Learned) -> Vec<LayoutCandidate> {
    layout_options_learned(&spec(), BOTH, aspects, &everything(), learned)
        .unwrap()
        .in_order
}

/// A posição da opção com estes quadros.
fn position(list: &[LayoutCandidate], frames: &[RelRect]) -> usize {
    list.iter().position(|c| c.frames == frames).unwrap()
}

#[test]
fn al_b1_sem_nada_aprendido_nada_muda() {
    let cases: [Vec<f64>; 6] = [
        vec![H],
        vec![H, V],
        vec![H; 4],
        vec![H, H, V, H],
        vec![H; 8],
        vec![H, V, H, H, V, H, H, H, V, H, H, H],
    ];
    let unknown = Learned::from_weights([("nenhum desenho".to_owned(), 50_000)]);
    let options = LayoutOptions::default();
    for aspects in &cases {
        let base = layout_options(&spec(), BOTH, aspects, &options).unwrap();
        for learned in [&Learned::default(), &unknown] {
            let got = layout_options_learned(&spec(), BOTH, aspects, &options, learned).unwrap();
            assert_eq!(got, base, "{aspects:?}");
        }
    }
}

#[test]
fn al_b2_o_desenho_usado_sobe_e_a_nota_nao_muda() {
    let aspects = [H; 4];
    let base = plain(&aspects);
    // Depois das 12 do rodízio, a lista segue a nota: um par a menos de 4 pontos.
    let close = |&(a, b): &(usize, usize)| {
        let gap = base[a].score.total - base[b].score.total;
        gap > 0.1 && gap < 3.9 && design_key(&base[a], &aspects) != design_key(&base[b], &aspects)
    };
    let (ahead, behind) = (12..base.len())
        .flat_map(|a| (a + 1..base.len()).map(move |b| (a, b)))
        .find(close)
        .expect("um par de opções com notas próximas");
    let key = design_key(&base[behind], &aspects);
    let learned = Learned::from_weights([(key, 3000)]);
    let list = taught(&aspects, &learned);
    assert!(
        position(&list, &base[behind].frames) < position(&list, &base[ahead].frames),
        "o desenho usado devia passar o outro"
    );
    let totals: HashMap<u64, f64> = base
        .iter()
        .map(|c| (rank::key(&c.frames), c.score.total))
        .collect();
    assert_eq!(list.len(), base.len());
    for c in &list {
        assert_eq!(c.score.total, totals[&rank::key(&c.frames)], "a nota mudou");
    }
}

#[test]
fn al_b3_o_teto_do_bonus() {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(close(learned_bonus(1000), 2.0));
    assert!(close(learned_bonus(3000), 4.0));
    assert!(close(learned_bonus(12000), 6.4));
    assert!(close(learned_bonus(0), 0.0));
    let most = learned_bonus(u32::MAX);
    assert!(most < MAX_LEARNED_BONUS, "{most}");
    let key = Learned::from_weights([("desenho".to_owned(), u32::MAX)]);
    assert!(key.bonus("desenho") < MAX_LEARNED_BONUS);
    assert!(close(key.bonus("outro"), 0.0));
}

#[test]
fn al_b4_o_rodizio_continua_valendo() {
    let aspects = [H, H, V, H, H, V];
    let same_family = plain(&aspects)
        .iter()
        .filter(|c| c.family.kind() == FamilyKind::PerPage)
        .map(|c| (design_key(c, &aspects), 200_000))
        .collect::<Vec<_>>();
    assert!(same_family.len() > 12, "{}", same_family.len());
    let list = taught(&aspects, &Learned::from_weights(same_family));
    let first = &list[..12];
    for (i, a) in first.iter().enumerate() {
        for b in &first[i + 1..] {
            assert!(!a.family.same_outline(&b.family), "desenho geral repetido");
        }
    }
    // Uma família só repete depois de todas as outras: nenhuma estreia depois de uma repetição.
    let mut seen = HashSet::new();
    let mut repeated = false;
    for c in first {
        let slot = rotation_slot(&c.family);
        let fresh = seen.insert(slot);
        assert!(!(repeated && fresh), "{slot:?} estreou depois de repetir");
        repeated |= !fresh;
    }
    assert!(seen.len() > 1, "só uma família nas primeiras posições");
}

fn model(name: &str) -> SpreadModel {
    let slot = |x: f64| ModelSlot {
        rect: RelRect::new(x, 0.1, 0.4, 0.8).unwrap(),
        orientation: None,
        border: Border::default(),
        radius: Mm(0.0),
    };
    SpreadModel {
        name: name.to_owned(),
        slots: vec![slot(0.05), slot(0.55)],
        texts: Vec::new(),
        left: true,
        right: true,
    }
}

#[test]
fn al_b6_modelo_nao_ganha_bonus_e_aprendido_de_nota_menor_nao_passa() {
    let aspects = [H, H];
    let at = (BOTH, DEFAULT_GAP);
    let models = [model("Casamento")];
    let mut base = layout_options(&spec(), BOTH, &aspects, &everything()).unwrap();
    add_models(&mut base, &spec(), at, &aspects, &models).unwrap();
    let is_model = |c: &LayoutCandidate| c.family.kind() == FamilyKind::Model;
    let of_model = |lists: &LayoutLists| {
        let list = &lists.in_order;
        let at = list.iter().position(is_model).unwrap();
        (at, list[at].score.total)
    };
    let (_, total) = of_model(&base);
    // Todas as opções com peso, inclusive uma chave com o nome do modelo.
    let mut weights: Vec<(String, u32)> = base
        .in_order
        .iter()
        .filter(|c| !is_model(c))
        .map(|c| (design_key(c, &aspects), 200_000))
        .collect();
    weights.push((crate::prefs::model_key("Casamento", false), 200_000));
    let learned = Learned::from_weights(weights);
    let mut lists =
        layout_options_learned(&spec(), BOTH, &aspects, &everything(), &learned).unwrap();
    add_models(&mut lists, &spec(), at, &aspects, &models).unwrap();
    let (place, now) = of_model(&lists);
    assert_eq!(now, total, "a nota do modelo não muda");
    for c in &lists.in_order[..place] {
        assert!(c.score.total >= now, "{:?} passou o modelo", c.family);
    }
}

/// A menor de até `TRIES` medidas, parando quando cabe em `goal`.
fn fastest(aspects: &[f64], learned: &Learned, goal: Duration) -> Duration {
    let (s, options) = (spec(), LayoutOptions::default());
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let start = Instant::now();
        layout_options_learned(&s, BOTH, aspects, &options, learned).unwrap();
        best = best.min(start.elapsed());
        if best < goal {
            break;
        }
    }
    best
}

#[test]
fn al_b11_rapido_com_o_arquivo_cheio() {
    let full =
        Learned::from_weights((0..500).map(|i| (format!("HHHH|paginas:E=L{i};D=L{i}|m0c"), 5000)));
    let twelve = [H, H, V, H, H, V, H, H, V, H, H, H];
    let got = fastest(&twelve, &full, budget(200));
    assert!(got < budget(200), "12 fotos: {got:?}");
    let many: Vec<f64> = twelve.iter().chain(&twelve).copied().collect();
    let got = fastest(&many, &full, budget(1000));
    assert!(got < budget(1000), "24 fotos: {got:?}");
}
