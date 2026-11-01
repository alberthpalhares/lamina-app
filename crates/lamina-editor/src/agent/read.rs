//! O que a IA lê do álbum (spec IA, seção 3): o resumo e uma lâmina com a prévia. Os quadros vêm
//! numerados a partir de 1, na ordem da lâmina, com a posição em mm a partir do corte.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{Frame, FrameContent, Photo, PhotoId, Project};
use lamina_media::{SCREEN_SIZE, load_oriented, to_rgba};
use serde_json::{Value, json};

use super::paths::file_name;
use super::{Agent, ToolError, ToolImage, ToolReply, ViewSpread, session, spread_index};
use crate::{Editor, Session, color_hex, frame_dpi_text, used_photos};

/// Lado maior da prévia de uma lâmina (seção 4).
const PREVIEW_SIDE: u32 = 1024;
/// Folga para dizer de que lado da dobra está um quadro (fração da lâmina).
const SPINE_EPS: f64 = 1e-6;

impl Agent {
    pub(super) fn summary(&self, editor: &Editor) -> Result<ToolReply, ToolError> {
        let s = session(editor)?;
        let p = s.project();
        let trim = p.spec.trim_rect();
        let pages = p.spec.edges().page_count(p.spreads.len());
        let data = json!({
            "nome": s.title(),
            "arquivo": s.file(),
            "especificacao": {
                "lamina_mm": [round(trim.width.get()), round(trim.height.get())],
                "dpi": p.spec.dpi().get(),
            },
            "laminas": p.spreads.len(),
            "paginas": pages,
            "paginas_em_branco": p.spreads.iter().enumerate()
                .filter(|(_, spread)| !spread.blank.is_none())
                .map(|(i, spread)| json!({ "lamina": i + 1, "lados": blank_sides(spread.blank) }))
                .collect::<Vec<_>>(),
            "aviso_de_paginas": s.page_warning(),
            "fotos": {
                "total": p.photos.len(),
                "usadas": used_photos(p),
                "sem_uso": p.photos.len() - used_photos(p),
                "faltando": s.missing().len(),
            },
            "avisos": (0..p.spreads.len()).map(|i| s.warnings_for(i).len()).sum::<usize>(),
            "passos_no_historico": s.history_list().position,
            "alteracoes_nao_salvas": s.is_dirty(),
            "imagens": if self.images { "ligadas" } else { "desligadas" },
            "presets": self.presets.iter().map(crate::Preset::full_name).collect::<Vec<_>>(),
            "pastas_permitidas": self.roots.list(),
        });
        let dirty = if s.is_dirty() {
            "alterações não salvas"
        } else {
            "sem alterações"
        };
        Ok(ToolReply::new(format!("{} · {dirty}.", s.counter()), data))
    }

    pub(super) fn view_spread(
        &self,
        editor: &Editor,
        a: &ViewSpread,
    ) -> Result<ToolReply, ToolError> {
        let s = session(editor)?;
        let index = spread_index(s.project(), a.lamina)?;
        let frames = s.project().spreads[index].frames.len();
        let mut reply = ToolReply::new(
            format!("Lâmina {} com {frames} quadros.", a.lamina),
            spread_data(s, index),
        );
        if a.imagem && self.images {
            match self.preview(s.project(), index, PREVIEW_SIDE) {
                Ok(image) => reply.images.push(image),
                Err(e) => reply.text.push_str(&format!(" A prévia não saiu: {e}")),
            }
        }
        Ok(reply)
    }

    /// A prévia da lâmina `index` em PNG (lado maior `side`).
    pub(super) fn preview(
        &self,
        project: &Project,
        index: usize,
        side: u32,
    ) -> Result<ToolImage, ToolError> {
        let source = self.photo_source(project);
        let bytes = lamina_export::preview_png(project, index, side, &source)?;
        Ok(ToolImage {
            mime: "image/png",
            bytes,
        })
    }

