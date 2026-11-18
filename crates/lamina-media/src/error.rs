use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Erros da importação de fotos.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("Não foi possível ler {}: {}.", path.display(), io_reason(source))]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("A imagem {} está ilegível: {source}.", path.display())]
    Image {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("Erro no catálogo de fotos: {0}.")]
    Catalog(#[from] rusqlite::Error),

    #[error("A foto {path} é grande demais para importar: {size} bytes (o máximo é {max}).")]
    TooLarge { path: PathBuf, size: i64, max: i64 },
}

/// Motivo de um erro de arquivo em português, o mesmo em qualquer sistema (o texto do sistema
/// operacional vem no idioma dele: inglês no Linux e no macOS).
pub fn io_reason(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "arquivo não encontrado".to_owned(),
        io::ErrorKind::PermissionDenied => "sem permissão de acesso".to_owned(),
        io::ErrorKind::AlreadyExists => "o arquivo já existe".to_owned(),
        io::ErrorKind::StorageFull => "disco cheio".to_owned(),
        io::ErrorKind::IsADirectory => "é uma pasta, não um arquivo".to_owned(),
        _ => error.to_string(),
    }
}

impl MediaError {
    pub(crate) fn io(path: impl Into<PathBuf>) -> impl FnOnce(io::Error) -> MediaError {
        let path = path.into();
        move |source| MediaError::Io { path, source }
    }

    pub(crate) fn image(path: impl Into<PathBuf>) -> impl FnOnce(image::ImageError) -> MediaError {
        let path = path.into();
        move |source| MediaError::Image { path, source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arquivo_inexistente_diz_nao_encontrado() {
        let err = MediaError::io("fotos/a.jpg")(io::Error::from(io::ErrorKind::NotFound));
        let text = err.to_string();
        assert!(text.starts_with("Não foi possível ler fotos"), "{text}");
        assert!(text.ends_with("a.jpg: arquivo não encontrado."), "{text}");
    }

    #[test]
    fn foto_grande_demas_diz_o_maximo() {
        let err = MediaError::TooLarge {
            path: PathBuf::from("fotos/hdr.tif"),
            size: 4_000_000_000,
            max: 1_500_000_000,
        };
        let text = err.to_string();
        assert!(
            text.starts_with("A foto fotos/hdr.tif é grande demais"),
            "{text}"
        );
        assert!(text.contains("o máximo é 1500000000"), "{text}");
    }
}
