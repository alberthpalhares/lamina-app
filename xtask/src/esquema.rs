//! Esquema do projeto (spec EI, regra 9): `cargo xtask esquema` grava o JSON Schema do formato
//! em `docs/formato/`, e a checagem de documentos reprova quando o publicado não bate com o
//! código.

use std::fs;
use std::path::PathBuf;

use lamina_core::{project_schema, schema_file_name};

/// `docs/formato/projeto-v8.schema.json`.
pub fn path() -> PathBuf {
    PathBuf::from("docs/formato").join(schema_file_name())
}

/// Grava o esquema gerado dos tipos do `lamina-core`.
pub fn run() -> bool {
    let path = path();
    let written = project_schema()
        .map_err(|e| e.to_string())
        .and_then(|text| {
            fs::create_dir_all("docs/formato").map_err(|e| e.to_string())?;
            fs::write(&path, text).map_err(|e| e.to_string())
        });
    match written {
        Ok(()) => {
            println!("  esquema gravado em {}", path.display());
            true
        }
        Err(e) => {
            eprintln!("  ✘ não foi possível gravar {}: {e}", path.display());
            false
        }
    }
}

/// O problema, se o esquema publicado não é o que o código gera.
pub fn problem() -> Option<String> {
    let expected = match project_schema() {
        Ok(text) => text,
        Err(e) => return Some(format!("o esquema do projeto não pôde ser gerado: {e}")),
    };
    let published = fs::read_to_string(path()).unwrap_or_default();
    (published != expected).then(|| {
        format!(
            "{} desatualizado: rode cargo xtask esquema",
            path().display()
        )
    })
}
