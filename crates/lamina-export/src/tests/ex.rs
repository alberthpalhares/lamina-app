//! Cenários da spec `specs/exportacao.md`: EX-B1 a EX-B6 aqui; EX-B7 a EX-B12 em `ex_more.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::{Rgba, RgbaImage};
use lamina_core::{
    AlbumEdge, AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, PhotoId, Project, RelRect, Rgb,
};

use crate::{ExportError, ExportOptions, ExportReport, export, srgb_profile};

pub fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap()
        .with_safe_margin(Mm(10.0))
        .unwrap()
}

/// Projeto de teste com fotos em memória.
pub struct Album {
    pub project: Project,
    pub images: HashMap<PathBuf, Arc<RgbaImage>>,
}

impl Album {
    pub fn new(spec: AlbumSpec, spreads: usize) -> Self {
        let mut project = Project::new("teste", spec);
        for _ in 0..spreads {
            project.add_spread();
        }
        Album {
            project,
            images: HashMap::new(),
        }
    }

    pub fn photo(&mut self, name: &str, image: RgbaImage) -> PhotoId {
        let id = self.project.add_photo(name, image.width(), image.height());
        self.images.insert(PathBuf::from(name), Arc::new(image));
        id
    }

    pub fn frame(&mut self, spread: usize, content: FrameContent, rect: [f64; 4]) {
        let id = self.project.spreads[spread].id;
        let rect = RelRect::new(rect[0], rect[1], rect[2], rect[3]).unwrap();
        self.project.add_frame(id, rect, content).unwrap();
    }

    pub fn photo_frame(&mut self, spread: usize, photo: PhotoId, rect: [f64; 4]) {
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        self.frame(spread, content, rect);
    }

    pub fn export(
        &self,
        folder: &Path,
        options: &ExportOptions,
    ) -> Result<ExportReport, ExportError> {
        let source = |p: &Photo, _: (u32, u32)| {
            self.images
                .get(&p.path)
                .cloned()
                .ok_or_else(|| "arquivo sumiu".to_owned())
        };
        export(&self.project, folder, &source, options, &|_, _| {})
    }

    /// A lâmina `spread` renderizada para impressão, sem passar pela exportação.
    pub fn print(&self, spread: usize) -> RgbaImage {
        let source = |p: &Photo, _: (u32, u32)| {
            self.images
                .get(&p.path)
                .cloned()
                .ok_or_else(|| "arquivo sumiu".to_owned())
        };
        let target = lamina_render::RenderTarget::Print;
        lamina_render::render_spread(&self.project, spread, target, &source).unwrap()
    }
}

pub fn solid(w: u32, h: u32, c: [u8; 3]) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba([c[0], c[1], c[2], 255]))
}

/// Ruído (sempre o mesmo): muitos detalhes, para a qualidade do JPEG fazer diferença.
pub fn noise(w: u32, h: u32) -> RgbaImage {
    let mut seed: u32 = 1;
    RgbaImage::from_fn(w, h, |_, _| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let [a, b, c, _] = seed.to_le_bytes();
        Rgba([a, b, c, 255])
    })
}

/// Três lâminas, cada uma com uma foto na página direita.
pub fn three() -> Album {
    let mut album = Album::new(spec(), 3);
    for s in 0..3 {
        let p = album.photo(&format!("f{s}.jpg"), solid(600, 400, [200, 50, 50]));
        album.photo_frame(s, p, [0.55, 0.2, 0.3, 0.4]);
    }
    album
}

pub fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

#[test]
fn ex_b1_uma_imagem_por_lamina() {
    let dir = tempfile::tempdir().unwrap();
    let report = three()
        .export(dir.path(), &ExportOptions::default())
        .unwrap();
    assert_eq!(
        names(dir.path()),
        ["lamina-01.jpg", "lamina-02.jpg", "lamina-03.jpg"]
    );
    assert_eq!(report.files.len(), 3);
    for f in &report.files {
        assert_eq!(image::image_dimensions(f).unwrap(), (7157, 3614));
    }
}

/// Densidade gravada no cabeçalho JFIF: (unidade, x, y).
fn jfif_density(bytes: &[u8]) -> (u8, u16, u16) {
    let at = bytes
        .windows(5)
        .position(|w| w == b"JFIF\0")
        .expect("sem JFIF");
    let unit = bytes[at + 7];
    let x = u16::from_be_bytes([bytes[at + 8], bytes[at + 9]]);
    let y = u16::from_be_bytes([bytes[at + 10], bytes[at + 11]]);
    (unit, x, y)
}

