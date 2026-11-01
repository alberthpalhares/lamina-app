//! Só o que mudou é atualizado (spec MV, regra 3): cada lista da janela (faixa de lâminas,
//! navegador, linha do tempo) vira pares (chave, valor), e [`row_ops`] diz quais linhas trocar,
//! inserir ou remover para ir da lista velha à nova. O painel de layouts ganha uma chave que não
//! muda quando só a opção em uso muda.

use std::collections::HashSet;
use std::iter::once;

use lamina_core::{ActivePages, FrameContent, PageSide, PhotoId, Project, Spread, SpreadId};
use lamina_layout::LayoutCandidate;

use crate::editor::Panel;
use crate::edits::timeline;
use crate::photos::{Filter, PhotoItem};
use crate::session::Session;
use crate::suggestions::List;

/// Uma mudança numa lista da janela. As operações valem na ordem dada; `from` é a posição na
/// lista nova de onde vem o valor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowOp {
    Set { at: usize, from: usize },
    Insert { at: usize, from: usize },
    Remove { at: usize },
}

/// Acima disto, a parte do meio que mudou é trocada inteira (sem procurar o que se manteve).
const MAX_CELLS: usize = 1_000_000;

/// As operações que levam `old` a `new`: as linhas com a mesma chave ficam (trocadas só se o
/// valor mudou), as que saíram são removidas e as que chegaram, inseridas.
pub fn row_ops<K: PartialEq, V: PartialEq>(old: &[(K, V)], new: &[(K, V)]) -> Vec<RowOp> {
    let start = old.iter().zip(new).take_while(|(a, b)| a.0 == b.0).count();
    let end = old[start..]
        .iter()
        .rev()
        .zip(new[start..].iter().rev())
        .take_while(|(a, b)| a.0 == b.0)
        .count();
    let (a, b) = (&old[start..old.len() - end], &new[start..new.len() - end]);
    let mut ops: Vec<RowOp> = (0..start)
        .filter(|&i| old[i].1 != new[i].1)
        .map(|i| RowOp::Set { at: i, from: i })
        .collect();
    let (mut at, mut i, mut j) = (start, 0, 0);
    for (mi, mj) in matching(a, b).into_iter().chain(once((a.len(), b.len()))) {
        ops.extend((i..mi).map(|_| RowOp::Remove { at }));
        for k in j..mj {
            ops.push(RowOp::Insert {
                at,
                from: start + k,
            });
            at += 1;
        }
        if mi < a.len() {
            if a[mi].1 != b[mj].1 {
                ops.push(RowOp::Set {
                    at,
                    from: start + mj,
                });
            }
            at += 1;
        }
        (i, j) = (mi + 1, mj + 1);
    }
    let tail = (0..end).filter_map(|k| {
        let (o, n) = (old.len() - end + k, new.len() - end + k);
        (old[o].1 != new[n].1).then_some(RowOp::Set { at: n, from: n })
    });
    ops.extend(tail);
    ops
}

/// Os pares (posição em `a`, posição em `b`) das chaves que ficam, em ordem crescente (a maior
/// sequência comum).
fn matching<K: PartialEq, V>(a: &[(K, V)], b: &[(K, V)]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 || n.saturating_mul(m) > MAX_CELLS {
        return Vec::new();
    }
    // lcs[i * (m + 1) + j] = tamanho da maior sequência comum de a[i..] e b[j..].
    let w = m + 1;
    let mut lcs = vec![0_u32; (n + 1) * w];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i * w + j] = if a[i].0 == b[j].0 {
                lcs[(i + 1) * w + j + 1] + 1
            } else {
                lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i].0 == b[j].0 {
            out.push((i, j));
            (i, j) = (i + 1, j + 1);
        } else if lcs[(i + 1) * w + j] >= lcs[i * w + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// O que a faixa de lâminas mostra de uma lâmina.
#[derive(Debug, Clone, PartialEq)]
pub struct StripRow {
    pub spread: Spread,
    /// As páginas ativas (a primeira e a última lâmina podem ter só uma).
    pub active: ActivePages,
    pub warned: bool,
    pub missing: bool,
    /// Os ajustes das fotos da lâmina (spec AF): mudar um refaz a miniatura.
    pub adjusts: Vec<lamina_core::PhotoAdjust>,
}

/// As linhas da faixa de lâminas, pela lâmina.
pub fn strip_rows(session: &Session) -> Vec<(SpreadId, StripRow)> {
    let project = session.album();
    let count = project.spreads.len();
    let missing = session.missing();
    project
        .spreads
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let row = StripRow {
                spread: s.clone(),
                active: project.spec.edges().active_pages(i, count),
                warned: !session.spread_warnings(i).is_empty(),
                missing: photos_in(s).any(|id| missing.contains(&id)),
                adjusts: photos_in(s)
                    .filter_map(|id| project.photo(id).ok().map(|p| p.adjust))
                    .collect(),
            };
            (s.id, row)
        })
        .collect()
}

