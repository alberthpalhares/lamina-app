//! Alinhar, Igualar e Distribuir os quadros do modo organizar (spec EG, regra 9), em mm. Alinhar e
//! Igualar usam o primeiro quadro clicado (a referência); Igualar muda o tamanho a partir do
//! centro de cada quadro (o recorte da foto guarda o centro); Distribuir não mexe nas pontas.

use lamina_core::{Mm, Rect};

/// Para onde Alinhar leva os quadros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

/// O que Igualar copia da referência.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Equal {
    Width,
    Height,
    Size,
}

/// Os quadros alinhados ao de índice `reference`.
pub fn align(rects: &[Rect], reference: usize, how: Align) -> Option<Vec<Rect>> {
    let r = rects.get(reference)?;
    let (cx, cy) = (
        r.x.get() + r.width.get() / 2.0,
        r.y.get() + r.height.get() / 2.0,
    );
    Some(
        rects
            .iter()
            .map(|o| {
                let (w, h) = (o.width.get(), o.height.get());
                let (x, y) = (o.x.get(), o.y.get());
                let (x, y) = match how {
                    Align::Left => (r.x.get(), y),
                    Align::Center => (cx - w / 2.0, y),
                    Align::Right => (r.right().get() - w, y),
                    Align::Top => (x, r.y.get()),
                    Align::Middle => (x, cy - h / 2.0),
                    Align::Bottom => (x, r.bottom().get() - h),
                };
                Rect::new(Mm(x), Mm(y), o.width, o.height)
            })
            .collect(),
    )
}

/// Os quadros com a largura, a altura ou as duas do de índice `reference`, cada um no seu centro.
pub fn equalize(rects: &[Rect], reference: usize, how: Equal) -> Option<Vec<Rect>> {
    let r = rects.get(reference)?;
    Some(
        rects
            .iter()
            .map(|o| {
                let w = if how == Equal::Height {
                    o.width
                } else {
                    r.width
                };
                let h = if how == Equal::Width {
                    o.height
                } else {
                    r.height
                };
                let cx = o.x.get() + o.width.get() / 2.0;
                let cy = o.y.get() + o.height.get() / 2.0;
                Rect::new(Mm(cx - w.get() / 2.0), Mm(cy - h.get() / 2.0), w, h)
            })
            .collect(),
    )
}

/// Os quadros alinhados à **área útil da lâmina** `spread` (pedido do dono na conferência da
/// beta 9.1: alinhar com referência na lâmina inteira, não só entre os selecionados). Alinhar ao
/// "Left" é a borda esquerda da área; "Center", o centro dela; e assim por diante.
pub fn align_to(rects: &[Rect], spread: Rect, how: Align) -> Vec<Rect> {
    let (cx, cy) = (
        spread.x.get() + spread.width.get() / 2.0,
        spread.y.get() + spread.height.get() / 2.0,
    );
    rects
        .iter()
        .map(|o| {
            let (w, h) = (o.width.get(), o.height.get());
            let (x, y) = (o.x.get(), o.y.get());
            let (x, y) = match how {
                Align::Left => (spread.x.get(), y),
                Align::Center => (cx - w / 2.0, y),
                Align::Right => (spread.right().get() - w, y),
                Align::Top => (x, spread.y.get()),
                Align::Middle => (x, cy - h / 2.0),
                Align::Bottom => (x, spread.bottom().get() - h),
            };
            Rect::new(Mm(x), Mm(y), o.width, o.height)
        })
        .collect()
}

/// Distribuir na horizontal (`across`) ou vertical **dentro da área útil da lâmina**: os vãos
/// entre os quadros (e das pontas até a borda da área) ficam iguais. `None` com menos de um
/// quadro.
pub fn distribute_to(rects: &[Rect], spread: Rect, across: bool) -> Option<Vec<Rect>> {
    if rects.is_empty() {
        return None;
    }
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by(|a, b| {
        axis(&rects[*a], across)
            .0
            .total_cmp(&axis(&rects[*b], across).0)
    });
    let total: f64 = order.iter().map(|i| axis(&rects[*i], across).1).sum();
    let span = axis(&spread, across).1;
    let gaps = f64::from(u32::try_from(rects.len() + 1).unwrap_or(u32::MAX));
    let gap = (span - total) / gaps;
    let mut out = rects.to_vec();
    let mut at = axis(&spread, across).0 + gap;
    for i in order {
        let r = rects[i];
        out[i] = if across {
            Rect::new(Mm(at), r.y, r.width, r.height)
        } else {
            Rect::new(r.x, Mm(at), r.width, r.height)
        };
        at += axis(&r, across).1 + gap;
    }
    Some(out)
}

/// O começo e o tamanho num eixo: x (`across`) ou y.
fn axis(r: &Rect, across: bool) -> (f64, f64) {
    if across {
        (r.x.get(), r.width.get())
    } else {
        (r.y.get(), r.height.get())
    }
}

/// Vãos iguais na horizontal (`across`) ou na vertical, sem mexer no primeiro nem no último
/// quadro; `None` com menos de três.
pub fn distribute(rects: &[Rect], across: bool) -> Option<Vec<Rect>> {
    if rects.len() < 3 {
        return None;
    }
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by(|a, b| {
        axis(&rects[*a], across)
            .0
            .total_cmp(&axis(&rects[*b], across).0)
    });
    let (first, last) = (order[0], order[order.len() - 1]);
    let (start, first_size) = axis(&rects[first], across);
    let end = axis(&rects[last], across).0;
    let middle: f64 = order[1..order.len() - 1]
        .iter()
        .map(|i| axis(&rects[*i], across).1)
        .sum();
    let spaces = f64::from(u32::try_from(rects.len() - 1).unwrap_or(u32::MAX));
    let gap = (end - (start + first_size) - middle) / spaces;
    let mut out = rects.to_vec();
    let mut at = start + first_size + gap;
    for i in &order[1..order.len() - 1] {
        let r = rects[*i];
        out[*i] = if across {
            Rect::new(Mm(at), r.y, r.width, r.height)
        } else {
            Rect::new(r.x, Mm(at), r.width, r.height)
        };
        at += axis(&r, across).1 + gap;
    }
    Some(out)
}
