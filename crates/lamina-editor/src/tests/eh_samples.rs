//! Um exemplo de cada variante de cada tipo de erro que chega à janela (EH-B1 e B2).

use std::io;
use std::path::PathBuf;

use lamina_core::{CoreError, PhotoId};
use lamina_export::{ExportError, ProjectFileError};
use lamina_layout::{BuildError, LayoutError, PlanError};
use lamina_media::MediaError;
use lamina_render::RenderError;

use crate::{EditorError, Human, HumanMessage, ModelError, PresetError};

pub struct Sample {
    pub message: HumanMessage,
    pub technical: String,
}

fn s<E: Human + std::fmt::Display>(error: E) -> Sample {
    Sample {
        message: error.human(),
        technical: error.to_string(),
    }
}

fn core() -> Vec<CoreError> {
    let id: PhotoId = serde_json::from_str("999").unwrap();
    vec![
        CoreError::InvalidDpi(0),
        CoreError::InvalidLength {
            field: "page_width",
            value: 0.0,
        },
        CoreError::InvalidRelRect("x fora de 0..1".into()),
        CoreError::InvalidMargin {
            margin: "bleed",
            side: "outer",
            value: -1.0,
        },
        CoreError::SafeMarginTooLarge {
            free_width: -2.0,
            free_height: 10.0,
        },
        CoreError::Migration {
            from: 1,
            reason: "campo faltando".into(),
        },
        CoreError::InvalidCrop("zoom 0".into()),
        CoreError::InvalidAdjust {
            photo: "IMG_0001.jpg".into(),
        },
        CoreError::InvalidOpacity {
            value: 150,
            spread: 2,
        },
        CoreError::UnknownPhoto(id),
        CoreError::UnknownSpread(serde_json::from_str("7").unwrap()),
        CoreError::UnknownFrame(serde_json::from_str("8").unwrap()),
        CoreError::UnsupportedSchema {
            found: 99,
            supported: 3,
        },
        CoreError::InvalidName("festa/2026".into()),
        CoreError::Json(serde_json::from_str::<serde_json::Value>("x").unwrap_err()),
    ]
}

fn media() -> Vec<MediaError> {
    let path = PathBuf::from("/fotos/festa-07.jpg");
    let dir = tempfile::tempdir().unwrap();
    let catalog = lamina_media::Catalog::open(dir.path()).err().unwrap();
    assert!(matches!(catalog, MediaError::Catalog(_)), "{catalog}");
    vec![
        MediaError::Io {
            path: path.clone(),
            source: io::Error::new(io::ErrorKind::NotFound, "No such file or directory"),
        },
        MediaError::Io {
            path: path.clone(),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"),
        },
        MediaError::Image {
            path,
            source: image::ImageError::IoError(io::Error::other("unexpected EOF")),
        },
        catalog,
    ]
}

fn render() -> Vec<RenderError> {
    vec![
        RenderError::NoSpread {
            number: 9,
            count: 8,
        },
        RenderError::PreviewTooSmall,
        RenderError::Photo {
            name: "festa-07.jpg".into(),
            reason: "arquivo não encontrado".into(),
        },
        RenderError::Resize {
            name: "festa-07.jpg".into(),
            reason: "zero".into(),
        },
        RenderError::Canvas {
            width: 90_000,
            height: 40_000,
        },
        RenderError::Core(CoreError::InvalidDpi(0)),
    ]
}

fn layout() -> Vec<Sample> {
    let layouts = vec![
        LayoutError::NoPhotos,
        LayoutError::TooManyPhotos {
            photos: 30,
            pages: 2,
            capacity: 24,
        },
        LayoutError::InvalidAspect(3),
        LayoutError::GapTooLarge,
    ];
    let plans = vec![
        PlanError::NoPhotos,
        PlanError::NoSpreads,
        PlanError::InvalidMaxPerPage { value: 20, max: 12 },
        PlanError::MoreSpreadsThanPhotos {
            photos: 4,
            spreads: 6,
        },
        PlanError::TooManyPhotos {
            photos: 120,
            capacity: 96,
            spreads: 12,
            min_spreads: 15,
        },
    ];
    let builds = vec![
        BuildError::InvalidPhoto("/fotos/festa-07.jpg".into()),
        BuildError::Plan(PlanError::NoSpreads),
        BuildError::Layout(LayoutError::GapTooLarge),
        BuildError::Core(CoreError::InvalidDpi(0)),
    ];
    let mut all: Vec<Sample> = layouts.into_iter().map(s).collect();
    all.extend(plans.into_iter().map(s));
    all.extend(builds.into_iter().map(s));
    all
}

