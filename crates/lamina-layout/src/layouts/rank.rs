//! Candidatos finais: coordenadas relativas, recorte, nota, regra da lombada, ordem e remoção
//! de repetidos (TP, regra 6; VL, regras 5 e 6).
//!
//! Para ser rápido, a ordem é decidida numa versão leve de cada candidato (nota, família e
//! chave); só os que entram na lista são montados por inteiro.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use lamina_core::{PageSide, Rect, RelRect};
use rayon::prelude::*;

use super::area::{Ctx, EPS, Raw};
use super::fill::crop_of;
use super::rotation::rotate;
use super::score::score;
use super::{FamilyKind, LayoutCandidate, Score};
use crate::learned::Learned;
use crate::prefs::design_key_of;

/// Largura mínima de um quadro sobre a lombada, em fração da largura da página.
const GUTTER_MIN_SHARE: f64 = 0.25;
/// Parte mínima de um quadro sobre a lombada que fica de cada lado dela.
const GUTTER_MIN_SIDE: f64 = 0.1;

/// Critério de ordem da lista.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// TP: do maior para o menor aproveitamento.
    Coverage,
    /// VL: nota, com rodízio de famílias nas primeiras posições.
    Score,
}

/// Uma sequência de fotos (índices originais) e os candidatos brutos montados nela.
pub struct Sequence {
    pub order: Vec<usize>,
    pub raws: Vec<Raw>,
}

/// O que fica de fora da lista: repetidos de outra lista, e o tamanho máximo.
pub struct Limits<'a> {
    pub max: usize,
    pub exclude: &'a HashSet<u64>,
    /// O que o fotógrafo mais escolhe; só muda a ordem (spec AL, regra 1).
    pub learned: &'a Learned,
}

/// Versão leve de um candidato.
pub struct Entry {
    pub value: f64,
    pub kind: FamilyKind,
    seq: usize,
    raw: usize,
    key: u64,
}

/// Monta, ordena e limpa os candidatos de todas as sequências.
pub fn finish(
    ctx: &Ctx,
    aspects: &[f64],
    sequences: &[Sequence],
    sort: Sort,
    limits: &Limits,
) -> Vec<LayoutCandidate> {
    let mut entries = entries(ctx, aspects, sequences, sort, limits.learned);
    entries.sort_by(|a, b| {
        b.value
            .total_cmp(&a.value)
            .then(a.kind.cmp(&b.kind))
            .then((a.seq, a.raw).cmp(&(b.seq, b.raw)))
    });
    let mut seen = HashSet::new();
    entries.retain(|e| !limits.exclude.contains(&e.key) && seen.insert(e.key));
    if sort == Sort::Score {
        entries = rotate(entries, |e| &sequences[e.seq].raws[e.raw].family);
    }
    entries.truncate(limits.max);
    entries
        .iter()
        .filter_map(|e| {
            let sequence = &sequences[e.seq];
            let raw = &sequence.raws[e.raw];
            let m = measure(ctx, aspects, &sequence.order, raw)?;
            Some(LayoutCandidate {
                family: raw.family.clone(),
                breathing: raw.breathing,
                frames: m.frames,
                order: sequence.order.clone(),
                crop: m.crop,
                coverage: m.coverage,
                score: m.score,
                model: None,
            })
        })
        .collect()
}

