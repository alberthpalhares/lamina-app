//! O álbum de teste da spec IA e um atalho para chamar as ferramentas pelo nome, como o servidor
//! MCP chama.

use std::path::{Path, PathBuf};

use lamina_core::{FrameContent, PhotoId, Project};
use serde_json::Value;

use super::common::Base;
use crate::agent::{Agent, AgentOptions, ToolCall, ToolError, ToolReply};
use crate::edits::{layout_with, spread_photos};
use crate::{Editor, builtin_presets};

/// O álbum de IA-B1: 6 lâminas, 30 fotos (2 sem uso, tiradas da lâmina 6) e uma foto ampliada
/// demais (o primeiro quadro da lâmina 2, com as medidas reduzidas no projeto).
pub fn album_project(base: &Base) -> Project {
    let mut project = base.project(6);
    let last = spread_photos(&project, 5);
    layout_with(&mut project, 5, &last[..last.len() - 2]).unwrap();
    let FrameContent::Photo { photo, .. } = project.spreads[1].frames[0].content else {
        panic!("o primeiro quadro da lâmina 2 devia ter foto");
    };
    let small = project.photos.iter_mut().find(|p| p.id == photo).unwrap();
    small.width_px /= 20;
    small.height_px /= 20;
    project
}

/// As fotos sem uso do álbum, em ordem.
pub fn unused(project: &Project) -> Vec<PhotoId> {
    let used: Vec<PhotoId> = (0..project.spreads.len())
        .flat_map(|i| spread_photos(project, i))
        .collect();
    project
        .photos
        .iter()
        .map(|p| p.id)
        .filter(|id| !used.contains(id))
        .collect()
}

/// Um agente e um editor sobre um álbum, com as pastas permitidas `roots`.
pub struct Lab {
    pub agent: Agent,
    pub editor: Editor,
}

impl Lab {
    pub fn new(base: &Base, roots: Vec<PathBuf>, images: bool) -> Self {
        let options = AgentOptions {
            roots,
            images,
            cache: base.cache(),
            presets: builtin_presets(),
        };
        Lab {
            agent: Agent::new(options),
            editor: Editor::new(),
        }
    }

    /// O álbum de IA-B1 gravado em `<base>/album.lamina` e aberto pela IA, com a pasta da base
    /// permitida.
    pub fn opened(base: &Base, images: bool) -> (Self, PathBuf) {
        let file = base.save(album_project(base), "album.lamina");
        let mut lab = Lab::new(base, vec![base.dir.path().to_path_buf()], images);
        lab.ok("abrir_album", serde_json::json!({ "caminho": file }));
        (lab, file)
    }

    pub fn call(&mut self, name: &str, args: Value) -> Result<ToolReply, ToolError> {
        let call = ToolCall::parse(name, args)?;
        self.agent.run(&mut self.editor, call, &|_, _| {})
    }

    pub fn ok(&mut self, name: &str, args: Value) -> ToolReply {
        match self.call(name, args) {
            Ok(reply) => reply,
            Err(e) => panic!("{name} recusou: {e}"),
        }
    }

    pub fn project(&self) -> &Project {
        self.editor.session().unwrap().project()
    }

    /// O álbum inteiro, mesmo com a capa aberta.
    pub fn project_album(&self) -> &Project {
        self.editor.session().unwrap().album()
    }

    /// Os rótulos do histórico, em ordem.
    pub fn labels(&self) -> Vec<String> {
        let list = self.editor.session().unwrap().history_list();
        list.steps.into_iter().map(|s| s.label).collect()
    }
}

pub fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}
