//! Os arquivos da filmagem: os quadros em PNG e a conferência das pastas do usuário.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use lamina_core::{ExportPrefs, PageLimits, Project};
use lamina_layout::{Preferences, Reuse};

use super::FilmError;
use crate::album::{NewAlbum, SpecFields};
use crate::new_project::AlbumStyle;

/// As variáveis de onde saem as pastas do usuário, em cada sistema (`config_dir` e o cache).
const USER_FOLDERS: [&str; 6] = [
    "APPDATA",
    "LOCALAPPDATA",
    "USERPROFILE",
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
];

/// Grava um quadro: `rgb` tem 3 bytes por pixel.
pub fn write_frame(path: &Path, width: u32, height: u32, rgb: &[u8]) -> Result<(), FilmError> {
    let failed = |message: String| FilmError::Write {
        path: path.display().to_string(),
        message,
    };
    let file = File::create(path).map_err(|e| failed(e.to_string()))?;
    let encoder =
        PngEncoder::new_with_quality(BufWriter::new(file), CompressionType::Fast, FilterType::Sub);
    encoder
        .write_image(rgb, width, height, ExtendedColorType::Rgb8)
        .map_err(|e| failed(e.to_string()))
}

/// O álbum sem fotos do roteiro com `"abrir": "vazio"`.
pub fn empty_album() -> Result<Project, String> {
    let fields = SpecFields {
        width_cm: 30.0,
        height_cm: 30.0,
        dpi: 300,
        ..SpecFields::default()
    };
    let spec = fields.to_spec().map_err(|failure| failure.to_string())?;
    Ok(crate::new_project::empty_album(&NewAlbum {
        folder: PathBuf::new(),
        name: Some("Álbum vazio".to_owned()),
        spec,
        spreads: None,
        reuse: Reuse::Medium,
        preferences: Preferences::default(),
        models: Vec::new(),
        limits: PageLimits::default(),
        export: ExportPrefs::default(),
        style: AlbumStyle::default(),
        files: Vec::new(),
        build: false,
    }))
}

/// O caminho para comparar: barras iguais, sem a última e sem diferença de maiúsculas.
fn normal(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Confere se as pastas do usuário ficam dentro de `output`, só pelas variáveis recebidas. Devolve as
/// que estão fora; sem nenhuma delas, as que precisariam existir.
pub fn isolated(vars: &[(String, String)], output: &str) -> Result<(), Vec<String>> {
    let inside = format!("{}/", normal(output));
    let known: Vec<&(String, String)> = vars
        .iter()
        .filter(|(name, _)| USER_FOLDERS.contains(&name.as_str()))
        .collect();
    if known.is_empty() {
        return Err(vec!["APPDATA".to_owned(), "HOME".to_owned()]);
    }
    let outside: Vec<String> = known
        .iter()
        .filter(|(_, value)| !format!("{}/", normal(value)).starts_with(&inside))
        .map(|(name, _)| name.clone())
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}
