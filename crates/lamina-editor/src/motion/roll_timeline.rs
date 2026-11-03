//! O selo da lâmina e o trecho dela na linha do tempo (spec RL, regras 16 e 22).

use lamina_core::{AlbumEdge, Edges};

use super::rows::TimelineKey;

/// O selo da lâmina `index` num álbum de `count` lâminas: "Lâmina 5 · páginas 8 e 9", com a
/// numeração impressa (regra 16).
pub fn spread_badge(edges: Edges, index: usize, count: usize) -> String {
    let n = index + 1;
    let active = edges.active_pages(index, count);
    let first = match edges.start {
        AlbumEdge::SinglePage if index == 0 => 1,
        AlbumEdge::SinglePage => 2 * index,
        AlbumEdge::FullSpread => 2 * index + 1,
    };
    match (active.left, active.right) {
        (true, true) => format!("Lâmina {n} · páginas {first} e {}", first + 1),
        (true, false) | (false, true) => format!("Lâmina {n} · página {first}"),
        (false, false) => format!("Lâmina {n}"),
    }
}

/// Os quadrinhos da linha do tempo, em pixels (os de hoje).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineSizes {
    pub photo: f64,
    pub cut: f64,
    pub spacing: f64,
    pub padding: f64,
}

/// As medidas da linha do tempo da janela.
pub const TIMELINE: TimelineSizes = TimelineSizes {
    photo: 46.0,
    cut: 14.0,
    spacing: 2.0,
    padding: 6.0,
};

/// O trecho da lâmina `spread` na linha do tempo: o começo e a largura, da primeira à última
/// foto dela (regra 22).
pub fn timeline_run(rows: &[(TimelineKey, usize)], spread: usize, s: TimelineSizes) -> (f64, f64) {
    let mut x = s.padding;
    let mut run: Option<(f64, f64)> = None;
    for (key, owner) in rows {
        let wide = match key {
            TimelineKey::Photo(..) => s.photo,
            TimelineKey::Cut(_) => s.cut,
        };
        let mine = *owner == spread && matches!(key, TimelineKey::Photo(..));
        if mine {
            run = Some(run.map_or((x, x + wide), |(a, _)| (a, x + wide)));
        } else if run.is_none() && *owner >= spread {
            run = Some((x, x));
        }
        x += wide + s.spacing;
    }
    let (a, b) = run.unwrap_or((x, x));
    (a, b - a)
}

/// A foto marcada pelo teclado na linha do tempo (spec DD, regra 28): a partir da linha `from`, anda
/// uma foto para `forward`, pulando os cortes, e fica na ponta. Sem marca (`from` vazio), é a
/// primeira foto da lâmina `spread`, ou a primeira depois dela, se ela não tiver fotos.
pub fn timeline_step(
    rows: &[(TimelineKey, usize)],
    from: Option<usize>,
    forward: bool,
    spread: usize,
) -> Option<usize> {
    let is_photo = |i: &usize| matches!(rows[*i].0, TimelineKey::Photo(..));
    let Some(from) = from else {
        let first = (0..rows.len()).find(|&i| is_photo(&i) && rows[i].1 >= spread);
        return first.or_else(|| (0..rows.len()).rev().find(is_photo));
    };
    let next = if forward {
        (from + 1..rows.len()).find(is_photo)
    } else {
        (0..from.min(rows.len())).rev().find(is_photo)
    };
    next.or(Some(from).filter(|i| *i < rows.len()))
}

/// O deslocamento da linha do tempo que deixa o meio do trecho `run` no meio da faixa visível
/// (`view` de largura), sem passar das pontas do conteúdo (`content` de largura).
pub fn timeline_scroll(run: (f64, f64), view: f64, content: f64) -> f64 {
    let want = run.0 + run.1 / 2.0 - view / 2.0;
    want.min(content - view).max(0.0)
}
