//! Fotos e projetos de teste.

use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};
use lamina_core::Project;
use lamina_layout::Reuse;
use rayon::prelude::*;

use crate::{Cache, NewAlbum, Session, SpecFields, create_album};

/// Pasta temporária com `fotos/` e `cache/`.
pub struct Base {
    pub dir: tempfile::TempDir,
}

impl Base {
    /// `count` fotos `w × h` (uma vertical a cada quatro), diferentes entre si.
    pub fn with_photos(count: usize, (w, h): (u32, u32)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let photos = dir.path().join("fotos");
        std::fs::create_dir_all(&photos).unwrap();
        let gradient = |pw: u32, ph: u32| {
            RgbImage::from_fn(pw, ph, |x, y| {
                let (r, g) = (x * 255 / pw, y * 255 / ph);
                Rgb([u8::try_from(r).unwrap(), u8::try_from(g).unwrap(), 128])
            })
        };
        let (landscape, portrait) = (gradient(w, h), gradient(h, w));
        (0..count).into_par_iter().for_each(|i| {
            let mut img = if i % 4 == 3 {
                portrait.clone()
            } else {
                landscape.clone()
            };
            let tone = u8::try_from(i * 37 % 256).unwrap();
            for y in 0..16 {
                for x in 0..16 {
                    img.put_pixel(x, y, Rgb([tone, 255 - tone, tone]));
                }
            }
            img.save(photos.join(format!("f{i:02}.jpg"))).unwrap();
        });
        Base { dir }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    pub fn cache(&self) -> Cache {
        Cache::new(self.path("cache"))
    }

    /// Pedido de álbum novo com as fotos da base, numa página de 10 × 10 cm a 100 DPI (fotos
    /// pequenas não ficam ampliadas).
    pub fn new_album(&self, spreads: Option<usize>) -> NewAlbum {
        let fields = SpecFields {
            width_cm: 10.0,
            height_cm: 10.0,
            dpi: 100,
            ..SpecFields::default()
        };
        NewAlbum {
            folder: self.path("fotos"),
            name: Some("casamento".into()),
            spec: fields.to_spec().unwrap(),
            spreads,
            reuse: Reuse::Medium,
            preferences: lamina_layout::Preferences::default(),
            learned: lamina_layout::Learned::default(),
            models: Vec::new(),
            limits: lamina_core::PageLimits::default(),
            export: lamina_core::ExportPrefs::default(),
            style: Default::default(),
            files: Vec::new(),
            build: true,
        }
    }

    /// O álbum montado com `spreads` lâminas.
    pub fn project(&self, spreads: usize) -> Project {
        let (built, _) =
            create_album(&self.new_album(Some(spreads)), &self.cache(), &|_| {}).unwrap();
        built.project
    }

    /// Grava `project` em `<base>/<rel>` (a pasta é criada) e devolve o caminho.
    pub fn save(&self, project: Project, rel: &str) -> PathBuf {
        let file = self.path(rel);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        Session::new_unsaved(project).save_as(&file).unwrap()
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// Roda `f` num pool de threads só dele: os testes de velocidade não esperam o pool global,
/// que os outros testes do mesmo binário ocupam em paralelo (AGENTS.md, regra 13).
pub fn isolated<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(f)
}
