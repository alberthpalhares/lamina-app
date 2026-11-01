//! O Novo projeto em três etapas (spec NP): 1, Tamanho; 2, Montagem e aparência; 3, Fotos e
//! arquivo. A janela só mostra estes dados; voltar não perde nada (regra 10).

use lamina_core::{AlbumLook, DEFAULT_GAP, Mm, Mounting};
use lamina_layout::Reuse;

mod choices;
mod colors;
mod create;
mod formats;
mod preview;
mod size;
mod units;

pub use choices::{
    ADD_BINDER, ALL_BINDERS, advanced_error, binder_list, binder_or_all, binders, clean_binder,
    is_advanced, last_preset, matching_preset, short_summary, size_choices, size_labels,
};
pub use colors::picker_color;
pub use create::{
    CANNOT_WRITE, NO_PHOTOS_HERE, PICK_PHOTOS, StepThreeState, apply_look, can_write,
    documents_dir, empty_album, free_file, put_style, suggested_folder,
};
pub use formats::{Format, formats};
pub use preview::{Drawing, PREVIEW_SIDE, drawing, mount_preview};
pub use size::{SAFE_TOO_BIG, SIZE_INVALID, SizeField, SizeForm};
pub use units::{NOT_A_NUMBER, Unit};

/// A montagem e a aparência do álbum novo (etapa 2), que vão para o projeto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlbumStyle {
    pub mounting: Mounting,
    pub look: AlbumLook,
    /// Espaço entre fotos do álbum.
    pub gap: Mm,
}

impl Default for AlbumStyle {
    /// O de sempre: montagem livre, fundo branco, sem borda, 8 pt entre fotos.
    fn default() -> Self {
        AlbumStyle {
            mounting: Mounting::default(),
            look: AlbumLook::default(),
            gap: DEFAULT_GAP,
        }
    }
}

/// As três etapas e o que cada uma guarda até Criar ou Cancelar.
#[derive(Debug, Clone, PartialEq)]
pub struct NewProject {
    /// 1, 2 ou 3.
    pub step: u8,
    pub size: SizeForm,
    pub mounting: Mounting,
    /// Fotos podem atravessar a dobra (o `DesignMode` da especificação).
    pub lay_flat: bool,
    pub reuse: Reuse,
    pub look: AlbumLook,
    pub gap: Mm,
}

impl NewProject {
    pub fn new(size: SizeForm) -> Self {
        let style = AlbumStyle::default();
        NewProject {
            step: 1,
            size,
            mounting: style.mounting,
            lay_flat: true,
            reuse: Reuse::Medium,
            look: style.look,
            gap: style.gap,
        }
    }

    /// Próximo: só com a etapa sem erro, e não passa da 3. `true` = avançou.
    pub fn advance(&mut self) -> bool {
        if self.step >= 3 || (self.step == 1 && self.size.error().is_some()) {
            return false;
        }
        self.step += 1;
        true
    }

    /// Voltar: a etapa anterior, com tudo o que foi digitado.
    pub fn back(&mut self) {
        self.step = self.step.saturating_sub(1).max(1);
    }

    /// A montagem e a aparência escolhidas, para o projeto.
    pub fn style(&self) -> AlbumStyle {
        AlbumStyle {
            mounting: self.mounting,
            look: self.look,
            gap: self.gap,
        }
    }
}
