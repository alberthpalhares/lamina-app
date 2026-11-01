//! A área de cópia e o Delete em camadas (spec EG, regras 11 e 12). Copiar guarda os quadros
//! selecionados; colar em outra lâmina os põe na mesma posição e, na mesma lâmina, 5 mm
//! deslocados; Duplicar cola na hora sem mexer na área de cópia. A área de cópia não vai para o
//! arquivo.

use lamina_core::{Frame, FrameContent, FrameId, Mm, Project, Rect, SpreadId};

use super::Editor;
use crate::EditorError;
use crate::edits::remove_photos;
use crate::hand::{fit_rect, remove_frame};

/// Quanto o colado na mesma lâmina sai deslocado, em mm (regra 12).
pub const PASTE_SHIFT_MM: f64 = 5.0;

/// O que foi copiado: a lâmina de onde veio e os quadros (com conteúdo, borda e cantos).
#[derive(Debug, Clone)]
pub(super) struct Clip {
    spread: SpreadId,
    frames: Vec<Frame>,
}

const FRAMES: (&str, &str) = ("quadro", "quadros");
const PHOTOS: (&str, &str) = ("foto", "fotos");

/// "Colar quadro" com um; "Colar 2 quadros" com mais.
fn counted(verb: &str, (one, many): (&str, &str), n: usize) -> String {
    if n == 1 {
        format!("{verb} {one}")
    } else {
        format!("{verb} {n} {many}")
    }
}

/// Os quadros colados na lâmina `index`, deslocados `shift` mm; os ids novos.
fn paste_into(
    project: &mut Project,
    index: usize,
    frames: &[Frame],
    shift: f64,
) -> Result<Vec<FrameId>, EditorError> {
    let spread = project.spreads.get(index).ok_or(EditorError::NoFrame)?.id;
    let mut ids = Vec::new();
    for f in frames {
        let r = project.frame_rect(f);
        let moved = Rect::new(
            Mm(r.x.get() + shift),
            Mm(r.y.get() + shift),
            r.width,
            r.height,
        );
        let rel = fit_rect(project, moved)?;
        if !matches!(f.content, FrameContent::Text(_)) {
            crate::blank::check_photo_rect(project, index, &rel)?;
        }
        let id = project.add_frame(spread, rel, f.content.clone())?;
        if let Some(new) = project.spreads[index].frames.last_mut() {
            new.border = f.border;
            new.radius = f.radius;
            new.opacity = f.opacity;
        }
        ids.push(id);
    }
    if frames
        .iter()
        .any(|f| !matches!(f.content, FrameContent::Text(_)))
    {
        project.spreads[index].manual = true;
        project.spreads[index].model = None;
    }
    Ok(ids)
}

impl Editor {
    /// Os quadros selecionados (no modo, todos; fora, o de sempre), como estão na lâmina.
    fn picked_frames(&self) -> Option<(SpreadId, Vec<Frame>)> {
        let s = self.session.as_ref()?;
        let spread = s.project().spreads.get(s.current())?;
        let frames: Vec<Frame> = self
            .selected_frames()
            .iter()
            .filter_map(|f| spread.frames.get(*f).cloned())
            .collect();
        (!frames.is_empty()).then_some((spread.id, frames))
    }

    /// Ctrl+C: guarda os quadros selecionados na área de cópia. Não é uma edição.
    pub(super) fn copy_frames(&mut self) -> bool {
        let Some((spread, frames)) = self.picked_frames() else {
            return false;
        };
        self.clipboard = Some(Clip { spread, frames });
        true
    }

    /// Há algo na área de cópia?
    pub fn can_paste(&self) -> bool {
        self.session.is_some() && self.clipboard.is_some()
    }

    /// Ctrl+V: cola a área de cópia na lâmina atual; os colados ficam selecionados.
    pub(super) fn paste_frames(&mut self) -> bool {
        let (Some(clip), Some(s)) = (self.clipboard.clone(), &self.session) else {
            return false;
        };
        let same = s.project().spreads.get(s.current()).map(|sp| sp.id) == Some(clip.spread);
        let shift = if same { PASTE_SHIFT_MM } else { 0.0 };
        self.place(
            &clip.frames,
            shift,
            counted("Colar", FRAMES, clip.frames.len()),
        )
    }

    /// Ctrl+D: os quadros selecionados de novo, 5 mm deslocados, sem mexer na área de cópia.
    pub(super) fn duplicate_frames(&mut self) -> bool {
        let Some((_, frames)) = self.picked_frames() else {
            return false;
        };
        let label = counted("Duplicar", FRAMES, frames.len());
        self.place(&frames, PASTE_SHIFT_MM, label)
    }

    fn place(&mut self, frames: &[Frame], shift: f64, label: String) -> bool {
        let mut ids = Vec::new();
        let done = self.change(&label, |p, i| {
            ids = paste_into(p, i, frames, shift)?;
            Ok(())
        });
        if done {
            self.select_ids(ids);
        }
        done
    }

    /// Delete no modo, em camadas: com fotos na seleção, tira as fotos com os quadros e refaz o
    /// layout ("Tirar 2 fotos"); sem
    /// fotos, apaga os quadros ("Apagar 3 quadros").
    pub(super) fn delete_group(&mut self) -> bool {
        let frames = self.selected_frames();
        let Some((_, picked)) = self.picked_frames() else {
            return false;
        };
        let photos: Vec<usize> = frames
            .iter()
            .zip(&picked)
            .filter(|(_, f)| matches!(f.content, FrameContent::Photo { .. }))
            .map(|(i, _)| *i)
            .collect();
        if !photos.is_empty() {
            let label = counted("Tirar", PHOTOS, photos.len());
            let done = self.change(&label, |p, i| remove_photos(p, i, &photos));
            if done {
                self.clear_frames();
            }
            return done;
        }
        let label = counted("Apagar", FRAMES, frames.len());
        let done = self.change(&label, |p, i| {
            frames.iter().rev().try_for_each(|f| remove_frame(p, i, *f))
        });
        if done {
            self.clear_frames();
        }
        done
    }
}
