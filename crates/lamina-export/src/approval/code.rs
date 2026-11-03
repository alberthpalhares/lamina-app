//! O código da prova (spec AC, regra 20): 6 caracteres que mudam quando o que o cliente vê muda, e
//! só então. Serve para distinguir versões, não para segurança.

use std::collections::HashSet;

use lamina_core::{FrameContent, Photo, PhotoId, Project};

use crate::ExportError;

/// O conteúdo que o cliente vê, em ordem fixa: especificação, espaço entre fotos, lâminas, capa sem
/// o guia e as fotos usadas. Caminhos, nome do álbum e preferências ficam de fora.
pub fn proof_code(project: &Project) -> Result<String, ExportError> {
    let cover = project.cover.clone().map(|mut cover| {
        cover.guide = None;
        cover
    });
    let content = (
        &project.spec,
        project.gap,
        &project.spreads,
        &cover,
        used_photos(project),
    );
    let json = serde_json::to_vec(&content).map_err(|e| ExportError::Encode(e.to_string()))?;
    Ok(blake3::hash(&json).to_hex()[..6].to_uppercase())
}

/// O que identifica uma foto: a impressão digital; sem ela, o nome do arquivo e o tamanho.
type Identity = (String, u32, u32);

/// As fotos usadas no álbum e na capa, em ordem de id, cada uma com os ajustes e a identidade.
fn used_photos(project: &Project) -> Vec<(PhotoId, &lamina_core::PhotoAdjust, Identity)> {
    let used: HashSet<PhotoId> = project
        .all_content()
        .flat_map(|spread| &spread.frames)
        .filter_map(|frame| match &frame.content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        })
        .collect();
    let mut photos: Vec<&Photo> = project
        .photos
        .iter()
        .filter(|photo| used.contains(&photo.id))
        .collect();
    photos.sort_by_key(|photo| photo.id);
    photos
        .into_iter()
        .map(|photo| (photo.id, &photo.adjust, identity(photo)))
        .collect()
}

fn identity(photo: &Photo) -> Identity {
    match &photo.fingerprint {
        Some(fingerprint) => (fingerprint.clone(), 0, 0),
        None => {
            let name = photo.path.file_name().unwrap_or(photo.path.as_os_str());
            (
                name.to_string_lossy().into_owned(),
                photo.width_px,
                photo.height_px,
            )
        }
    }
}