    /// As imagens de tela do cache quando bastam para a prévia; senão, o original.
    fn photo_source(
        &self,
        project: &Project,
    ) -> impl Fn(&Photo, (u32, u32)) -> Result<Arc<RgbaImage>, String> + Sync + use<> {
        let screens: HashMap<PhotoId, PathBuf> = project
            .photos
            .iter()
            .filter_map(|p| self.cache.screen(p).map(|path| (p.id, path)))
            .collect();
        move |photo: &Photo, needed: (u32, u32)| {
            if needed.0.max(needed.1) <= SCREEN_SIZE
                && let Some(Ok(screen)) = screens.get(&photo.id).map(image::open)
            {
                return Ok(Arc::new(to_rgba(screen)));
            }
            load_oriented(&photo.path)
                .map(Arc::new)
                .map_err(|e| e.to_string())
        }
    }
}

/// Os dados da lâmina `index`: fundo, espaço, avisos e os quadros (seção 3).
/// Os lados em branco de uma lâmina, como a IA os escreve (spec PB, regra 12).
fn blank_sides(blank: lamina_core::BlankPages) -> Vec<&'static str> {
    [(blank.left, "esquerda"), (blank.right, "direita")]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, side)| side)
        .collect()
}

pub(super) fn spread_data(s: &Session, index: usize) -> Value {
    let p = s.project();
    let spread = &p.spreads[index];
    let frames: Vec<Value> = spread
        .frames
        .iter()
        .enumerate()
        .map(|(i, f)| frame_data(p, index, i, f))
        .collect();
    let warnings: Vec<String> = s
        .warnings_for(index)
        .iter()
        .map(ToString::to_string)
        .collect();
    json!({
        "numero": index + 1,
        "id": spread.id.get(),
        "fundo": color_hex(spread.background),
        "fundo_direita": spread.background_right.map(color_hex),
        "espaco_mm": round(p.spread_gap(spread).get()),
        "a_mao": spread.manual,
        "modelo": spread.model,
        "paginas_em_branco": blank_sides(spread.blank),
        "avisos": warnings,
        "quadros": frames,
    })
}

fn frame_data(p: &Project, index: usize, i: usize, f: &Frame) -> Value {
    let (trim, r) = (p.spec.trim_rect(), p.frame_rect(f));
    let page = if f.rect.x() + f.rect.width() <= 0.5 + SPINE_EPS {
        "esquerda"
    } else if f.rect.x() >= 0.5 - SPINE_EPS {
        "direita"
    } else {
        "atravessa a dobra"
    };
    let mut data = json!({
        "numero": i + 1,
        "pagina": page,
        "posicao_mm": {
            "x": round(r.x.get() - trim.x.get()),
            "y": round(r.y.get() - trim.y.get()),
            "largura": round(r.width.get()),
            "altura": round(r.height.get()),
        },
        "borda_mm": round(f.border.width.get()),
        "cantos_mm": round(f.radius.get()),
        "opacidade": f.opacity.get(),
    });
    let extra = match &f.content {
        FrameContent::Empty => json!({ "tipo": "vazio" }),
        FrameContent::Photo { photo, crop } => {
            let name = p
                .photo(*photo)
                .map(|ph| file_name(&ph.path))
                .unwrap_or_default();
            let dpi = frame_dpi_text(p, index, i);
            json!({
                "tipo": "foto",
                "foto": { "numero": photo.get(), "nome": name },
                "recorte": { "zoom": crop.zoom, "centro_x": crop.center_x, "centro_y": crop.center_y },
                "dpi_efetivo": dpi.as_ref().map(|d| d.0.clone()),
                "ampliada_demais": dpi.is_some_and(|d| d.1),
            })
        }
        FrameContent::Text(t) => json!({
            "tipo": "texto",
            "texto": t.text,
            "fonte": t.font,
            "tamanho_pt": t.size_pt,
        }),
    };
    if let (Value::Object(base), Value::Object(more)) = (&mut data, extra) {
        base.extend(more);
    }
    data
}

/// Milímetros com uma casa: o bastante para a IA, sem ruído de ponto flutuante.
fn round(mm: f64) -> f64 {
    (mm * 10.0).round() / 10.0
}
