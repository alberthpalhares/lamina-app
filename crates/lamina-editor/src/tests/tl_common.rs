//! Projetos pequenos para os cenários TL: uma lâmina, fotos sem arquivo (o motor só olha as
//! proporções).

use lamina_core::{AlbumEdge, AlbumSpec, Dpi, FrameContent, Mm, PhotoId, Project, RelRect};
use lamina_layout::{BuildOptions, auto_build};

use crate::Editor;

pub const H: (u32, u32) = (3000, 2000);
pub const V: (u32, u32) = (2000, 3000);

pub fn spec(width_cm: f64, height_cm: f64) -> AlbumSpec {
    AlbumSpec::new(
        Mm::from_cm(width_cm),
        Mm::from_cm(height_cm),
        Dpi::new(300).unwrap(),
    )
    .unwrap()
    .with_safe_margin(Mm(10.0))
    .unwrap()
}

/// Projeto com estas fotos, montado pela montagem automática em `spreads` lâminas.
pub fn built(spec: AlbumSpec, photos: &[(u32, u32)], spreads: usize) -> Project {
    let mut project = Project::new("teste", spec);
    for (i, (w, h)) in photos.iter().enumerate() {
        project.add_photo(format!("f{i:02}.jpg"), *w, *h);
    }
    auto_build(&project, spreads, &BuildOptions::default())
        .unwrap()
        .project
}

/// Álbum de 3 lâminas com 4 fotos cada (uma vertical a cada quatro).
pub fn three_spreads() -> Project {
    let photos: Vec<(u32, u32)> = (0..12).map(|i| if i % 4 == 3 { V } else { H }).collect();
    built(spec(30.0, 30.0), &photos, 3)
}

/// Como [`three_spreads`], com fotos inteiras: os quadros não chegam às bordas do corte.
pub fn three_spreads_whole() -> Project {
    let photos: Vec<(u32, u32)> = (0..12).map(|i| if i % 4 == 3 { V } else { H }).collect();
    built_whole(spec(30.0, 30.0), &photos, 3)
}

/// Como [`built`], com a montagem "fotos inteiras".
pub fn built_whole(spec: AlbumSpec, photos: &[(u32, u32)], spreads: usize) -> Project {
    let mut project = Project::new("teste", spec);
    for (i, (w, h)) in photos.iter().enumerate() {
        project.add_photo(format!("f{i:02}.jpg"), *w, *h);
    }
    project.mounting.crop = lamina_core::CropLevel::Whole;
    auto_build(&project, spreads, &BuildOptions::default())
        .unwrap()
        .project
}

/// Editor com o projeto, na lâmina `index`.
pub fn editor_at(project: Project, index: usize) -> Editor {
    let mut editor = Editor::new();
    editor.adopt(project);
    assert!(editor.session_mut().unwrap().go_to(index));
    editor
}

/// Quadros da lâmina atual: (foto, retângulo).
pub fn frames(editor: &Editor) -> Vec<(PhotoId, RelRect)> {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, .. } => Some((*photo, f.rect)),
            _ => None,
        })
        .collect()
}

/// Quadros da página (`left`) da lâmina atual.
pub fn page(editor: &Editor, left: bool) -> Vec<(PhotoId, RelRect)> {
    frames(editor)
        .into_iter()
        .filter(|(_, r)| (r.x() < 0.5) == left)
        .collect()
}

/// Começa com página simples.
pub fn single_start(spec: AlbumSpec) -> AlbumSpec {
    spec.with_edges(AlbumEdge::SinglePage, AlbumEdge::FullSpread)
}