fn write_error() -> ExportError {
    ExportError::Write {
        path: "/saida/lamina-03.jpg".into(),
        reason: "No space left on device (os error 28)".into(),
    }
}

fn export() -> Vec<ExportError> {
    vec![
        ExportError::Quality(0),
        ExportError::CropMarksNeedPdf,
        ExportError::Format("PSD"),
        ExportError::BadRange("abc".into()),
        ExportError::RangeTooBig {
            asked: 12,
            count: 8,
        },
        ExportError::Exists {
            name: "lamina-01.jpg".into(),
            folder: "/saida".into(),
        },
        ExportError::NoSpreads,
        ExportError::Folder {
            folder: "/saida".into(),
            reason: "Permission denied (os error 13)".into(),
        },
        write_error(),
        ExportError::Remove {
            path: "/saida/lamina-09.jpg".into(),
            reason: "Access is denied. (os error 5)".into(),
        },
        ExportError::Encode("buffer".into()),
        ExportError::Render(RenderError::PreviewTooSmall),
        ExportError::Stopped {
            done: vec![PathBuf::from("a.jpg"), PathBuf::from("b.jpg")],
            source: Box::new(write_error()),
        },
    ]
}

fn files() -> Vec<ProjectFileError> {
    vec![
        ProjectFileError::Read {
            path: "/albuns/festa.lamina".into(),
            reason: "No such file or directory (os error 2)".into(),
        },
        ProjectFileError::Write {
            path: "/albuns/festa.lamina".into(),
            reason: "disco cheio".into(),
        },
        ProjectFileError::MissingPhoto {
            name: "festa-07.jpg".into(),
            path: "/fotos/festa-07.jpg".into(),
        },
        ProjectFileError::Core(CoreError::InvalidName("a/b".into())),
    ]
}

fn editor() -> Vec<Sample> {
    let editors = vec![
        EditorError::MissingFolder("/clientes/Ana/Festa".into()),
        EditorError::NoPhotos("/clientes/Ana/Festa".into()),
        EditorError::OnlyOtherFiles {
            folder: "/clientes/Ana/Festa".into(),
            others: 3,
        },
        EditorError::NoPhotoReadable {
            folder: "/clientes/Ana/Festa".into(),
            found: 50,
            first: "festa-01.jpg: sem permissão de acesso".into(),
        },
        EditorError::Media(media().remove(0)),
        EditorError::Build(BuildError::InvalidPhoto("x.jpg".into())),
        EditorError::File(files().remove(0)),
        EditorError::Export(write_error()),
        EditorError::Core(CoreError::InvalidDpi(0)),
        EditorError::Layout(LayoutError::GapTooLarge),
        EditorError::NoFrame,
        EditorError::BlankPage,
        EditorError::NoProject,
        EditorError::NoFile,
        EditorError::NoExample("/programa/exemplo".into()),
        EditorError::CopyExample {
            path: "/config/Álbum de exemplo".into(),
            reason: "disco cheio".into(),
        },
        EditorError::SpreadFull {
            spread: 4,
            capacity: 24,
        },
        EditorError::AlreadyInSpread { spread: 4 },
        EditorError::LastPhoto { spread: 2 },
        EditorError::PhotoFileMissing("D:/Fotos/IMG_0001.jpg".into()),
    ];
    let models = vec![
        ModelError::EmptyName,
        ModelError::NoPhotoFrame,
        ModelError::TooMany {
            frames: 30,
            pages: 2,
            capacity: 24,
        },
        ModelError::Exists("Abertura".into()),
        ModelError::Write("disco cheio".into()),
    ];
    let presets = vec![
        PresetError::EmptyName,
        PresetError::Builtin,
        PresetError::LimitsOrder,
        PresetError::Spec(CoreError::InvalidDpi(0).human().line()),
        PresetError::Exists("Laboratório X".into()),
        PresetError::Write("disco cheio".into()),
    ];
    let mut all: Vec<Sample> = editors.into_iter().map(s).collect();
    all.extend(models.into_iter().map(s));
    all.extend(presets.into_iter().map(s));
    all
}

/// Todos os exemplos.
pub fn samples() -> Vec<Sample> {
    let mut all: Vec<Sample> = core().into_iter().map(s).collect();
    all.extend(media().into_iter().map(s));
    all.extend(render().into_iter().map(s));
    all.extend(layout());
    all.extend(export().into_iter().map(s));
    all.extend(files().into_iter().map(s));
    all.extend(editor());
    all
}
