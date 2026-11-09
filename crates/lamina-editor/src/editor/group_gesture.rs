//! Mover e ampliar o grupo pelo mouse no modo organizar (spec EG, regras 4 a 6 e 10): segurar o
//! corpo de um quadro selecionado ou uma alça da caixa do grupo, arrastar e soltar vira uma
//! edição. As contas estão em [`crate::group`].

use lamina_core::{FrameContent, Mm, Project, Rect};

use super::Editor;
use crate::group::{clamp_shift, group_box, scaled, scaled_text, shifted};
use crate::hand::{MIN_FRAME_MM, frame_mm, set_rect};
use crate::magnets::{self, Mark, Near};
use crate::snap::{Grip, SnapLine};

/// O gesto do grupo: os quadros, a alça, os retângulos do começo e os de agora (mm), os corpos de
/// texto do começo e os fatores de agora.
#[derive(Debug, Clone)]
pub(super) struct GroupGesture {
    frames: Vec<usize>,
    grip: Grip,
    starts: Vec<Rect>,
    now: Vec<Rect>,
    text_pt: Vec<Option<f64>>,
    scale: (f64, f64),
    pub(super) lines: Vec<SnapLine>,
    pub(super) marks: Vec<Mark>,
}

fn text_pt(project: &Project, index: usize, frame: usize) -> Option<f64> {
    match &project.spreads.get(index)?.frames.get(frame)?.content {
        FrameContent::Text(t) => Some(t.size_pt),
        _ => None,
    }
}

impl Editor {
    /// Segura o grupo pelo quadro `frame` (que entra na seleção, sozinho, se estava fora) com a
    /// alça `grip`. `true` = o gesto começou.
    pub fn group_grab(&mut self, frame: usize, grip: Grip) -> bool {
        self.exit_crop();
        if !self.selected_frames().contains(&frame) {
            self.organize_click(frame, false, false);
        }
        let frames = self.selected_frames();
        let all = self.frames_mm();
        let Some(s) = &self.session else {
            return false;
        };
        let (project, index) = (s.project(), s.current());
        let starts: Vec<Rect> = frames
            .iter()
            .filter_map(|f| frame_mm(project, index, *f))
            .collect();
        if starts.is_empty() || starts.len() != frames.len() || all.is_empty() {
            return false;
        }
        let text_pt = frames.iter().map(|f| text_pt(project, index, *f)).collect();
        self.group = Some(GroupGesture {
            frames,
            grip,
            now: starts.clone(),
            starts,
            text_pt,
            scale: (1.0, 1.0),
            lines: Vec::new(),
            marks: Vec::new(),
        });
        true
    }

    /// Arrasto de `dx`, `dy` mm desde o começo. Mover: a caixa encaixa nas guias (sem os quadros do
    /// grupo; `magnet` falso com Alt) e para no corte. Alça: amplia o grupo; `free` (Shift) deforma.
    pub fn group_drag(
        &mut self,
        (dx, dy): (f64, f64),
        magnet: bool,
        tolerance: f64,
        free: bool,
    ) -> bool {
        let (Some(s), Some(g)) = (&self.session, &self.group) else {
            return false;
        };
        let (project, index) = (s.project(), s.current());
        let trim = project.spec.trim_rect();
        let Some(start) = group_box(&g.starts) else {
            return false;
        };
        let near = if magnet {
            Near::of(project, index, &g.frames)
        } else {
            Near::default()
        };
        let (now, scale, lines, marks) = match g.grip {
            Grip::Move => {
                let moved = shifted(&[start], dx, dy)[0];
                let (target, lines, marks) = if magnet {
                    magnets::magnet(moved, Grip::Move, &near, tolerance)
                } else {
                    (moved, Vec::new(), Vec::new())
                };
                let wanted = (
                    target.x.get() - start.x.get(),
                    target.y.get() - start.y.get(),
                );
                let (dx, dy) = clamp_shift(start, trim, wanted.0, wanted.1);
                (shifted(&g.starts, dx, dy), (1.0, 1.0), lines, marks)
            }
            grip => {
                let (rects, sx, sy) = scaled(&g.starts, grip, (dx, dy), free, (trim, MIN_FRAME_MM));
                // Um quadro só: o ímã de mesmo tamanho (spec EG). Vários (spec GJ, conferência do
                // dono 2026-11-07): a caixa do grupo encaixa na altura/largura dos quadros de fora
                // e os quadros do grupo acompanham a escala.
                let alone_rect = alone(&rects, grip, free);
                let group_box = if alone_rect.is_none() {
                    crate::group::group_box(&rects)
                } else {
                    None
                };
                match alone_rect.filter(|_| magnet) {
                    Some(rect) => sized_alone(start, rect, grip, (&near, tolerance), trim)
                        .unwrap_or((rects, (sx, sy), Vec::new(), Vec::new())),
                    None => match group_box.filter(|_| magnet) {
                        Some(box_rect) => {
                            sized_group(&rects, box_rect, grip, (&near, tolerance), trim)
                                .unwrap_or((rects, (sx, sy), Vec::new(), Vec::new()))
                        }
                        None => (rects, (sx, sy), Vec::new(), Vec::new()),
                    },
                }
            }
        };
        if let Some(g) = self.group.as_mut() {
            g.now = now;
            g.scale = scale;
            g.lines = lines;
            g.marks = marks;
        }
        true
    }