fn photos_in(spread: &Spread) -> impl Iterator<Item = PhotoId> + '_ {
    spread.frames.iter().filter_map(|f| match &f.content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    })
}

/// O que o painel de fotos mostra de uma foto.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoRow {
    pub item: PhotoItem,
    /// Faz parte da seleção do painel.
    pub picked: bool,
    /// Já usada, na aba Todas: aparece esmaecida (spec PF, regra 11).
    pub dim: bool,
    /// O arquivo não está onde estava: a marca âmbar (spec RF, regra 8). Quem monta as linhas
    /// sabe quais faltam (a sessão); aqui começa desligada.
    pub missing: bool,
}

/// As linhas do painel de fotos da aba `filter`, pela foto.
pub fn photo_rows(
    items: &[PhotoItem],
    selection: &[PhotoId],
    filter: Filter,
) -> Vec<(PhotoId, PhotoRow)> {
    let picked: HashSet<PhotoId> = selection.iter().copied().collect();
    items
        .iter()
        .map(|item| {
            let row = PhotoRow {
                item: item.clone(),
                picked: picked.contains(&item.id),
                dim: filter == Filter::All && item.uses > 0,
                missing: false,
            };
            (item.id, row)
        })
        .collect()
}

/// Uma posição da linha do tempo: a n-ésima vez que uma foto aparece, ou o corte depois de uma
/// lâmina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKey {
    Photo(PhotoId, usize),
    Cut(usize),
}

/// As linhas da linha do tempo; o valor é a lâmina de cada posição.
pub fn timeline_rows(project: &Project) -> Vec<(TimelineKey, usize)> {
    let spreads = timeline(project);
    let last = spreads.len().saturating_sub(1);
    let mut seen: Vec<PhotoId> = Vec::new();
    let mut rows = Vec::new();
    for (k, photos) in spreads.iter().enumerate() {
        for id in photos {
            let nth = seen.iter().filter(|s| *s == id).count();
            seen.push(*id);
            rows.push((TimelineKey::Photo(*id, nth), k));
        }
        if k < last {
            rows.push((TimelineKey::Cut(k), k));
        }
    }
    rows
}

/// O que as prévias do painel de layouts usam: igual, as prévias feitas valem de novo, e só a
/// opção em uso precisa mudar na janela.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelKey {
    photos: Vec<PhotoId>,
    aspects: Vec<u64>,
    gap: u64,
    whole: bool,
    active: Option<ActivePages>,
    list: List,
    options: Vec<LayoutCandidate>,
    favorite: Vec<bool>,
    unavailable: bool,
    /// O que as prévias copiam da lâmina atual além do arranjo: medidas, fundo, bordas, cantos e
    /// fotos ausentes (escrito por extenso, só para comparar).
    look: String,
}

/// A chave das prévias do `panel` da lâmina atual da `session`.
pub fn panel_key(panel: &Panel, session: &Session) -> PanelKey {
    let sel = panel.selection.as_ref();
    let project = session.project();
    let look = project.spreads.get(session.current()).map(|s| {
        let mut styles: Vec<String> = s
            .frames
            .iter()
            .map(|f| format!("{:?} {:?}", f.border, f.radius))
            .collect();
        styles.sort();
        styles.dedup();
        let missing: Vec<&PhotoId> = photos_in(s)
            .filter_map(|id| session.missing().iter().find(|m| **m == id))
            .collect();
        format!(
            "{:?} {:?} {:?} {styles:?} {missing:?}",
            project.spec,
            s.background,
            s.page_background(PageSide::Right)
        )
    });
    PanelKey {
        look: look.unwrap_or_default(),
        photos: sel.map(|s| s.photos.clone()).unwrap_or_default(),
        aspects: sel
            .map(|s| s.aspects.iter().map(|a| a.to_bits()).collect())
            .unwrap_or_default(),
        gap: sel.map_or(0, |s| s.gap.get().to_bits()),
        whole: sel.is_some_and(|s| s.whole),
        active: sel.map(|s| s.active),
        list: panel.list,
        options: panel.options.clone(),
        favorite: panel.favorite.clone(),
        unavailable: panel.unavailable.is_some(),
    }
}
