//! Modelos do fotógrafo (spec MD): uma lâmina salva vira uma opção das listas quando as fotos
//! casam com ela. O modelo guarda as posições em frações do corte (ADR 0003) e a orientação da
//! foto que cabe em cada quadro.

use lamina_core::{ActivePages, Border, Mm, Orientation, Rect, RelRect, TextBox};
use serde::{Deserialize, Serialize};

use super::area::Ctx;
use super::fill::crop_of;
use super::score::score;
use super::{Breathing, Family, LayoutCandidate};

/// Um quadro de foto do modelo. `orientation` `None` aceita qualquer foto (quadro vazio).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSlot {
    pub rect: RelRect,
    #[serde(default)]
    pub orientation: Option<Orientation>,
    #[serde(default)]
    pub border: Border,
    #[serde(default)]
    pub radius: Mm,
}

/// Um quadro de texto do modelo (spec TX).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelText {
    pub rect: RelRect,
    pub text: TextBox,
}

/// Um modelo salvo pelo fotógrafo (`modelos.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpreadModel {
    pub name: String,
    pub slots: Vec<ModelSlot>,
    #[serde(default)]
    pub texts: Vec<ModelText>,
    /// Páginas que a lâmina usa.
    pub left: bool,
    pub right: bool,
    /// É um modelo de fábrica (spec RM, beta 9.1): chave `fabrica:`, não gravado em
    /// `modelos.json`.
    #[serde(default)]
    pub factory: bool,
}

/// O que o candidato de um modelo leva além dos quadros: borda e cantos de cada foto (na ordem
/// original das fotos) e os textos.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelLook {
    pub styles: Vec<(Border, Mm)>,
    pub texts: Vec<ModelText>,
}

fn flip(r: RelRect) -> RelRect {
    let x = (1.0 - r.x() - r.width()).clamp(0.0, 1.0);
    RelRect::new(x, r.y(), r.width(), r.height()).unwrap_or(r)
}

impl SpreadModel {
    pub fn pages(&self) -> ActivePages {
        ActivePages {
            left: self.left,
            right: self.right,
        }
    }

    /// O mesmo modelo com esquerda e direita trocadas (regra 4).
    pub fn mirrored(&self) -> SpreadModel {
        SpreadModel {
            name: self.name.clone(),
            factory: self.factory,
            slots: self
                .slots
                .iter()
                .map(|s| ModelSlot {
                    rect: flip(s.rect),
                    ..s.clone()
                })
                .collect(),
            texts: self
                .texts
                .iter()
                .map(|t| ModelText {
                    rect: flip(t.rect),
                    text: t.text.clone(),
                })
                .collect(),
            left: self.right,
            right: self.left,
        }
    }

    /// Igual ao próprio espelho: os mesmos quadros com as mesmas orientações e os mesmos textos
    /// no mesmo lugar, em qualquer ordem (auditoria do Marco 3, M7).
    pub fn is_symmetric(&self) -> bool {
        let m = self.mirrored();
        let same = |a: &RelRect, b: &RelRect| {
            [
                a.x() - b.x(),
                a.y() - b.y(),
                a.width() - b.width(),
                a.height() - b.height(),
            ]
            .iter()
            .all(|d| d.abs() < 1e-6)
        };
        let slots = self.slots.iter().all(|s| {
            m.slots
                .iter()
                .any(|t| same(&s.rect, &t.rect) && s.orientation == t.orientation)
        });
        let texts = self.texts.iter().all(|s| {
            m.texts
                .iter()
                .any(|t| same(&s.rect, &t.rect) && s.text == t.text)
        });
        m.pages() == self.pages() && slots && texts
    }
}

/// A foto de proporção `aspect` cabe no quadro?
fn fits(aspect: f64, slot: &ModelSlot) -> bool {
    let photo = Orientation::from_aspect_ratio(aspect);
    photo == Orientation::Square || slot.orientation.is_none_or(|o| o == photo)
}

/// Uma rodada da arrumação: a foto de proporção dada pode ir para o quadro?
type Pass<'a> = dyn Fn(f64, &ModelSlot) -> bool + 'a;

/// Quadro de cada foto: na ordem, se casa; senão, a primeira arrumação que casa (as fotos
/// horizontais e verticais primeiro nos quadros da mesma orientação, depois nos livres, e as
/// quadradas no que sobrar). `None` = não casa (regra 2).
pub fn assign(aspects: &[f64], slots: &[ModelSlot]) -> Option<(Vec<usize>, bool)> {
    if aspects.len() != slots.len() || slots.is_empty() {
        return None;
    }
    if aspects.iter().zip(slots).all(|(a, s)| fits(*a, s)) {
        return Some(((0..slots.len()).collect(), true));
    }
    let mut slot_of: Vec<Option<usize>> = vec![None; aspects.len()];
    let mut free = vec![true; slots.len()];
    let square = |a: f64| Orientation::from_aspect_ratio(a) == Orientation::Square;
    let passes: [&Pass<'_>; 3] = [
        &|a, s| !square(a) && s.orientation == Some(Orientation::from_aspect_ratio(a)),
        &|a, s| !square(a) && s.orientation.is_none(),
        &|a, _| square(a),
    ];
    for pass in passes {
        for (photo, a) in aspects.iter().enumerate() {
            if slot_of[photo].is_some() {
                continue;
            }
            if let Some(k) = (0..slots.len()).find(|&k| free[k] && pass(*a, &slots[k])) {
                slot_of[photo] = Some(k);
                free[k] = false;
            }
        }
    }
    let slots: Option<Vec<usize>> = slot_of.into_iter().collect();
    slots.map(|s| (s, false))
}

