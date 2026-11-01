//! Erros do editor, com as mensagens que a janela e a linha de comando mostram.

use lamina_core::CoreError;
use lamina_export::{ExportError, ProjectFileError};
use lamina_layout::{BuildError, LayoutError};
use lamina_media::MediaError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("A pasta {0} não existe.")]
    MissingFolder(String),
    #[error("Nenhuma foto encontrada em {0}.")]
    NoPhotos(String),
    /// A pasta tem arquivos, mas nenhum é foto.
    #[error(
        "Nenhuma foto encontrada em {folder}: {}, mas nenhum em JPEG, PNG, TIFF ou HEIC.",
        crate::empty_import::files(*others, "arquivo")
    )]
    OnlyOtherFiles { folder: String, others: usize },
    /// Há fotos, mas nenhuma pôde ser lida (sem permissão, disco desconectado, arquivo estragado).
    #[error("Nenhuma das {found} fotos de {folder} pôde ser lida. Primeiro motivo: {first}.")]
    NoPhotoReadable {
        folder: String,
        found: usize,
        first: String,
    },
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error(transparent)]
    File(#[from] ProjectFileError),
    #[error(transparent)]
    Export(#[from] ExportError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("Quadro ou lâmina que não existe.")]
    NoFrame,
    #[error("Nenhum projeto aberto.")]
    NoProject,
    #[error("O projeto ainda não tem arquivo: use Salvar como.")]
    NoFile,
    #[error("O álbum de exemplo não está em {0}.")]
    NoExample(String),
    #[error("Não foi possível copiar o álbum de exemplo para {path}: {reason}.")]
    CopyExample { path: String, reason: String },
    /// Acrescentar numa lâmina que já tem a capacidade dela (spec ML, regra 8).
    #[error("A lâmina {} já tem {capacity} fotos, o máximo que cabe nela.", spread + 1)]
    SpreadFull { spread: usize, capacity: usize },
    /// Acrescentar uma foto que já está na lâmina (spec ML, regra 9).
    #[error("Esta foto já está na lâmina {}.", spread + 1)]
    AlreadyInSpread { spread: usize },
    /// Levar embora a única foto da lâmina pela Sequência (spec SQ, regra 5).
    #[error("A lâmina {} ficaria sem fotos.", spread + 1)]
    LastPhoto { spread: usize },
    /// Foto numa página em branco ou numa lâmina toda em branco (spec PB, regras 7 e 10).
    #[error("Esta lâmina está em branco. Desmarque uma página para pôr fotos.")]
    BlankPage,
    /// "Mostrar no Explorador" de uma foto cujo arquivo sumiu (spec TF, seção 6).
    #[error("O arquivo {0} não existe mais.")]
    PhotoFileMissing(String),
}
