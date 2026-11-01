//! Soltar uma foto na faixa Sequência pelo editor (spec SQ): um passo só do histórico, com o
//! aviso da recusa quando a foto não pode ir.

use std::sync::Arc;

use lamina_core::{PhotoId, Project};

use super::Editor;
use crate::EditorError;
use crate::motion::{TIMELINE, TimelineKey, timeline_rows};
use crate::sequence::{SeqSpot, move_in_sequence, sequence_spot, spot_x};

/// O projeto que a Sequência mostra e as linhas dele.
type SequenceRows = (Arc<Project>, Vec<(TimelineKey, usize)>);

impl Editor {
    /// Solta a foto `photo` da lâmina `from` no lugar `to` da Sequência. `true` = mudou; a lâmina
    /// atual continua a mesma.
    pub fn sequence_move(&mut self, from: usize, photo: PhotoId, to: SeqSpot) -> bool {
        let label = if to.spread == from {
            format!("Reordenar fotos · lâmina {}", from + 1)
        } else {
            format!("Mover foto · lâmina {} → {}", from + 1, to.spread + 1)
        };
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let mut moved = false;
        let result = session.edit(&label, |p| {
            moved = move_in_sequence(p, from, photo, to)?;
            if moved {
                Ok(())
            } else {
                Err(EditorError::NoFrame)
            }
        });
        match result {
            Ok(()) => true,
            // Soltou no mesmo lugar: nada a dizer.
            Err(EditorError::NoFrame) => false,
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }

    /// O projeto e as linhas que a Sequência mostra: sempre os do álbum, com a capa aberta ou não.
    pub fn sequence_rows(&self) -> Option<SequenceRows> {
        let project = self.session.as_ref()?.album_shared();
        let rows = timeline_rows(&project);
        Some((project, rows))
    }

    /// O lugar de entrada na Sequência para o ponteiro em `x` (pixels da linha) e o `x` da linha
    /// que o mostra na faixa (regra 2).
    pub fn sequence_target(&self, x: f64) -> Option<(SeqSpot, f64)> {
        let rows = timeline_rows(self.session.as_ref()?.album());
        let spot = sequence_spot(&rows, x, TIMELINE)?;
        Some((spot, spot_x(&rows, spot, TIMELINE)))
    }

    /// Solta a foto da posição `row` da Sequência com o ponteiro em `x` (a faixa só conhece as
    /// posições). `false`: não era uma foto, ou nada mudou.
    pub fn sequence_drop(&mut self, row: usize, x: f64) -> bool {
        self.leave_cover();
        let Some(s) = self.session.as_ref() else {
            return false;
        };
        let rows = timeline_rows(s.project());
        let Some((TimelineKey::Photo(photo, _), from)) = rows.get(row).copied() else {
            return false;
        };
        let Some(spot) = sequence_spot(&rows, x, TIMELINE) else {
            return false;
        };
        self.sequence_move(from, photo, spot)
    }
}
