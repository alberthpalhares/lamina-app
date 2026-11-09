//! Alinhar, Igualar e Distribuir no modo organizar e na edição comum (specs EG e GJ). As contas
//! estão em [`crate::arrange`]; cada comando é uma edição. Os campos da seleção ficam em
//! `group_fields.rs`. Os comandos "à lâmina" usam a área útil dela como referência (pedido do
//! dono na conferência da beta 9.1).

use lamina_core::{Mm, PageSide, Rect};

use super::Editor;
use crate::Command;
use crate::arrange::{Align, Equal, align, align_to, distribute, distribute_to, equalize};
use crate::hand::set_rect;

/// O aviso de Distribuir com menos de três quadros (seção 6).
pub const NEED_THREE: &str = "Selecione três quadros ou mais.";
/// O que cada comando faz com os retângulos (e o rótulo do histórico).
enum Arrange {
    Align(Align),
    Equal(Equal),
    Distribute(bool),
    /// Com a área útil da lâmina como referência (spec GJ, conferência do dono 2026-11-04).
    AlignToSpread(Align),
    DistributeToSpread(bool),
}

fn arrange_of(command: Command) -> Option<Arrange> {
    use Command as C;
    Some(match command {
        C::AlignLeft => Arrange::Align(Align::Left),
        C::AlignCenter => Arrange::Align(Align::Center),
        C::AlignRight => Arrange::Align(Align::Right),
        C::AlignTop => Arrange::Align(Align::Top),
        C::AlignMiddle => Arrange::Align(Align::Middle),
        C::AlignBottom => Arrange::Align(Align::Bottom),
        C::EqualWidth => Arrange::Equal(Equal::Width),
        C::EqualHeight => Arrange::Equal(Equal::Height),
        C::EqualSize => Arrange::Equal(Equal::Size),
        C::DistributeAcross => Arrange::Distribute(true),
        C::DistributeDown => Arrange::Distribute(false),
        C::AlignLeftToSpread => Arrange::AlignToSpread(Align::Left),
        C::AlignCenterToSpread => Arrange::AlignToSpread(Align::Center),
        C::AlignRightToSpread => Arrange::AlignToSpread(Align::Right),
        C::AlignTopToSpread => Arrange::AlignToSpread(Align::Top),
        C::AlignMiddleToSpread => Arrange::AlignToSpread(Align::Middle),
        C::AlignBottomToSpread => Arrange::AlignToSpread(Align::Bottom),
        C::DistributeAcrossToSpread => Arrange::DistributeToSpread(true),
        C::DistributeDownToSpread => Arrange::DistributeToSpread(false),
        _ => return None,
    })
}

impl Editor {
    /// Disponibilidade de Alinhar, Igualar e Distribuir; `None` = não é deles. Valem no modo
    /// organizar e na edição comum (spec GJ): pedem 2 quadros selecionados (3 para Distribuir
    /// entre os selecionados); os "à lâmina" bastam 1 — alinhar uma foto só à área da lâmina é
    /// a uso comum (pedido do dono 2026-11-09).
    pub(super) fn arrange_available(&self, command: Command) -> Option<bool> {
        let n = self.selected_frames().len();
        let need = match arrange_of(command)? {
            Arrange::Distribute(_) => 3,
            Arrange::AlignToSpread(_) | Arrange::DistributeToSpread(_) => 1,
            _ => 2,
        };
        Some(n >= need)
    }

    /// Roda Alinhar, Igualar ou Distribuir; `None` = não é deles.
    pub(super) fn arrange_run(&mut self, command: Command) -> Option<bool> {
        let how = arrange_of(command)?;
        if self.arrange_available(command) != Some(true) {
            return Some(false);
        }
        let frames = self.selected_frames();
        let all = self.frames_mm();
        let rects: Vec<Rect> = frames.iter().filter_map(|f| all.get(*f).copied()).collect();
        let reference = self
            .selected_frame
            .and_then(|r| frames.iter().position(|f| *f == r))
            .unwrap_or(0);
        let (moved, label) = match how {
            Arrange::Align(a) => (align(&rects, reference, a), "Alinhar quadros"),
            Arrange::Equal(e) => (equalize(&rects, reference, e), command.label()),
            Arrange::Distribute(across) => (distribute(&rects, across), "Distribuir quadros"),
            Arrange::AlignToSpread(a) => (
                Some(align_to(&rects, self.spread_area(), a)),
                command.label(),
            ),
            Arrange::DistributeToSpread(across) => (
                distribute_to(&rects, self.spread_area(), across),
                command.label(),
            ),
        };
        let Some(moved) = moved.filter(|m| *m != rects) else {
            return Some(false);
        };
        Some(self.change(label, |p, i| {
            frames
                .iter()
                .zip(&moved)
                .try_for_each(|(f, r)| set_rect(p, i, *f, *r))
        }))
    }

    /// A área útil da lâmina atual (a união das zonas seguras das páginas ativas; spec GJ). Sem
    /// sessão, o corte vazio.
    fn spread_area(&self) -> Rect {
        let Some(s) = self.session.as_ref() else {
            return Rect::new(Mm(0.0), Mm(0.0), Mm(0.0), Mm(0.0));
        };
        let project = s.project();
        let index = s.current();
        let active = project.content_pages(index);
        let left = project.spec.safe_rect(PageSide::Left);
        let right = project.spec.safe_rect(PageSide::Right);
        let pick = |r: Rect, side: PageSide| {
            let page = project.spec.page_rect(side);
            let (x, w) = if r.x.get() < page.x.get() {
                (r.x.get(), r.right().get() - r.x.get())
            } else {
                (page.x.get(), r.right().get() - page.x.get())
            };
            Rect::new(lamina_core::Mm(x), r.y, lamina_core::Mm(w), r.height)
        };
        match (active.left, active.right) {
            (true, true) => Rect::new(
                lamina_core::Mm(left.x.get().min(right.x.get())),
                lamina_core::Mm(left.y.get().min(right.y.get())),
                lamina_core::Mm(left.width.get() + right.width.get()),
                lamina_core::Mm(left.height.get().max(right.height.get())),
            ),
            (true, false) => pick(left, PageSide::Left),
            (false, true) => pick(right, PageSide::Right),
            (false, false) => left,
        }
    }

    /// "Selecione três quadros ou mais." enquanto Distribuir não vale no modo (vazio quando vale).
    pub fn distribute_hint(&self) -> &'static str {
        if self.organizing() && self.selected_frames().len() < 3 {
            NEED_THREE
        } else {
            ""
        }
    }
}
