//! Tirar fotos do álbum pela grade (spec TF): uma edição que Ctrl+Z desfaz inteira (regra 6).

use lamina_core::PhotoId;

use super::Editor;
use crate::leaving::{remove_from_album, remove_label, used_in_spreads};

impl Editor {
    /// O arquivo da foto no disco (relativo à pasta do álbum, se o álbum tem arquivo).
    pub fn photo_file(&self, id: PhotoId) -> Option<std::path::PathBuf> {
        let session = self.session.as_ref()?;
        let photo = session.project().photo(id).ok()?;
        Some(match session.file().and_then(|f| f.parent()) {
            Some(dir) => dir.join(&photo.path),
            None => photo.path.clone(),
        })
    }

    /// As fotos dos quadros selecionados na lâmina mostrada, sem repetir (spec TF-B11).
    pub fn selected_frame_photos(&self) -> Vec<PhotoId> {
        let Some(spread) = self
            .session
            .as_ref()
            .and_then(|s| s.project().spreads.get(s.current()))
        else {
            return Vec::new();
        };
        let mut photos = Vec::new();
        for f in self.selected_frames() {
            if let Some(lamina_core::FrameContent::Photo { photo, .. }) =
                spread.frames.get(f).map(|fr| &fr.content)
                && !photos.contains(photo)
            {
                photos.push(*photo);
            }
        }
        photos
    }

    /// Quantas das fotos estão em lâminas (para a pergunta da regra 4).
    pub fn photos_in_spreads(&self, photos: &[PhotoId]) -> usize {
        self.session
            .as_ref()
            .map_or(0, |s| used_in_spreads(s.album(), photos))
    }

    /// Tira as fotos do álbum e das lâminas, numa edição só; a seleção perde essas fotos.
    pub fn remove_from_album(&mut self, photos: &[PhotoId]) -> bool {
        self.leave_cover();
        if photos.is_empty() {
            return false;
        }
        self.exit_crop();
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let label = remove_label(photos.len());
        match session.edit(&label, |p| remove_from_album(p, photos)) {
            Ok(()) => {
                self.selection.retain(|id| !photos.contains(id));
                self.selected_frame = None;
                true
            }
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }
}
