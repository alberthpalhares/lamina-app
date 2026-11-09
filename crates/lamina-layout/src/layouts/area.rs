//! Áreas onde os blocos são montados e o respiro (spec VL, regra 4): margem extra e
//! alinhamento do bloco dentro da área.

use lamina_core::{ActivePages, AlbumSpec, DesignMode, Mm, PageSide, Rect};

use super::{Align, Breathing, Family};

/// Margem extra de cada nível de respiro, em fração da área, de cada lado.
pub const MARGIN_LEVELS: [f64; 4] = [0.0, 0.08, 0.16, 0.24];

/// Tolerância em mm para comparar posições.
pub const EPS: f64 = 1e-6;

/// O que todo gerador precisa: a especificação, as páginas ativas, o espaço entre fotos e a
/// margem extra do nível de respiro.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub spec: &'a AlbumSpec,
    pub active: ActivePages,
    pub gap: f64,
    pub inset: f64,
}

impl Ctx<'_> {
    pub fn both(&self) -> bool {
        self.active.left && self.active.right
    }

    /// Lâmina inteira disponível: modo lâmina com as duas páginas ativas.
    pub fn spread_mode(&self) -> bool {
        self.both() && self.spec.mode() == DesignMode::Spread
    }

    pub fn sides(&self) -> Vec<PageSide> {
        [
            (self.active.left, PageSide::Left),
            (self.active.right, PageSide::Right),
        ]
        .into_iter()
        .filter_map(|(on, side)| on.then_some(side))
        .collect()
    }

    /// Área útil de uma página, com a margem extra.
    pub fn page_area(&self, side: PageSide) -> Rect {
        shrink(self.spec.safe_rect(side), self.inset)
    }

    /// Área útil da lâmina inteira, da borda externa esquerda à direita, com a margem extra.
    pub fn spread_area(&self) -> Rect {
        let (l, r) = (
            self.spec.safe_rect(PageSide::Left),
            self.spec.safe_rect(PageSide::Right),
        );
        let top = l.y.get().max(r.y.get());
        let bottom = l.bottom().get().min(r.bottom().get());
        let area = Rect::new(l.x, Mm(top), r.right() - l.x, Mm(bottom - top));
        shrink(area, self.inset)
    }

    /// Posição da lombada (meio do corte da lâmina), em mm.
    pub fn gutter(&self) -> f64 {
        let trim = self.spec.trim_rect();
        trim.x.get() + trim.width.get() / 2.0
    }
}

fn shrink(r: Rect, fraction: f64) -> Rect {
    let (dx, dy) = (r.width.get() * fraction, r.height.get() * fraction);
    Rect::new(
        r.x + Mm(dx),
        r.y + Mm(dy),
        r.width - Mm(2.0 * dx),
        r.height - Mm(2.0 * dy),
    )
}

/// Fotos de uma área, na sequência do arranjo.
#[derive(Clone)]
pub struct Block {
    pub area: Rect,
    pub rects: Vec<Rect>,
}

/// Um arranjo sem recorte, antes do alinhamento: família e blocos em sequência.
pub type Shaped = (Family, Vec<Block>);

/// Candidato bruto: quadros em mm na sequência do arranjo.
pub struct Raw {
    pub family: Family,
    pub breathing: Breathing,
    pub rects: Vec<Rect>,
}

/// As três versões de alinhamento de um arranjo (as que mudam alguma coisa). Com `window`
/// (listas da janela, spec RM), só o centralizado: as variantes deslocadas (Start/End, cima/baixo
/// e esquerda/direita) saem — quem quiser deslocado ajusta à mão e salva como modelo.
pub fn aligned(shaped: Shaped, margin_level: u8, gutter: f64, window: bool) -> Vec<Raw> {
    let (family, blocks) = shaped;
    let mut out = Vec::with_capacity(3);
    let alignments: &[Align] = if window {
        &[Align::Center]
    } else {
        &[Align::Center, Align::Start, Align::End]
    };
    for &align in alignments {
        let mut moved = false;
        let mut rects = Vec::new();
        for block in &blocks {
            let (dx, dy) = shift(block, align, gutter);
            moved |= dx.abs() > EPS || dy.abs() > EPS;
            rects.extend(
                block
                    .rects
                    .iter()
                    .map(|r| Rect::new(r.x + Mm(dx), r.y + Mm(dy), r.width, r.height)),
            );
        }
        if align == Align::Center || moved {
            let breathing = Breathing {
                margin_level,
                align,
            };
            out.push(Raw {
                family: family.clone(),
                breathing,
                rects,
            });
        }
    }
    out
}

/// Deslocamento do bloco: na altura, se sobra altura; senão na largura, para a borda externa
/// (`Start`) ou para a lombada (`End`).
fn shift(block: &Block, align: Align, gutter: f64) -> (f64, f64) {
    let Some(bbox) = bounds(&block.rects) else {
        return (0.0, 0.0);
    };
    let area = block.area;
    if align == Align::Center {
        return (0.0, 0.0);
    }
    let start = align == Align::Start;
    if area.height.get() - bbox.height.get() > EPS {
        let dy = if start {
            area.y - bbox.y
        } else {
            area.bottom() - bbox.bottom()
        };
        return (0.0, dy.get());
    }
    if area.width.get() - bbox.width.get() > EPS {
        let right_page = area.x.get() + area.width.get() / 2.0 > gutter + EPS;
        let to_left = start != right_page;
        let dx = if to_left {
            area.x - bbox.x
        } else {
            area.right() - bbox.right()
        };
        return (dx.get(), 0.0);
    }
    (0.0, 0.0)
}

/// Menor retângulo que contém todos os quadros.
pub fn bounds(rects: &[Rect]) -> Option<Rect> {
    let first = rects.first()?;
    let (mut x0, mut y0) = (first.x.get(), first.y.get());
    let (mut x1, mut y1) = (first.right().get(), first.bottom().get());
    for r in rects {
        x0 = x0.min(r.x.get());
        y0 = y0.min(r.y.get());
        x1 = x1.max(r.right().get());
        y1 = y1.max(r.bottom().get());
    }
    Some(Rect::new(Mm(x0), Mm(y0), Mm(x1 - x0), Mm(y1 - y0)))
}
