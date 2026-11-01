//! # lamina-core
//!
//! Modelo de domínio do Lamina. Este crate não faz IO nem sabe nada de interface:
//! tudo aqui é lógica pura, testável com `cargo test -p lamina-core`.
//!
//! - [`units`]: milímetros, DPI e conversão para pixels.
//! - [`geometry`]: retângulos absolutos (mm) e relativos (frações 0..=1).
//! - [`spec`]: especificação física do álbum (tamanho, sangria, zona segura).
//! - [`document`]: o projeto em si (fotos, lâminas, quadros) e o formato JSON.
//! - [`history`]: desfazer/refazer por snapshots.
//! - [`time`]: data e hora de captura das fotos.
//!
//! Convenção: identificadores em inglês, documentação em português.
//! O glossário em `docs/glossario.md` faz a ponte (ex.: lâmina = `Spread`, quadro = `Frame`).

pub mod byte_cache;
pub mod color;
pub mod document;
pub mod geometry;
pub mod history;
pub mod schema;
pub mod spec;
pub mod time;
pub mod units;

mod error;

pub use color::ColorSpace;
pub use document::{AlbumLook, CropLevel, LIGHT_MAX_CROP, Mounting, Opacity};
pub use document::{
    BlankPages, Border, CURRENT_SCHEMA_VERSION, Crop, DEFAULT_GAP, ExportPrefs, Frame,
    FrameContent, FrameId, Orientation, OutputFormat, PageLimits, Photo, PhotoAdjust, PhotoId,
    PixelRegion, Project, Rgb, Spread, SpreadId, TextAlign, TextBox, TextVAlign, lexical_path,
    relative_path, validate_project_name,
};
pub use document::{DEFAULT_FONT, LINE_SPACING, MAX_TEXT_CHARS, TEXT_SIZE_PT, clean_text};
pub use document::{cover_factor, screen_to_photo, turned_box};
pub use error::CoreError;

/// Apoio aos testes de velocidade dos outros crates (folga única no CI).
#[doc(hidden)]
pub mod testing;
pub use geometry::{Rect, RelRect};
pub use history::History;
pub use schema::{project_schema, schema_file_name};
pub use spec::{ActivePages, AlbumEdge, AlbumSpec, DesignMode, Edges, Margins, PageSide};
pub use time::CaptureTime;
pub use units::{Dpi, MM_PER_INCH, Mm};
