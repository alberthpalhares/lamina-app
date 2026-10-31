//! Listas que a IA lê (spec IA, seção 3): as fotos, as miniaturas, a pré-impressão, o histórico e
//! os comandos do registro.

use lamina_core::PhotoId;
use serde_json::{Value, json};

use super::paths::file_name;
use super::{Agent, PhotoFilter, ToolError, ToolImage, ToolReply, ViewPhotos, photo_id, session};
use crate::edits::timeline;
use crate::{Command, Editor, Filter, Menu, photo_info, photo_list};

/// Até quantas miniaturas numa resposta (regra 9).
pub(super) const MAX_VIEW_PHOTOS: usize = 24;

/// As fotos do filtro, em ordem de captura, com as lâminas onde aparecem.
pub(super) fn list_photos(editor: &Editor, filter: PhotoFilter) -> Result<ToolReply, ToolError> {
    let s = session(editor)?;
    let p = s.project();
    let used_in = timeline(p);
    let spreads_of = |id: PhotoId| -> Vec<usize> {
        let at = used_in
            .iter()
            .enumerate()
            .filter(|(_, photos)| photos.contains(&id));
        at.map(|(i, _)| i + 1).collect()
    };
    let photos: Vec<Value> = photo_list(p, Filter::All)
        .into_iter()
        .filter(|item| match filter {
            PhotoFilter::Todas => true,
            PhotoFilter::Usadas => item.uses > 0,
            PhotoFilter::SemUso => item.uses == 0,
            PhotoFilter::Faltando => s.missing().contains(&item.id),
        })
        .filter_map(|item| {
            let info = photo_info(p, item.id)?;
            Some(json!({
                "numero": item.id.get(),
                "nome": item.name,
                "data": info.taken_at,
                "camera": info.camera,
                "largura_px": info.width,
                "altura_px": info.height,
                "laminas": spreads_of(item.id),
                "faltando": s.missing().contains(&item.id),
            }))
        })
        .collect();
    let text = format!("{} fotos.", photos.len());
    Ok(ToolReply::new(text, json!({ "fotos": photos })))
}

impl Agent {
    /// As miniaturas das fotos pedidas, na ordem pedida (regras 8 e 9).
    pub(super) fn view_photos(
        &self,
        editor: &Editor,
        a: &ViewPhotos,
    ) -> Result<ToolReply, ToolError> {
        if !self.images {
            return Err(ToolError::ImagesOff);
        }
        if a.fotos.is_empty() {
            return Err(ToolError::NoPhotosChosen);
        }
        if a.fotos.len() > MAX_VIEW_PHOTOS {
            return Err(ToolError::TooManyPhotos(MAX_VIEW_PHOTOS));
        }
        let p = session(editor)?.project();
        let mut reply = ToolReply::new(String::new(), Value::Null);
        let mut listed = Vec::new();
        for &n in &a.fotos {
            let photo = p.photo(photo_id(p, n)?)?;
            let thumb = self.cache.thumbnail(photo).or_else(|| {
                self.cache.ensure_images(photo).ok()?;
                self.cache.thumbnail(photo)
            });
            let bytes = thumb.and_then(|t| std::fs::read(t).ok());
            listed.push(json!({ "numero": n, "nome": file_name(&photo.path), "miniatura": bytes.is_some() }));
            if let Some(bytes) = bytes {
                reply.images.push(ToolImage {
                    mime: "image/jpeg",
                    bytes,
                });
            }
        }
        reply.text = format!(
            "Miniaturas de {} fotos, na ordem pedida.",
            reply.images.len()
        );
        reply.data = json!({ "fotos": listed });
        Ok(reply)
    }
}

/// Todos os avisos de pré-impressão, com lâmina e quadro, nas frases da janela; os da capa vêm
/// depois dos das lâminas (spec CA, CA-B11).
pub(super) fn preflight(editor: &Editor) -> Result<ToolReply, ToolError> {
    let s = session(editor)?;
    let spreads = (0..s.album().spreads.len())
        .flat_map(|i| s.spread_warnings(i))
        .map(|w| json!({ "lamina": w.spread, "quadro": w.frame + 1, "aviso": w.to_string() }));
    let cover = s
        .cover_warnings()
        .into_iter()
        .map(|w| json!({ "capa": true, "quadro": w.frame + 1, "aviso": w.to_string() }));
    let warnings: Vec<Value> = spreads.chain(cover).collect();
    let text = match warnings.len() {
        0 => "Nenhum aviso de pré-impressão.".to_owned(),
        1 => "1 aviso de pré-impressão.".to_owned(),
        n => format!("{n} avisos de pré-impressão."),
    };
    let data = json!({
        "avisos": warnings,
        "fotos_faltando": s.missing_message(),
        "aviso_de_paginas": s.page_warning(),
    });
    Ok(ToolReply::new(text, data))
}

/// Os passos do histórico, com o atual e o salvo (spec HI).
pub(super) fn history(editor: &Editor) -> Result<ToolReply, ToolError> {
    let list = session(editor)?.history_list();
    let steps: Vec<Value> = list
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            json!({ "numero": i + 1, "rotulo": step.label, "desfeito": step.undone, "salvo": step.saved })
        })
        .collect();
    let text = format!(
        "{} passos no histórico; o atual é o passo {}.",
        steps.len(),
        list.position
    );
    let data = json!({
        "inicio": list.start,
        "inicio_salvo": list.start_saved,
        "passos": steps,
        "atual": list.position,
    });
    Ok(ToolReply::new(text, data))
}

/// Os comandos do registro, com nome estável, rótulo, menu e atalho (recurso `lamina://comandos`).
pub fn commands_data() -> Value {
    let commands: Vec<Value> = Command::ALL
        .into_iter()
        .map(|c| {
            json!({
                "nome": c.name(),
                "rotulo": c.label(),
                "menu": menu_name(c.menu()),
                "atalho": c.shortcut().map(|s| s.text()),
                "abre_janela": super::commands::is_dialog(c),
            })
        })
        .collect();
    json!({ "comandos": commands })
}

fn menu_name(menu: Menu) -> &'static str {
    match menu {
        Menu::File => "Arquivo",
        Menu::Edit => "Editar",
        Menu::Spread => "Lâmina",
        Menu::Cover => "Capa",
        Menu::Layout => "Layout",
        Menu::View => "Ver",
        Menu::Help => "Ajuda",
    }
}