#[test]
fn ex_b2_perfil_srgb_e_dpi_no_arquivo() {
    use image::ImageDecoder;
    let dir = tempfile::tempdir().unwrap();
    three()
        .export(dir.path(), &ExportOptions::default())
        .unwrap();
    let path = dir.path().join("lamina-01.jpg");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        jfif_density(&bytes),
        (1, 300, 300),
        "1 = pontos por polegada"
    );
    let mut decoder = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(&bytes)).unwrap();
    let icc = decoder.icc_profile().unwrap().expect("sem perfil ICC");
    assert_eq!(icc, srgb_profile().unwrap());
}

#[test]
fn ex_b3_qualidade() {
    let mut album = Album::new(spec(), 1);
    let p = album.photo("ruido.jpg", noise(1200, 800));
    album.photo_frame(0, p, [0.05, 0.1, 0.4, 0.8]);
    let size = |quality| {
        let dir = tempfile::tempdir().unwrap();
        let options = ExportOptions {
            quality,
            ..ExportOptions::default()
        };
        album.export(dir.path(), &options).unwrap();
        std::fs::metadata(dir.path().join("lamina-01.jpg"))
            .unwrap()
            .len()
    };
    assert!(size(60) < size(95));
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        quality: 0,
        ..ExportOptions::default()
    };
    let err = album.export(dir.path(), &options).unwrap_err();
    assert!(matches!(err, ExportError::Quality(0)));
    assert_eq!(
        err.to_string(),
        "A qualidade do JPEG vai de 1 a 100, não 0."
    );
}

#[test]
fn ex_b4_intervalo_de_laminas() {
    let album = Album::new(spec(), 5);
    let run = |range: &str| {
        let dir = tempfile::tempdir().unwrap();
        let options = ExportOptions {
            range: Some(range.to_owned()),
            ..ExportOptions::default()
        };
        album
            .export(dir.path(), &options)
            .map(|_| names(dir.path()))
    };
    assert_eq!(
        run("1, 3-4").unwrap(),
        ["lamina-01.jpg", "lamina-03.jpg", "lamina-04.jpg"]
    );
    let err = run("2-9").unwrap_err();
    assert!(matches!(
        err,
        ExportError::RangeTooBig { asked: 9, count: 5 }
    ));
    assert_eq!(
        err.to_string(),
        "O intervalo pede a lâmina 9; o projeto tem 5."
    );
    let err = run("a").unwrap_err();
    assert!(matches!(&err, ExportError::BadRange(t) if t == "a"));
    assert_eq!(
        err.to_string(),
        "Intervalo inválido: \"a\". Use, por exemplo, 1, 2, 5-7."
    );
}

#[test]
fn ex_b5_paginas_soltas() {
    let s = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let mut album = Album::new(s, 3);
    album.project.spreads[0].background = Rgb { r: 0, g: 0, b: 200 };
    let dir = tempfile::tempdir().unwrap();
    let options = ExportOptions {
        pages: true,
        ..ExportOptions::default()
    };
    album.export(dir.path(), &options).unwrap();
    let expected = [
        "pagina-01.jpg",
        "pagina-02.jpg",
        "pagina-03.jpg",
        "pagina-04.jpg",
    ];
    assert_eq!(names(dir.path()), expected);
    for name in expected {
        assert_eq!(
            image::image_dimensions(dir.path().join(name)).unwrap(),
            (3614, 3614)
        );
    }
    // pagina-01 é a página direita da primeira lâmina (azul); pagina-02 já é da segunda.
    let first = image::open(dir.path().join("pagina-01.jpg"))
        .unwrap()
        .to_rgb8();
    let second = image::open(dir.path().join("pagina-02.jpg"))
        .unwrap()
        .to_rgb8();
    assert!(first.get_pixel(1800, 1800)[2] > 180 && first.get_pixel(1800, 1800)[0] < 30);
    assert!(
        second.get_pixel(1800, 1800)[0] > 230,
        "a segunda lâmina tem fundo branco"
    );
}

#[test]
fn ex_b6_nao_sobrescreve_sem_pedir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lamina-01.jpg"), b"antigo").unwrap();
    let album = three();
    let err = album
        .export(dir.path(), &ExportOptions::default())
        .unwrap_err();
    assert!(matches!(&err, ExportError::Exists { name, .. } if name == "lamina-01.jpg"));
    let folder = dir.path().display().to_string();
    assert_eq!(
        err.to_string(),
        format!("Já existe lamina-01.jpg em {folder}; use sobrescrever para substituir.")
    );
    assert_eq!(names(dir.path()), ["lamina-01.jpg"], "nada foi gravado");
    let options = ExportOptions {
        overwrite: true,
        ..ExportOptions::default()
    };
    album.export(dir.path(), &options).unwrap();
    assert_ne!(
        std::fs::read(dir.path().join("lamina-01.jpg")).unwrap(),
        b"antigo"
    );
    assert_eq!(names(dir.path()).len(), 3);
}

#[cfg(test)]
#[path = "ex_more.rs"]
mod more;

#[cfg(test)]
#[path = "ex_quality.rs"]
mod quality;
