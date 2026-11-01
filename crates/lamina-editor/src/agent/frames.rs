//! Fotos nos quadros pela IA (spec IA, regra 4; spec FQ, regras 1 a 3): colocar, trocar e tirar.
//! Quadro de texto recusa, para a IA não apagar um título por engano.

use super::{
    Agent, FrameArg, PlacePhoto, SwapPhotos, ToolError, ToolReply, edit_on, photo_frame, photo_id,
    session, spread_index,
};
use crate::Editor;
use crate::edits::{place_photo, remove_photos, spread_photos, swap_photos};

impl Agent {
    pub(super) fn place(
        &mut self,
        editor: &mut Editor,
        a: &PlacePhoto,
    ) -> Result<ToolReply, ToolError> {
        if let Some(reply) = self.place_on_empty_cover(editor, a.foto)? {
            return Ok(reply);
        }
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let frame = photo_frame(p, index, a.quadro)?;
        let photo = photo_id(p, a.foto)?;
        edit_on(editor, index, "Colocar foto", |p| {
            place_photo(p, index, frame, photo)
        })?;
        let text = format!(
            "Foto {} no quadro {} da lâmina {}.",
            a.foto, a.quadro, a.lamina
        );
        self.edited(editor, index, &text)
    }

    pub(super) fn swap(
        &mut self,
        editor: &mut Editor,
        a: &SwapPhotos,
    ) -> Result<ToolReply, ToolError> {
        if let Some(b) = a.lamina_b.filter(|b| *b != a.lamina) {
            return self.swap_across(editor, (a.lamina, a.quadro_a), (b, a.quadro_b));
        }
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let first = photo_frame(p, index, a.quadro_a)?;
        let second = photo_frame(p, index, a.quadro_b)?;
        if first == second {
            return Err(ToolError::SameFrame);
        }
        edit_on(editor, index, "Trocar fotos", |p| {
            swap_photos(p, index, first, second)
        })?;
        let text = format!(
            "Fotos dos quadros {} e {} da lâmina {} trocadas.",
            a.quadro_a, a.quadro_b, a.lamina
        );
        self.edited(editor, index, &text)
    }

    pub(super) fn clear(
        &mut self,
        editor: &mut Editor,
        a: &FrameArg,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let frame = photo_frame(p, index, a.quadro)?;
        edit_on(editor, index, "Tirar foto", |p| {
            remove_photos(p, index, &[frame])
        })?;
        let left = spread_photos(session(editor)?.project(), index).len();
        let text = format!(
            "Foto do quadro {} tirada; a lâmina {} ficou com {left} foto(s), em layout novo.",
            a.quadro, a.lamina
        );
        self.edited(editor, index, &text)
    }
}
