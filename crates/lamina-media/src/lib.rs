//! # lamina-media
//!
//! Importação de fotos:
//!
//! - [`format`]: formatos reconhecidos e listagem de pastas.
//! - [`metadata`]: dimensões, orientação, data de captura e câmera (EXIF), sem decodificar a foto.
//! - [`color`]: o perfil de cor de cada foto e a conversão para sRGB ou Adobe RGB (spec CT).
//! - [`fingerprint`]: impressão digital rápida do conteúdo, para reencontrar fotos movidas.
//! - [`thumbnail`]: miniaturas e imagens de tela com a rotação EXIF aplicada.
//! - [`derive`]: as cópias reduzidas a partir de uma leitura só do arquivo (spec TR).
//! - [`pool`]: no máximo 4 fotos lidas ao mesmo tempo (spec TR).
//! - [`catalog`]: catálogo SQLite, que evita reprocessar fotos que não mudaram.
//! - [`import`]: tudo junto, em paralelo.

pub mod camera;
mod density;
pub use density::image_dpi;
pub mod catalog;
pub mod color;
pub mod decode;
pub mod derive;
pub mod fingerprint;
pub mod format;
pub mod import;
pub mod metadata;
pub mod photo_source;
pub mod pool;
pub mod thumbnail;

#[cfg(test)]
#[path = "tests/ct.rs"]
mod ct;
mod error;
#[cfg(test)]
mod testutil;
#[cfg(test)]
#[path = "tests/tr.rs"]
mod tr;

#[cfg(test)]
mod lv_tests;

pub use camera::{CameraDetails, camera_details};
pub use catalog::{Catalog, CatalogEntry};
pub use color::{embedded_profile, profile_problem};
pub use decode::{decode_in, decode_with_orientation, load_oriented, load_oriented_in, to_rgba};
pub use error::{MediaError, io_reason};
pub use format::{Gathered, ImageFormat, gather_photos, list_photos};
pub use import::{
    ImportReport, Progress, default_cache_dir, import_files, import_folder, import_folder_with,
};
pub use metadata::{PhotoMetadata, read_metadata};
pub use photo_source::{
    Loaded, cached_source, original_source, original_source_in, thumbnail_source,
};
pub use pool::{PHOTOS_AT_ONCE, map_photos, photos_at_once};
pub use thumbnail::{
    DerivedDirs, SCREEN_SIZE, THUMB_SIZE, derived_path, ensure_derived, ensure_derived_for,
    ensure_thumbnail,
};
