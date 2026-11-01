//! Modelos do fotógrafo (spec MD): a lâmina salva como modelo e o arquivo aberto `modelos.json`,
//! na pasta de configuração, ao lado de `presets.json`.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use lamina_core::{FrameContent, Orientation, Project};
use lamina_layout::{MAX_PER_PAGE, ModelSlot, ModelText, SpreadModel};
use serde::{Deserialize, Serialize};

pub const MODELS_FILE: &str = "modelos.json";
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelError {
    #[error("Dê um nome ao modelo.")]
    EmptyName,
    #[error("A lâmina não tem quadro de foto para virar modelo.")]
    NoPhotoFrame,
    #[error("São {frames} fotos para {pages} páginas: cabem no máximo {capacity}.")]
    TooMany {
        frames: usize,
        pages: usize,
        capacity: usize,
    },
    #[error("Já existe um modelo chamado \"{0}\".")]
    Exists(String),
    #[error("Não foi possível gravar os modelos: {0}.")]
    Write(String),
}

#[derive(Serialize, Deserialize)]
struct OnDisk {
    versao: u32,
    modelos: Vec<SpreadModel>,
}

/// Os modelos do fotógrafo e o arquivo deles.
#[derive(Debug, Default)]
pub struct ModelsStore {
    path: Option<PathBuf>,
    models: Vec<SpreadModel>,
    /// Aviso de leitura para a janela mostrar (regra 10).
    pub warning: Option<String>,
}

/// Prende os quadros ao corte (seção 6). `false` se algum não tem tamanho: o arquivo inteiro é
/// recusado.
fn fit(m: &mut SpreadModel) -> bool {
    let rects = m
        .slots
        .iter_mut()
        .map(|s| &mut s.rect)
        .chain(m.texts.iter_mut().map(|t| &mut t.rect));
    for r in rects {
        match r.clamped() {
            Some(c) => *r = c,
            None => return false,
        }
    }
    true
}

impl ModelsStore {
    /// Lê o arquivo; sem arquivo, nenhum modelo. Não lido: aviso, nenhum modelo e o arquivo fica.
    pub fn load(path: &Path) -> Self {
        let mut store = ModelsStore {
            path: Some(path.to_path_buf()),
            ..ModelsStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        let problem = match serde_json::from_str::<OnDisk>(&text) {
            Ok(disk) if disk.versao > VERSION => format!("versão {} do arquivo", disk.versao),
            Ok(mut disk) => {
                if disk.modelos.iter_mut().all(fit) {
                    store.models = disk.modelos;
                    return store;
                }
                "quadro sem tamanho".to_owned()
            }
            Err(e) => e.to_string(),
        };
        store.warning = Some(format!(
            "Não foi possível ler os modelos: {problem}. Os modelos ficam de fora até o arquivo ser corrigido."
        ));
        store
    }

    pub fn all(&self) -> &[SpreadModel] {
        &self.models
    }

    pub fn exists(&self, name: &str) -> bool {
        self.models.iter().any(|m| m.name == name.trim())
    }

    /// Guarda o modelo; um nome que já existe é substituído (a janela confirma antes).
    pub fn save(&mut self, model: SpreadModel) -> Result<(), ModelError> {
        match self.models.iter_mut().find(|m| m.name == model.name) {
            Some(existing) => *existing = model,
            None => self.models.push(model),
        }
        self.write()
    }

    pub fn remove(&mut self, name: &str) -> Result<bool, ModelError> {
        let before = self.models.len();
        self.models.retain(|m| m.name != name);
        if self.models.len() == before {
            return Ok(false);
        }
        self.write().map(|()| true)
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<bool, ModelError> {
        let new = new.trim();
        if new.is_empty() {
            return Err(ModelError::EmptyName);
        }
        if new != old && self.exists(new) {
            return Err(ModelError::Exists(new.to_owned()));
        }
        let Some(m) = self.models.iter_mut().find(|m| m.name == old) else {
            return Ok(false);
        };
        new.clone_into(&mut m.name);
        self.write().map(|()| true)
    }

    fn write(&self) -> Result<(), ModelError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let disk = OnDisk {
            versao: VERSION,
            modelos: self.models.clone(),
        };
        let fail = |e: &dyn std::fmt::Display| ModelError::Write(e.to_string());
        let json = serde_json::to_string_pretty(&disk).map_err(|e| fail(&e))?;
        let dir = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| fail(&e))?;
        let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(|e| fail(&e))?;
        tmp.write_all(json.as_bytes()).map_err(|e| fail(&e))?;
        tmp.persist(path).map_err(|e| fail(&e.error))?;
        Ok(())
    }
}

/// A lâmina `index` como modelo `name` (regra 1): quadros de foto com a orientação da foto que
/// estava neles (vazio aceita qualquer uma), borda, cantos, textos e as páginas usadas.
pub fn model_from_spread(
    project: &Project,
    index: usize,
    name: &str,
) -> Result<SpreadModel, ModelError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ModelError::EmptyName);
    }
    let spread = project.spreads.get(index).ok_or(ModelError::NoPhotoFrame)?;
    let mut slots = Vec::new();
    let mut texts = Vec::new();
    for f in &spread.frames {
        let orientation = match &f.content {
            FrameContent::Text(text) => {
                texts.push(ModelText {
                    rect: f.rect,
                    text: text.clone(),
                });
                continue;
            }
            FrameContent::Empty => None,
            FrameContent::Photo { photo, .. } => project
                .photo(*photo)
                .ok()
                .map(|p| Orientation::from_aspect_ratio(p.aspect_ratio())),
        };
        slots.push(ModelSlot {
            rect: f.rect,
            orientation,
            border: f.border,
            radius: f.radius,
        });
    }
    if slots.is_empty() {
        return Err(ModelError::NoPhotoFrame);
    }
    let pages = project.content_pages(index);
    // Lâmina sem página ativa conta como a direita, como no motor.
    let (left, right) = (pages.left, pages.right || !pages.left);
    let count = usize::from(left) + usize::from(right);
    if slots.len() > MAX_PER_PAGE * count.max(1) {
        return Err(ModelError::TooMany {
            frames: slots.len(),
            pages: count.max(1),
            capacity: MAX_PER_PAGE * count.max(1),
        });
    }
    Ok(SpreadModel {
        name: name.to_owned(),
        slots,
        texts,
        left,
        right,
    })
}
