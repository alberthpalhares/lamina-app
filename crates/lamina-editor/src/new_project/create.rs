//! Criar o álbum (spec NP, regras 8 e 6): onde salvar, sem nunca substituir um arquivo, o álbum
//! sem fotos e a aparência aplicada em todas as lâminas.

use std::path::{Path, PathBuf};

use lamina_core::{FrameContent, Project};

use super::AlbumStyle;
use crate::NewAlbum;

/// Mensagens da seção 6.
pub const NO_PHOTOS_HERE: &str = "Não achei fotos nessa pasta.";
pub const CANNOT_WRITE: &str = "Não dá para salvar nessa pasta. Escolha outra.";
pub const PICK_PHOTOS: &str = "Escolha a pasta das fotos ou comece sem fotos.";

/// O que a etapa 3 diz embaixo dos campos (spec DD, regra 16): a falta da pasta é só o próximo passo,
/// em cor neutra; erro é o que está errado de verdade. Com qualquer um dos dois, Criar fica apagado.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepThreeState {
    pub hint: String,
    pub error: String,
}

impl StepThreeState {
    pub fn new(problem: Option<String>, missing_photos: bool) -> Self {
        Self {
            hint: if missing_photos {
                PICK_PHOTOS.to_owned()
            } else {
                String::new()
            },
            error: problem.unwrap_or_default(),
        }
    }

    pub fn can_create(&self) -> bool {
        self.hint.is_empty() && self.error.is_empty()
    }
}

/// A pasta Documentos do fotógrafo (a pasta pessoal, se não houver).
pub fn documents_dir() -> PathBuf {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map_or_else(std::env::temp_dir, PathBuf::from);
    let docs = home.join("Documents");
    if docs.is_dir() { docs } else { home }
}

/// Dá para gravar um álbum em `folder`? A pasta que ainda não existe vale pela primeira que
/// existe acima dela (criar a cria).
pub fn can_write(folder: &Path) -> bool {
    let Some(existing) = folder.ancestors().find(|p| p.is_dir()) else {
        return false;
    };
    tempfile::tempfile_in(existing).is_ok()
}

/// A pasta sugerida: a das fotos, senão a última usada, senão `Documentos\Lamina`.
pub fn suggested_folder(photos: Option<&Path>, last: Option<&Path>, documents: &Path) -> PathBuf {
    photos
        .or(last)
        .map_or_else(|| documents.join("Lamina"), Path::to_path_buf)
}

/// `<pasta>/<nome>.lamina`, ou com " (2)", " (3)"… se o arquivo já existe: criar nunca
/// substitui um álbum.
pub fn free_file(folder: &Path, name: &str) -> PathBuf {
    let name = name.trim();
    let first = folder.join(format!("{name}.lamina"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| folder.join(format!("{name} ({n}).lamina")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// "Começar sem fotos": o álbum com a especificação, a montagem e a aparência, sem fotos e sem
/// lâminas (as fotos chegam depois; spec IM).
pub fn empty_album(new: &NewAlbum) -> Project {
    let name = new.name.as_deref().map_or("Álbum", str::trim);
    let mut project = Project::new(name, new.spec.clone());
    project.page_limits = new.limits;
    project.export = new.export;
    put_style(&mut project, &new.style);
    project
}

/// A montagem, a aparência e o espaço entre fotos no projeto, antes de montar as lâminas.
pub fn put_style(project: &mut Project, style: &AlbumStyle) {
    project.mounting = style.mounting;
    project.look = style.look;
    project.gap = style.gap;
}

/// A aparência do projeto em todas as lâminas: o fundo, o espaço do álbum e a borda e os cantos
/// das fotos (NP-B11).
pub fn apply_look(project: &mut Project) {
    let look = project.look;
    for spread in &mut project.spreads {
        spread.background = look.background;
        spread.background_right = None;
        spread.gap = None;
        for frame in &mut spread.frames {
            if matches!(frame.content, FrameContent::Photo { .. }) {
                frame.border = look.border;
                frame.radius = look.radius;
            }
        }
    }
}
