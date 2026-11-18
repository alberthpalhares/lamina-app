//! Trocar o nome do álbum (spec NA): o nome limpo vira um passo do histórico; vazio, igual ou com
//! caractere que não serve para nome de arquivo não muda nada. O nome do arquivo `.lamina`
//! continua o mesmo (regra 6).

use lamina_core::{CoreError, validate_project_name};

use super::Editor;

/// O nome mais comprido que o álbum guarda, em letras (regra 2).
pub const MAX_ALBUM_NAME: usize = 120;

/// O nome como fica guardado: quebras de linha viram espaço, sem espaços nas pontas e com até
/// [`MAX_ALBUM_NAME`] letras. `None` quando não sobra nada (regra 3).
pub fn clean_album_name(name: &str) -> Option<String> {
    let flat: String = name
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let cut: String = flat.trim().chars().take(MAX_ALBUM_NAME).collect();
    let cut = cut.trim_end().to_owned();
    (!cut.is_empty()).then_some(cut)
}

impl Editor {
    /// Troca o nome do álbum aberto; `Ok(true)` se mudou (regras 3 a 5 e 8). Um nome que não
    /// serve para nome de arquivo devolve `Err(CoreError::InvalidName)`: como o título vira o
    /// nome do PDF (spec PD), precisa abrir em exFAT e no Windows.
    pub fn rename_album(&mut self, name: &str) -> Result<bool, CoreError> {
        let Some(clean) = clean_album_name(name) else {
            return Ok(false);
        };
        validate_project_name(&clean)?;
        let Some(session) = self.session_mut() else {
            return Ok(false);
        };
        if session.project().name == clean {
            return Ok(false);
        }
        session.edit("Renomear álbum · álbum", |p| {
            p.name = clean;
            Ok::<_, CoreError>(())
        })?;
        Ok(true)
    }
}