/// O candidato do modelo para as fotos `aspects`, com o quadro de cada foto em `slot_of`.
fn candidate(
    ctx: &Ctx,
    aspects: &[f64],
    model: &SpreadModel,
    mirrored: bool,
    slot_of: &[usize],
) -> LayoutCandidate {
    let trim = ctx.spec.trim_rect();
    let rects: Vec<Rect> = slot_of
        .iter()
        .map(|&k| model.slots[k].rect.to_absolute(&trim))
        .collect();
    let crop: Vec<f64> = rects
        .iter()
        .zip(aspects)
        .map(|(r, a)| crop_of(r.width.get() / r.height.get().max(1e-9), *a))
        .collect();
    let useful: f64 = ctx
        .sides()
        .into_iter()
        .map(|s| {
            let r = ctx.spec.safe_rect(s);
            r.width.get() * r.height.get()
        })
        .sum();
    let area: f64 = rects.iter().map(|r| r.width.get() * r.height.get()).sum();
    // Sequência das fotos pelos quadros: a foto de cada quadro, na ordem dos quadros.
    let mut order: Vec<usize> = (0..slot_of.len()).collect();
    order.sort_by_key(|&photo| slot_of[photo]);
    LayoutCandidate {
        family: Family::Model {
            name: model.name.clone(),
            mirrored,
            factory: model.factory,
        },
        breathing: Breathing::default(),
        frames: slot_of.iter().map(|&k| model.slots[k].rect).collect(),
        order,
        coverage: (area / useful.max(1e-9)).min(1.0),
        score: score(ctx, &rects, &crop, useful.max(1e-9)),
        crop,
        model: Some(ModelLook {
            styles: slot_of
                .iter()
                .map(|&k| (model.slots[k].border, model.slots[k].radius))
                .collect(),
            texts: model.texts.clone(),
        }),
    }
}

/// Os candidatos dos modelos que casam: (na ordem, reordenados).
/// Um candidato do motor vira modelo de fábrica (spec RM, regra 5): cada foto, na ordem, com a
/// orientação de `aspects` e as frações do corte do candidato. As páginas são as da mistura
/// (`both`: lâmina aberta; senão página única).
pub fn model_from_candidate(
    c: &LayoutCandidate,
    aspects: &[f64],
    name: &str,
    both: bool,
) -> Option<SpreadModel> {
    if c.model.is_some() {
        return None;
    }
    let mut slots = Vec::new();
    for (i, rect) in c.frames.iter().enumerate() {
        let orientation = c
            .order
            .get(i)
            .and_then(|&photo| aspects.get(photo))
            .map(|a| Orientation::from_aspect_ratio(*a));
        slots.push(ModelSlot {
            rect: *rect,
            orientation,
            border: Border::default(),
            radius: Mm::default(),
        });
    }
    if slots.is_empty() {
        return None;
    }
    Some(SpreadModel {
        name: name.to_owned(),
        slots,
        texts: Vec::new(),
        left: both,
        right: true,
        factory: true,
    })
}

pub(super) fn candidates(
    ctx: &Ctx,
    aspects: &[f64],
    models: &[SpreadModel],
) -> (Vec<LayoutCandidate>, Vec<LayoutCandidate>) {
    let (mut in_order, mut reordered) = (Vec::new(), Vec::new());
    for model in models {
        let mirror = (!model.is_symmetric()).then(|| model.mirrored());
        let variants = std::iter::once((model.clone(), false)).chain(mirror.map(|m| (m, true)));
        for (m, mirrored) in variants {
            if m.pages() != ctx.active {
                continue;
            }
            if let Some((slot_of, straight)) = assign(aspects, &m.slots) {
                let c = candidate(ctx, aspects, &m, mirrored, &slot_of);
                if straight {
                    in_order.push(c);
                } else {
                    reordered.push(c);
                }
            }
        }
    }
    (in_order, reordered)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(x: f64, w: f64, o: Option<Orientation>) -> ModelSlot {
        ModelSlot {
            rect: RelRect::new(x, 0.2, w, 0.5).unwrap(),
            orientation: o,
            border: Border::default(),
            radius: Mm(0.0),
        }
    }

    const H: f64 = 1.5;
    const V: f64 = 0.66;
    const Q: f64 = 1.0;

    #[test]
    fn casa_na_ordem_noutra_ordem_ou_nao_casa() {
        let (h, v) = (Some(Orientation::Landscape), Some(Orientation::Portrait));
        let slots = [slot(0.0, 0.2, h), slot(0.3, 0.2, v), slot(0.6, 0.2, None)];
        assert_eq!(assign(&[H, V, V], &slots), Some((vec![0, 1, 2], true)));
        // V, H, H: a vertical vai para o quadro vertical, uma horizontal para o livre.
        assert_eq!(assign(&[V, H, H], &slots), Some((vec![1, 0, 2], false)));
        assert_eq!(assign(&[V, V, V], &slots), None);
        assert_eq!(assign(&[Q, Q, H], &slots), Some((vec![0, 1, 2], true)));
        assert_eq!(assign(&[H, V], &slots), None, "número de fotos diferente");
    }

    #[test]
    fn espelho_troca_os_lados_e_simetria_e_reconhecida() {
        let model = SpreadModel {
            name: "m".into(),
            slots: vec![slot(0.1, 0.2, None)],
            texts: Vec::new(),
            left: true,
            right: false,
            factory: false,
        };
        let m = model.mirrored();
        assert!((m.slots[0].rect.x() - 0.7).abs() < 1e-9);
        assert!(!m.left && m.right);
        assert!(!model.is_symmetric());
        let both = SpreadModel {
            slots: vec![slot(0.1, 0.2, None), slot(0.7, 0.2, None)],
            left: true,
            right: true,
            ..model
        };
        assert!(both.is_symmetric());
    }
}
