//! Quadros à mão pelo editor (spec TX): o gesto de mover ou redimensionar com o ímã, Alt+setas e
//! as medidas digitadas. Quadros novos, apagar e a ordem ficam em `hand_frames.rs`. As contas
//! estão em [`crate::hand`] e [`crate::snap`].

use lamina_core::{Mm, Rect};

use super::Editor;
use crate::hand::{MIN_FRAME_MM, fit_rect, frame_mm, set_rect};
use crate::keys::{Key, Shortcut};
use crate::magnets::{self, Mark, Near};
use crate::snap::{Grip, SnapLine, dragged};

/// O gesto em andamento: o quadro, a alça, o retângulo do começo e o de agora (em mm).
#[derive(Debug, Clone)]
pub(super) struct Gesture {
    frame: usize,
    grip: Grip,
    start: Rect,
    now: Rect,
    lines: Vec<SnapLine>,
    pub(super) marks: Vec<Mark>,
}

impl Editor {
    /// Começa a mover ou redimensionar o quadro `frame` da lâmina atual; ele fica selecionado.
    pub fn frame_grab(&mut self, frame: usize, grip: Grip) -> bool {
        self.exit_crop();
        let Some(s) = &self.session else {
            return false;
        };
        let Some(start) = frame_mm(s.project(), s.current(), frame) else {
            return false;
        };
        self.selected_frame = Some(frame);
        self.gesture = Some(Gesture {
            frame,
            grip,
            start,
            now: start,
            lines: Vec::new(),
            marks: Vec::new(),
        });
        true
    }

    /// Arrasto de `dx`, `dy` mm desde o começo do gesto. Com `magnet`, encaixa nas guias a menos
    /// de `tolerance` mm (8 pixels da tela, convertidos pela janela) e nos ímãs de vão igual e
    /// mesmo tamanho (spec GJ: os ímãs do organizar valem na edição comum).
    pub fn frame_drag(&mut self, dx: f64, dy: f64, magnet: bool, tolerance: f64) -> bool {
        let (Some(s), Some(g)) = (&self.session, &self.gesture) else {
            return false;
        };
        let moved = dragged(g.start, g.grip, dx, dy, MIN_FRAME_MM);
        let (rect, lines, marks) = if magnet {
            let near = Near::of(s.project(), s.current(), &[g.frame]);
            magnets::magnet(moved, g.grip, &near, tolerance)
        } else {
            (moved, Vec::new(), Vec::new())
        };
        // O que a tela mostra já é o que vai ficar: preso ao corte e ao mínimo.
        let fitted = fit_rect(s.project(), rect)
            .map(|r| r.to_absolute(&s.project().spec.trim_rect()))
            .unwrap_or(rect);
        if let Some(g) = self.gesture.as_mut() {
            g.now = fitted;
            g.lines = lines;
            g.marks = marks;
        }
        true
    }

    /// Solta: grava o gesto como uma edição (regra 10). Sem movimento, nada muda. Soltar num
    /// texto mostra o campo dele na aba Quadro, sem tirar o teclado da lâmina (correção TX de
    /// 2026-10-28); pedir só no soltar, porque mudar a tela no meio do gesto solta o mouse.
    pub fn frame_release(&mut self) -> bool {
        let Some(g) = self.gesture.take() else {
            return false;
        };
        self.show_text(super::TextFocus::Tab);
        if g.now == g.start {
            return false;
        }
        let label = match g.grip {
            Grip::Move => "Mover quadro",
            Grip::Resize { .. } => "Redimensionar quadro",
        };
        self.change(label, |p, i| set_rect(p, i, g.frame, g.now))
    }

    /// O quadro do gesto e onde ele está agora (mm), para a tela desenhar.
    pub fn gesture_rect(&self) -> Option<(usize, Rect)> {
        self.gesture.as_ref().map(|g| (g.frame, g.now))
    }

    /// As guias que pegaram no gesto (de um quadro ou do grupo), para as linhas da tela.
    pub fn snap_lines(&self) -> &[SnapLine] {
        if let Some(g) = &self.group {
            return &g.lines;
        }
        self.gesture.as_ref().map_or(&[], |g| g.lines.as_slice())
    }

    /// Um gesto à mão (de um quadro ou do grupo) está em curso: a tela não anima (spec MV).
    pub fn in_gesture(&self) -> bool {
        self.gesture.is_some() || self.group.is_some()
    }

    /// Alt+setas move o quadro selecionado 1 mm; com Shift, 10 mm. Seguidas no mesmo quadro
    /// viram uma edição (regra 10). `false` = não era Alt+seta ou não há quadro.
    pub fn nudge_key(&mut self, key: Shortcut) -> bool {
        if !key.alt || key.ctrl {
            return false;
        }
        let step = if key.shift { 10.0 } else { 1.0 };
        let (dx, dy) = match key.key {
            Key::Left => (-step, 0.0),
            Key::Right => (step, 0.0),
            Key::Up => (0.0, -step),
            Key::Down => (0.0, step),
            _ => return false,
        };
        let (Some(frame), Some(s)) = (self.selected_frame, self.session.as_mut()) else {
            return false;
        };
        let index = s.current();
        let Some(r) = frame_mm(s.project(), index, frame) else {
            return false;
        };
        let to = Rect::new(Mm(r.x.get() + dx), Mm(r.y.get() + dy), r.width, r.height);
        let key = format!("mover:{index}:{frame}");
        s.edit_merging("Mover quadro", &key, |p| set_rect(p, index, frame, to))
            .is_ok()
    }

    /// Põe o quadro selecionado nas medidas digitadas na aba Quadro (mm, regra 12).
    pub fn set_frame_mm(&mut self, rect: Rect) -> bool {
        let Some(frame) = self.selected_frame else {
            return false;
        };
        let Some(s) = &self.session else {
            return false;
        };
        let same_size = frame_mm(s.project(), s.current(), frame).is_some_and(|r| {
            (r.width.get() - rect.width.get()).abs() < 1e-9
                && (r.height.get() - rect.height.get()).abs() < 1e-9
        });
        let label = if same_size {
            "Mover quadro"
        } else {
            "Redimensionar quadro"
        };
        self.change(label, |p, i| set_rect(p, i, frame, rect))
    }
}
