use thiserror::Error;

use crate::document::{FrameId, PhotoId, SpreadId};

/// Erros do modelo de domínio.
///
/// Cada variante descreve um dado inválido. Quem chama decide como mostrar ao usuário.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("DPI inválido: {0} (use um valor entre 1 e {max})", max = crate::units::Dpi::MAX)]
    InvalidDpi(u32),

    #[error("medida inválida em `{field}`: {value} mm")]
    InvalidLength { field: &'static str, value: f64 },

    #[error("retângulo relativo inválido: {0}")]
    InvalidRelRect(String),

    #[error("margem `{margin}` inválida na borda `{side}`: {value} mm")]
    InvalidMargin {
        margin: &'static str,
        side: &'static str,
        value: f64,
    },

    /// Uma medida da capa fora dos limites (spec CA, seção 6).
    #[error("medida inválida na capa, em `{field}`: {value} mm")]
    InvalidCover { field: &'static str, value: f64 },

    #[error("a zona segura não cabe na página: sobrariam {free_width} × {free_height} mm")]
    SafeMarginTooLarge { free_width: f64, free_height: f64 },

    #[error("projeto na versão de formato {from} não pôde ser migrado: {reason}")]
    Migration { from: u32, reason: String },

    #[error(
        "ajuste da foto {photo} fora das faixas (−45° a 45°, brilho e contraste de −100 a 100)"
    )]
    InvalidAdjust { photo: String },

    #[error("opacidade {value}% fora da faixa (0 a 100) num quadro da lâmina {spread}")]
    InvalidOpacity { value: u8, spread: usize },

    #[error("recorte inválido: {0}")]
    InvalidCrop(String),

    #[error("foto {0:?} não existe no projeto")]
    UnknownPhoto(PhotoId),

    #[error("lâmina {0:?} não existe no projeto")]
    UnknownSpread(SpreadId),

    #[error("quadro {0:?} não existe na lâmina")]
    UnknownFrame(FrameId),

    #[error(
        "projeto salvo na versão de formato {found}, mas este programa só entende até a {supported}"
    )]
    UnsupportedSchema { found: u32, supported: u32 },

    #[error("Nome de projeto inválido: \"{0}\". Use letras, números, espaços, - e _.")]
    InvalidName(String),

    #[error("JSON do projeto inválido: {0}")]
    Json(#[from] serde_json::Error),
}
