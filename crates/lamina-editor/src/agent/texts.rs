//! Textos pela IA (spec IA, `adicionar_texto` e `editar_texto`; spec TX): o quadro de texto novo
//! da tecla T e os campos da aba Quadro. Cada chamada é uma edição só, com tudo o que pediu.

use lamina_core::{
    FrameContent, LINE_SPACING, Mm, PageSide, Project, Rect, Rgb, TEXT_SIZE_PT, TextAlign, TextBox,
    TextVAlign,
};
use lamina_render::font_styles;
use serde_json::json;

use super::values::{color, font, within};
use super::{
    AddText, Agent, AlignArg, EditText, PageArg, PlaceMm, ToolError, ToolReply, VAlignArg, edit_on,
    frame_index, session, spread_index,
};
use crate::hand::{add_text, frame_mm, page_for, set_rect, update_text};
use crate::{Editor, EditorError, TextChange};

const SIZE: &str = "O tamanho vai de 4 a 300 pt.";
const SPACING: &str = "A entrelinha vai de 0,8 a 3,0.";
const NOTHING: &str = "Diga o que mudar: texto, fonte, tamanho, negrito, italico, cor, \
                       alinhamento, vertical, entrelinha ou posicao_mm.";

/// As mudanças pedidas, conferidas como na aba Quadro, na ordem dos campos dela. `now` é o texto
/// de agora (a fonte dele decide se há negrito e itálico).
fn changes(a: &EditText, now: &TextBox) -> Result<Vec<TextChange>, ToolError> {
    let mut out = Vec::new();
    if let Some(t) = &a.texto {
        out.push(TextChange::Text(t.clone()));
    }
    let font_name = match &a.fonte {
        Some(f) => {
            let f = font(f)?;
            out.push(TextChange::Font(f.clone()));
            f
        }
        None => now.font.clone(),
    };
    if let Some(v) = a.tamanho {
        out.push(TextChange::Size(within(v, TEXT_SIZE_PT, SIZE)?));
    }
    styles(a, &font_name, &mut out)?;
    if let Some(c) = &a.cor {
        out.push(TextChange::Color(color(c)?));
    }
    if let Some(al) = a.alinhamento {
        out.push(TextChange::Align(match al {
            AlignArg::Esquerda => TextAlign::Left,
            AlignArg::Centro => TextAlign::Center,
            AlignArg::Direita => TextAlign::Right,
        }));
    }
    if let Some(v) = a.vertical {
        out.push(TextChange::VAlign(match v {
            VAlignArg::Topo => TextVAlign::Top,
            VAlignArg::Meio => TextVAlign::Middle,
            VAlignArg::Base => TextVAlign::Bottom,
        }));
    }
    if let Some(v) = a.entrelinha {
        out.push(TextChange::LineSpacing(within(v, LINE_SPACING, SPACING)?));
    }
    Ok(out)
}

/// Negrito e itálico só onde a fonte tem (na janela, a caixa fica desligada).
fn styles(a: &EditText, font_name: &str, out: &mut Vec<TextChange>) -> Result<(), ToolError> {
    let fam = font_styles(font_name);
    let refuse = |style| ToolError::NoStyle {
        font: font_name.to_owned(),
        style,
    };
    if let Some(b) = a.negrito {
        if b && fam.is_some_and(|f| !f.bold) {
            return Err(refuse("negrito"));
        }
        out.push(TextChange::Bold(b));
    }
    if let Some(i) = a.italico {
        if i && fam.is_some_and(|f| !f.italic) {
            return Err(refuse("itálico"));
        }
        out.push(TextChange::Italic(i));
    }
    Ok(())
}

/// O lugar pedido em mm a partir do corte; largura e altura que faltam vêm do quadro.
fn place(p: &Project, index: usize, frame: usize, at: PlaceMm) -> Result<Rect, EditorError> {
    let now = frame_mm(p, index, frame).ok_or(EditorError::NoFrame)?;
    let trim = p.spec.trim_rect();
    Ok(Rect::new(
        Mm(trim.x.get() + at.x),
        Mm(trim.y.get() + at.y),
        at.largura.map_or(now.width, Mm),
        at.altura.map_or(now.height, Mm),
    ))
}

impl Agent {
    pub(super) fn add_text(
        &mut self,
        editor: &mut Editor,
        a: &AddText,
    ) -> Result<ToolReply, ToolError> {
        let index = spread_index(session(editor)?.project(), a.lamina)?;
        let asked = EditText {
            capa: false,
            lamina: a.lamina,
            quadro: 0,
            texto: Some(a.texto.clone()),
            fonte: a.fonte.clone(),
            tamanho: a.tamanho,
            negrito: None,
            italico: None,
            cor: None,
            alinhamento: None,
            vertical: None,
            entrelinha: None,
            posicao_mm: a.posicao_mm,
        };
        let changes = changes(&asked, &TextBox::new(Rgb::WHITE))?;
        let side = match a.pagina {
            PageArg::Esquerda => PageSide::Left,
            PageArg::Direita => PageSide::Right,
        };
        let mut at = 0;
        edit_on(editor, index, "Novo texto", |p| {
            at = add_text(p, index, page_for(p, index, Some(side)))?;
            update_text(p, index, at, |t| {
                changes.into_iter().for_each(|c| c.apply(t))
            })?;
            if let Some(pl) = a.posicao_mm {
                let r = place(p, index, at, pl)?;
                set_rect(p, index, at, r)?;
            }
            Ok(())
        })?;
        let text = format!("Texto novo no quadro {} da lâmina {}.", at + 1, a.lamina);
        let mut reply = self.edited(editor, index, &text)?;
        reply.data["quadro"] = json!(at + 1);
        Ok(reply)
    }

    pub(super) fn edit_text(
        &mut self,
        editor: &mut Editor,
        a: &EditText,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.project();
        let index = spread_index(p, a.lamina)?;
        let frame = frame_index(p, index, a.quadro)?;
        let FrameContent::Text(before) = &p.spreads[index].frames[frame].content else {
            return Err(ToolError::NotText {
                spread: a.lamina,
                frame: a.quadro,
            });
        };
        let changes = changes(a, before)?;
        if changes.is_empty() && a.posicao_mm.is_none() {
            return Err(ToolError::NothingAsked(NOTHING));
        }
        let mut after = before.clone();
        changes.into_iter().for_each(|c| c.apply(&mut after));
        let rect = a
            .posicao_mm
            .map(|pl| place(p, index, frame, pl))
            .transpose()?;
        if after == *before && rect.is_none() {
            editor.go_to_spread(index);
            return self.edited(editor, index, "O texto já estava assim; nada mudou.");
        }
        edit_on(editor, index, "Editar texto", |p| {
            update_text(p, index, frame, |t| *t = after)?;
            if let Some(r) = rect {
                set_rect(p, index, frame, r)?;
            }
            Ok(())
        })?;
        let text = format!(
            "Texto do quadro {} da lâmina {} mudado.",
            a.quadro, a.lamina
        );
        self.edited(editor, index, &text)
    }
}
