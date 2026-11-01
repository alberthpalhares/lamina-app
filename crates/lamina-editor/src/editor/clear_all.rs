//! Limpar as lâminas (spec LL): tirar as fotos de todas as lâminas, com os quadros vazios no
//! lugar, ou apagar todas as lâminas. Cada uma é um passo só do histórico; a janela pergunta antes.

use std::collections::BTreeSet;

use lamina_core::{FrameContent, Project};

use super::Editor;

/// As fotos diferentes nos quadros do álbum.
fn placed(project: &Project) -> BTreeSet<lamina_core::PhotoId> {
    project
        .spreads
        .iter()
        .flat_map(|s| &s.frames)
        .filter_map(|f| match f.content {
            FrameContent::Photo { photo, .. } => Some(photo),
            _ => None,
        })
        .collect()
}

impl Editor {
    /// Quantas fotos diferentes estão nos quadros e quantas lâminas o álbum tem, para a pergunta
    /// da janela ("Tirar as 48 fotos de todas as 12 lâminas?").
    pub fn clear_counts(&self) -> (usize, usize) {
        self.session.as_ref().map_or((0, 0), |s| {
            (placed(s.album()).len(), s.album().spreads.len())
        })
    }

    /// Tirar as fotos: só o conteúdo dos quadros de foto sai; quadros, textos e o resto ficam.
    pub(super) fn clear_all_photos(&mut self) -> bool {
        self.exit_crop();
        self.selected_frame = None;
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        s.edit("Tirar as fotos · todas as lâminas", |p| {
            for f in p.spreads.iter_mut().flat_map(|s| &mut s.frames) {
                if matches!(f.content, FrameContent::Photo { .. }) {
                    f.content = FrameContent::Empty;
                }
            }
            Ok::<(), crate::EditorError>(())
        })
        .is_ok()
    }

    /// Apagar todas as lâminas: o álbum fica como o de "Começar sem fotos" (TR, regra 10), com as
    /// fotos importadas, todas não usadas.
    pub(super) fn delete_all_spreads(&mut self) -> bool {
        self.exit_crop();
        self.selected_frame = None;
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let done = s
            .edit("Apagar · todas as lâminas", |p| {
                p.spreads.clear();
                Ok::<(), crate::EditorError>(())
            })
            .is_ok();
        if done {
            s.rewind();
        }
        done
    }
}