    /// Solta: grava o gesto como uma edição ("Mover 3 quadros", "Redimensionar 3 quadros").
    pub fn group_release(&mut self) -> bool {
        let Some(g) = self.group.take() else {
            return false;
        };
        if g.now == g.starts {
            return false;
        }
        let n = g.frames.len();
        let (one, many) = match g.grip {
            Grip::Move => ("Mover quadro", "Mover"),
            Grip::Resize { .. } => ("Redimensionar quadro", "Redimensionar"),
        };
        let label = if n == 1 {
            one.to_owned()
        } else {
            format!("{many} {n} quadros")
        };
        self.change(&label, |p, i| apply_group(p, i, &g))
    }

    /// As marcas do ímã de mesmo tamanho e de espaçamento igual do gesto (de um quadro na edição
    /// comum ou do grupo), para a tela.
    pub fn magnet_marks(&self) -> &[Mark] {
        if let Some(g) = &self.group {
            return &g.marks;
        }
        self.gesture.as_ref().map_or(&[], |g| g.marks.as_slice())
    }

    /// O gesto do grupo está em curso.
    pub fn group_moving(&self) -> bool {
        self.group.is_some()
    }

    /// O projeto com o gesto do grupo aplicado, para a tela desenhar no meio do gesto.
    pub(super) fn apply_group_preview(&self, project: &mut Project, index: usize) {
        if let Some(g) = &self.group {
            // Os retângulos do gesto já estão presos ao corte; erro só deixa como estava.
            let _ = apply_group(project, index, g);
        }
    }
}

/// O quadro sozinho num gesto de tamanho pela borda (ou num canto com Shift): só aí vale o ímã de
/// mesmo tamanho, porque o canto proporcional não muda um eixo só (regra 7).
fn alone(rects: &[Rect], grip: Grip, free: bool) -> Option<Rect> {
    let Grip::Resize {
        left,
        right,
        top,
        bottom,
    } = grip
    else {
        return None;
    };
    let corner = (left || right) && (top || bottom);
    match rects {
        [one] if free || !corner => Some(*one),
        _ => None,
    }
}

/// Os retângulos, os fatores, as linhas e as marcas do quadro sozinho com os ímãs da borda
/// segurada; `None` se o encaixe sairia do corte ou ficaria abaixo do mínimo.
type Sized = (Vec<Rect>, (f64, f64), Vec<SnapLine>, Vec<Mark>);

fn sized_alone(
    start: Rect,
    rect: Rect,
    grip: Grip,
    (near, tolerance): (&Near, f64),
    trim: Rect,
) -> Option<Sized> {
    let (r, lines, marks) = magnets::magnet(rect, grip, near, tolerance);
    let inside = r.x.get() >= trim.x.get() - 1e-9
        && r.y.get() >= trim.y.get() - 1e-9
        && r.right().get() <= trim.right().get() + 1e-9
        && r.bottom().get() <= trim.bottom().get() + 1e-9;
    let big = r.width.get() >= MIN_FRAME_MM && r.height.get() >= MIN_FRAME_MM;
    let scale = (
        r.width.get() / start.width.get(),
        r.height.get() / start.height.get(),
    );
    (inside && big).then(|| (vec![r], scale, lines, marks))
}

/// A caixa do grupo encaixada num quadro de fora (spec GJ): os quadros do grupo são reescalados
/// pela mesma proporção da caixa, a partir do canto oposto ao grip.
fn sized_group(
    rects: &[Rect],
    box_rect: Rect,
    grip: Grip,
    (near, tolerance): (&Near, f64),
    trim: Rect,
) -> Option<Sized> {
    let (r, lines, marks) = magnets::magnet(box_rect, grip, near, tolerance);
    // Sem encaixe (a caixa não mudou de tamanho), deixa o gesto seguir pelo `scaled` normal.
    if (r.width.get() - box_rect.width.get()).abs() < 1e-9
        && (r.height.get() - box_rect.height.get()).abs() < 1e-9
    {
        return None;
    }
    let inside = r.x.get() >= trim.x.get() - 1e-9
        && r.y.get() >= trim.y.get() - 1e-9
        && r.right().get() <= trim.right().get() + 1e-9
        && r.bottom().get() <= trim.bottom().get() + 1e-9;
    if !inside {
        return None;
    }
    let (bw, bh) = (
        box_rect.width.get().max(1e-9),
        box_rect.height.get().max(1e-9),
    );
    let (sx, sy) = (r.width.get() / bw, r.height.get() / bh);
    let Grip::Resize {
        left,
        right: _,
        top,
        bottom: _,
    } = grip
    else {
        return None;
    };
    let (ax, ay) = (
        if left {
            box_rect.right().get()
        } else {
            box_rect.x.get()
        },
        if top {
            box_rect.bottom().get()
        } else {
            box_rect.y.get()
        },
    );
    let out: Vec<Rect> = rects
        .iter()
        .map(|s| {
            Rect::new(
                Mm(ax + (s.x.get() - ax) * sx),
                Mm(ay + (s.y.get() - ay) * sy),
                Mm(s.width.get() * sx),
                Mm(s.height.get() * sy),
            )
        })
        .collect();
    Some((out, (sx, sy), lines, marks))
}

/// Os retângulos e os corpos de texto do gesto no projeto.
fn apply_group(
    project: &mut Project,
    index: usize,
    g: &GroupGesture,
) -> Result<(), crate::EditorError> {
    for (k, (f, r)) in g.frames.iter().zip(&g.now).enumerate() {
        set_rect(project, index, *f, *r)?;
        let Some(start) = g.text_pt.get(k).copied().flatten() else {
            continue;
        };
        if let Some(FrameContent::Text(t)) = project
            .spreads
            .get_mut(index)
            .and_then(|s| s.frames.get_mut(*f))
            .map(|f| &mut f.content)
        {
            t.size_pt = scaled_text(start, g.scale.0, g.scale.1);
        }
    }
    Ok(())
}
