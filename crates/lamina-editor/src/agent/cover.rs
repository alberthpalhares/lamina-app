//! A capa pela IA (spec CA, regra 15): `criar_capa`, `medidas_da_capa` e o alvo `capa: true`. Com o
//! alvo, a capa fica aberta e a ferramenta vale nela como na lâmina 1 da projeção; sem ele, a
//! ferramenta vale no álbum.

use lamina_core::{CoverMeasures, Mm, Spine};

use super::calls::CoverCall;
use super::{Agent, ToolCall, ToolError, ToolReply, edit_on, photo_id, session, session_mut};
use crate::Editor;
use crate::editor::cover::{CoverZone, fill_zone};

/// O alvo da ferramenta: abre a capa, ou fecha antes de valer no álbum. Recusa `capa` sem capa e
/// `capa` junto com `lamina`, sem passo.
pub(super) fn prepare(editor: &mut Editor, mut call: ToolCall) -> Result<ToolCall, ToolError> {
    let target = match &mut call {
        ToolCall::ViewSpread(a) => Some((a.capa, &mut a.lamina)),
        ToolCall::PlacePhoto(a) => Some((a.capa, &mut a.lamina)),
        ToolCall::AddText(a) => Some((a.capa, &mut a.lamina)),
        ToolCall::EditText(a) => Some((a.capa, &mut a.lamina)),
        ToolCall::Look(a) => Some((a.capa, &mut a.lamina)),
        _ => None,
    };
    let wants = match target {
        Some((true, lamina)) => {
            if *lamina != 0 {
                return Err(ToolError::CoverAndSpread);
            }
            *lamina = 1;
            true
        }
        _ => matches!(&call, ToolCall::BuildSpread(a) if a.capa),
    };
    if wants {
        session(editor)?;
        if !editor.on_cover() && !editor.go_to_cover() {
            return Err(ToolError::NoCover);
        }
    } else if !matches!(call, ToolCall::CreateCover(_) | ToolCall::CoverMeasures(_)) {
        editor.leave_cover();
    }
    Ok(call)
}

/// A ferramenta só lê o álbum: a capa aberta volta a abrir depois dela.
pub(super) fn only_reads(call: &ToolCall) -> bool {
    matches!(
        call,
        ToolCall::Summary(_)
            | ToolCall::ListPhotos(_)
            | ToolCall::ViewPhotos(_)
            | ToolCall::Preflight(_)
            | ToolCall::History(_)
    )
}

/// As medidas pedidas por cima de `base`: `lombada_cm` fixa a lombada, e `folha_mm` a calcula.
fn merged(base: CoverMeasures, a: &CoverCall, pages: usize) -> CoverMeasures {
    let cm = |v: f64| Mm(v * 10.0);
    let mut m = base;
    if let Some(v) = a.face_largura_cm {
        m.face_width = cm(v);
    }
    if let Some(v) = a.face_altura_cm {
        m.face_height = cm(v);
    }
    if let Some(v) = a.dobra_cm {
        m.wrap = cm(v);
    }
    if let Some(v) = a.zona_segura_cm {
        m.safe = cm(v);
    }
    if let Some(v) = a.folha_mm {
        m.spine = Spine::Auto { sheet: Mm(v) };
    }
    if let Some(v) = a.lombada_cm {
        m.spine = Spine::Fixed { mm: cm(v), pages };
    }
    m
}

impl Agent {
    pub(super) fn create_cover(
        &mut self,
        editor: &mut Editor,
        a: &CoverCall,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.album();
        let measures = merged(CoverMeasures::defaults(&p.spec), a, p.page_count());
        session_mut(editor)?.edit_album("Criar a capa", |p| p.create_cover(measures))?;
        editor.go_to_cover();
        self.edited(editor, 0, "Capa criada, vazia.")
    }

    pub(super) fn cover_measures(
        &mut self,
        editor: &mut Editor,
        a: &CoverCall,
    ) -> Result<ToolReply, ToolError> {
        let p = session(editor)?.album();
        let Some(cover) = &p.cover else {
            return Err(ToolError::NoCover);
        };
        let measures = merged(cover.measures, a, p.page_count());
        measures.validate().map_err(crate::EditorError::Core)?;
        let guide = cover.guide.clone();
        editor.set_cover_measures(measures, guide);
        editor.go_to_cover();
        self.edited(editor, 0, "Medidas da capa trocadas.")
    }

    /// `colocar_foto` numa capa vazia: a foto ocupa a capa inteira, até a dobra.
    pub(super) fn place_on_empty_cover(
        &mut self,
        editor: &mut Editor,
        foto: u64,
    ) -> Result<Option<ToolReply>, ToolError> {
        let s = session(editor)?;
        if !s.on_cover() || !s.project().spreads[0].frames.is_empty() {
            return Ok(None);
        }
        let photo = photo_id(s.project(), foto)?;
        edit_on(editor, 0, "Colocar foto", |p| {
            fill_zone(p, CoverZone::Spine, photo)
        })?;
        self.edited(editor, 0, "Foto colocada na capa.").map(Some)
    }
}
