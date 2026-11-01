//! Os campos do texto pela aba Quadro (spec TX, seção 3): cada mudança é uma edição com rótulo,
//! e mudanças seguidas no mesmo campo do mesmo quadro viram uma só (regra 10).

use lamina_core::{LINE_SPACING, Rgb, TEXT_SIZE_PT, TextAlign, TextBox, TextVAlign, clean_text};

use super::Editor;
use crate::hand::update_text;

/// Uma mudança num campo do texto.
#[derive(Debug, Clone, PartialEq)]
pub enum TextChange {
    Text(String),
    Font(String),
    Size(f64),
    Bold(bool),
    Italic(bool),
    Color(Rgb),
    Align(TextAlign),
    VAlign(TextVAlign),
    LineSpacing(f64),
    Rotation(lamina_core::TextRotation),
}

impl TextChange {
    /// Rótulo do histórico e o campo (a chave da emenda).
    fn label(&self) -> (&'static str, &'static str) {
        match self {
            TextChange::Text(_) => ("Texto", "texto"),
            TextChange::Font(_) => ("Fonte", "fonte"),
            TextChange::Bold(_) => ("Fonte", "negrito"),
            TextChange::Italic(_) => ("Fonte", "italico"),
            TextChange::Size(_) => ("Tamanho do texto", "tamanho"),
            TextChange::Color(_) => ("Cor do texto", "cor"),
            TextChange::Align(_) => ("Alinhamento", "alinhamento"),
            TextChange::Rotation(_) => ("Girar texto", "giro"),
            TextChange::VAlign(_) => ("Alinhamento", "vertical"),
            TextChange::LineSpacing(_) => ("Entrelinha", "entrelinha"),
        }
    }

    /// Fora da faixa (tamanho, entrelinha) não muda nada (spec TX, seção 6).
    fn valid(&self) -> bool {
        match self {
            TextChange::Size(v) => (TEXT_SIZE_PT.0..=TEXT_SIZE_PT.1).contains(v),
            TextChange::LineSpacing(v) => (LINE_SPACING.0..=LINE_SPACING.1).contains(v),
            _ => true,
        }
    }

    pub(crate) fn apply(self, t: &mut TextBox) {
        match self {
            TextChange::Text(s) => t.text = clean_text(&s),
            TextChange::Font(f) => t.font = f,
            TextChange::Size(v) => t.size_pt = v,
            TextChange::Bold(b) => t.bold = b,
            TextChange::Italic(b) => t.italic = b,
            TextChange::Color(c) => t.color = c,
            TextChange::Align(a) => t.align = a,
            TextChange::Rotation(r) => t.rotation = r,
            TextChange::VAlign(a) => t.valign = a,
            TextChange::LineSpacing(v) => t.line_spacing = v,
        }
    }
}

impl Editor {
    /// O texto do quadro selecionado, se for um quadro de texto.
    pub fn selected_text(&self) -> Option<&TextBox> {
        match self.selected_content()? {
            lamina_core::FrameContent::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Muda um campo do texto selecionado. `false` = nada mudou (valor fora da faixa, igual ao
    /// de antes ou nenhum texto selecionado).
    pub fn change_text(&mut self, change: TextChange) -> bool {
        if !change.valid() {
            return false;
        }
        let (Some(frame), Some(before)) = (self.selected_frame, self.selected_text().cloned())
        else {
            return false;
        };
        let mut after = before.clone();
        change.clone().apply(&mut after);
        if after == before {
            return false;
        }
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let index = s.current();
        let (label, field) = change.label();
        let key = format!("texto:{index}:{frame}:{field}");
        s.edit_merging(label, &key, |p| {
            update_text(p, index, frame, |t| change.apply(t))
        })
        .is_ok()
    }
}
