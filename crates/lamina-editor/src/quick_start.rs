//! "Início rápido" (spec IQ): o álbum vazio com o tamanho padrão, sem assistente. O tamanho, as
//! fotos, o nome e o destino vêm depois, com o álbum já aberto: "Especificação do álbum…" readequa
//! os quadros (spec PR, regra 6, ADR 0003) e importar traz as fotos (spec IM).

use lamina_core::{CoreError, Project};

use crate::album::SpecFields;
use crate::new_project::{AlbumStyle, put_style};
use crate::{Editor, EditorError};

/// O nome do álbum do início rápido; o fotógrafo renomeia depois (spec NA).
pub const QUICK_START_NAME: &str = "Álbum novo";

/// O álbum vazio do início rápido (regras 1 a 3): o tamanho padrão (`SpecFields::default()`, o
/// mesmo do `lamina build` e do Personalizado do Novo álbum: 30 × 30 cm, 300 DPI, sangria 3 mm,
/// zona segura 10 mm, lay-flat), a montagem e a aparência de sempre, zero fotos e zero lâminas.
pub fn quick_start_project() -> Result<Project, CoreError> {
    let mut project = Project::new(QUICK_START_NAME, SpecFields::default().to_spec()?);
    put_style(&mut project, &AlbumStyle::default());
    Ok(project)
}

impl Editor {
    /// Abre o álbum do início rápido, ainda sem arquivo (regra 3): o primeiro Salvar pede onde
    /// gravar (JP, regra 8) e só então ele entra nos recentes (TI, regra 1).
    pub fn quick_start(&mut self) -> Result<(), EditorError> {
        self.adopt(quick_start_project()?);
        Ok(())
    }
}
