//! O documento do projeto: fotos vinculadas, lâminas e quadros.
//!
//! O projeto é salvo como JSON versionado (ADR 0004). As fotos **não** são embutidas:
//! o documento guarda o caminho relativo e uma impressão digital para reencontrar o
//! arquivo se ele mudar de pasta.

#[cfg(test)]
mod adjust_tests;
#[cfg(test)]
mod blank_tests;
mod content;
mod crop;
mod ids;
mod migration;
mod mounting;
mod opacity;
#[cfg(test)]
mod opacity_tests;
mod paths;
mod photo;
mod print;
mod project;
mod text;
mod turn;
#[cfg(test)]
mod turn_tests;

pub use content::{BlankPages, Border, Frame, FrameContent, Rgb, Spread};
pub use crop::{Crop, PixelRegion};
pub use ids::{FrameId, PhotoId, SpreadId};
pub use mounting::{AlbumLook, CropLevel, LIGHT_MAX_CROP, Mounting};
pub use opacity::Opacity;
pub use paths::{lexical_path, relative_path, validate_project_name};
pub use photo::{Orientation, Photo, PhotoAdjust};
pub use print::{ExportPrefs, OutputFormat, PageLimits};
pub use project::{CURRENT_SCHEMA_VERSION, DEFAULT_GAP, Project};
pub use text::{
    DEFAULT_FONT, LINE_SPACING, MAX_TEXT_CHARS, TEXT_SIZE_PT, TextAlign, TextBox, TextVAlign,
    clean_text,
};
pub use turn::{cover_factor, screen_to_photo, turned_box};