/// Versão leve de todos os candidatos, calculada em paralelo.
fn entries(
    ctx: &Ctx,
    aspects: &[f64],
    sequences: &[Sequence],
    sort: Sort,
    learned: &Learned,
) -> Vec<Entry> {
    // Sem nada aprendido, as chaves nem são montadas (spec AL, regra 3).
    let bonus = |order: &[usize], r: &Raw| {
        if learned.is_empty() {
            return 0.0;
        }
        learned.bonus(&design_key_of(&r.family, r.breathing, order, aspects))
    };
    let pairs: Vec<(usize, usize)> = sequences
        .iter()
        .enumerate()
        .flat_map(|(seq, s)| (0..s.raws.len()).map(move |raw| (seq, raw)))
        .collect();
    pairs
        .into_par_iter()
        .filter_map(|(seq, raw)| {
            let (sequence, r) = (&sequences[seq], &sequences[seq].raws[raw]);
            let m = measure(ctx, aspects, &sequence.order, r)?;
            let value = match sort {
                Sort::Coverage => m.coverage,
                // O bônus só ordena; a nota da opção continua pura (spec AL, regra 1).
                Sort::Score => m.score.total + bonus(&sequence.order, r),
            };
            Some(Entry {
                value: (value * 1e6).round() / 1e6,
                kind: r.family.kind(),
                seq,
                raw,
                key: key(&m.frames),
            })
        })
        .collect()
}

/// Quadros relativos (na ordem original das fotos), recorte e nota de um candidato bruto.
struct Measured {
    frames: Vec<RelRect>,
    crop: Vec<f64>,
    coverage: f64,
    score: Score,
}

/// `None` se um quadro sai do corte ou se um quadro estreito cruza a lombada.
fn measure(ctx: &Ctx, aspects: &[f64], order: &[usize], raw: &Raw) -> Option<Measured> {
    // Toda foto precisa de um quadro; uma família com quadros a menos é descartada.
    if raw.rects.len() != order.len() || !gutter_ok(ctx, &raw.rects) {
        return None;
    }
    let trim = ctx.spec.trim_rect();
    let n = order.len();
    let mut frames = vec![RelRect::full(); n];
    let mut crop = vec![0.0; n];
    for (rect, &photo) in raw.rects.iter().zip(order) {
        frames[photo] = relative(&trim, rect)?;
        crop[photo] = crop_of(rect.width.get() / rect.height.get(), aspects[photo]);
    }
    let useful: f64 = ctx
        .sides()
        .into_iter()
        .map(|s| area(&ctx.spec.safe_rect(s)))
        .sum();
    let coverage = (raw.rects.iter().map(area).sum::<f64>() / useful).min(1.0);
    let cut: f64 = ctx
        .sides()
        .into_iter()
        .map(|s| area(&ctx.spec.page_rect(s)))
        .sum();
    let score = score(ctx, &raw.rects, &crop, cut);
    Some(Measured {
        frames,
        crop,
        coverage,
        score,
    })
}

fn area(r: &Rect) -> f64 {
    r.width.get() * r.height.get()
}

/// Um quadro que cruza a lombada precisa de pelo menos 25% da largura da página e de deixar ao
/// menos 10% da própria largura de cada lado da dobra: nada de filete da foto na outra página
/// (spec VL, regra 5; auditoria do Marco 1, M3).
pub(super) fn gutter_ok(ctx: &Ctx, rects: &[Rect]) -> bool {
    let gutter = ctx.gutter();
    let min = ctx.spec.page_rect(PageSide::Left).width.get() * GUTTER_MIN_SHARE;
    rects.iter().all(|r| {
        let (left, right) = (gutter - r.x.get(), r.right().get() - gutter);
        let crosses = left > EPS && right > EPS;
        let side_min = r.width.get() * GUTTER_MIN_SIDE;
        !crosses || (r.width.get() >= min && left >= side_min && right >= side_min)
    })
}

fn relative(trim: &Rect, r: &Rect) -> Option<RelRect> {
    RelRect::new(
        (r.x - trim.x).get() / trim.width.get(),
        (r.y - trim.y).get() / trim.height.get(),
        r.width.get() / trim.width.get(),
        r.height.get() / trim.height.get(),
    )
    .ok()
}

/// Chave de repetição: a posição de cada foto, arredondada.
pub fn key(frames: &[RelRect]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for f in frames {
        for v in [f.x(), f.y(), f.width(), f.height()] {
            (v * 1e7).round().to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
}
