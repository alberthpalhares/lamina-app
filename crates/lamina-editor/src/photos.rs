//! O navegador de fotos (spec JP, seção 4, item 4): as fotos em ordem de captura, com filtro e
//! número de usos.

use std::collections::HashMap;
use std::path::Path;

use lamina_core::{FrameContent, PhotoId, Project};

/// Quais fotos o navegador mostra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Used,
    Unused,
}

/// Uma foto no navegador.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoItem {
    pub id: PhotoId,
    pub name: String,
    /// Em quantos quadros a foto aparece.
    pub uses: usize,
    /// A primeira lâmina onde aparece (a partir de 0).
    pub first_spread: Option<usize>,
}

/// Os dados de uma foto, para o painel de informações.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Data e hora da captura, ou "—".
    pub taken_at: String,
    pub camera: String,
    pub uses: usize,
}

/// Usos de cada foto e a primeira lâmina de cada uma.
fn usage(project: &Project) -> HashMap<PhotoId, (usize, usize)> {
    let mut map: HashMap<PhotoId, (usize, usize)> = HashMap::new();
    for (index, spread) in project.all_content().enumerate() {
        // A capa vem depois das lâminas: uma foto só nela aponta para a lâmina 1 (spec CA, regra 11).
        let at = index.min(project.spreads.len().saturating_sub(1));
        for frame in &spread.frames {
            if let FrameContent::Photo { photo, .. } = &frame.content {
                map.entry(*photo).or_insert((0, at)).0 += 1;
            }
        }
    }
    map
}

/// As fotos do filtro, em ordem de captura (sem data no fim, por nome).
pub fn photo_list(project: &Project, filter: Filter) -> Vec<PhotoItem> {
    let usage = usage(project);
    let mut photos: Vec<_> = project.photos.iter().collect();
    photos.sort_by_key(|&p| (p.taken_at.is_none(), p.taken_at, &p.path));
    photos
        .into_iter()
        .map(|p| {
            let (uses, first) = usage.get(&p.id).copied().unwrap_or_default();
            PhotoItem {
                id: p.id,
                name: name_of(&p.path),
                uses,
                first_spread: (uses > 0).then_some(first),
            }
        })
        .filter(|item| match filter {
            Filter::All => true,
            Filter::Used => item.uses > 0,
            Filter::Unused => item.uses == 0,
        })
        .collect()
}

/// A lista de fotos sairia igual nos dois projetos: as mesmas fotos, cada uma nas mesmas lâminas.
/// Mexer só num quadro não refaz a lista (spec LV, regra 6).
pub fn same_photo_list(a: &Project, b: &Project) -> bool {
    fn uses(project: &Project) -> impl Iterator<Item = (usize, PhotoId)> + '_ {
        let frames = project.all_content().enumerate();
        frames.flat_map(|(index, spread)| {
            spread
                .frames
                .iter()
                .filter_map(move |frame| match &frame.content {
                    FrameContent::Photo { photo, .. } => Some((index, *photo)),
                    _ => None,
                })
        })
    }
    a.photos == b.photos && uses(a).eq(uses(b))
}

/// Quantas fotos diferentes aparecem em algum quadro.
pub fn used_photos(project: &Project) -> usize {
    usage(project).len()
}

/// Os dados da foto `id`, ou `None` se ela não está no projeto.
pub fn photo_info(project: &Project, id: PhotoId) -> Option<PhotoInfo> {
    let photo = project.photo(id).ok()?;
    let uses = usage(project).get(&id).map_or(0, |u| u.0);
    Some(PhotoInfo {
        name: name_of(&photo.path),
        width: photo.width_px,
        height: photo.height_px,
        taken_at: photo
            .taken_at
            .map_or_else(|| "—".to_owned(), |t| t.to_string()),
        camera: photo.camera.clone().unwrap_or_else(|| "—".to_owned()),
        uses,
    })
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}
