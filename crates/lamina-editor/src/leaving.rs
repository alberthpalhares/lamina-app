//! Tirar fotos do álbum pela grade (spec TF): as fotos saem da lista do projeto e de todas as
//! lâminas, que são refeitas como no "Tirar foto" (spec FQ, regra 3). O arquivo no disco nunca é
//! tocado (regra 3).

use lamina_core::{FrameContent, PhotoId, Project};

use crate::EditorError;
use crate::removing::remove_photos;

/// Quantas das fotos estão em alguma lâmina: com alguma, a janela pergunta antes (regra 4).
pub fn used_in_spreads(project: &Project, photos: &[PhotoId]) -> usize {
    photos
        .iter()
        .filter(|id| {
            project
                .all_content()
                .any(|s| s.frames.iter().any(|f| holds(&f.content, **id)))
        })
        .count()
}

fn holds(content: &FrameContent, id: PhotoId) -> bool {
    matches!(content, FrameContent::Photo { photo, .. } if *photo == id)
}

/// Tira as fotos do álbum: de cada lâmina que as usa (refeita, regra 5) e da lista de fotos.
pub fn remove_from_album(project: &mut Project, photos: &[PhotoId]) -> Result<(), EditorError> {
    for index in 0..project.spreads.len() {
        let frames: Vec<usize> = project.spreads[index]
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| photos.iter().any(|id| holds(&f.content, *id)))
            .map(|(i, _)| i)
            .collect();
        if !frames.is_empty() {
            remove_photos(project, index, &frames)?;
        }
    }
    // Na capa, o quadro sai com a foto: o arquivo não pode citar uma foto que saiu do álbum.
    if let Some(cover) = project.cover.as_mut() {
        cover
            .content
            .frames
            .retain(|f| !photos.iter().any(|id| holds(&f.content, *id)));
    }
    project.photos.retain(|p| !photos.contains(&p.id));
    Ok(())
}

/// O botão que confirma a pergunta, pela grade e pelo menu do quadro (TF-B11).
pub const REMOVE_BUTTON: &str = "Remover do álbum";

/// O nome do recurso, igual na grade, no menu do quadro e no histórico: "Remover fotografia do
/// álbum" ou "Remover 3 fotografias do álbum" (correções do dono de 2026-10-29, TF-B11 e TF-B12).
pub fn remove_label(count: usize) -> String {
    if count <= 1 {
        "Remover fotografia do álbum".to_owned()
    } else {
        format!("Remover {count} fotografias do álbum")
    }
}

/// A pergunta da regra 4, ou nenhuma quando nenhuma foto está em lâmina.
pub fn leave_question(used: usize) -> Option<String> {
    match used {
        0 => None,
        1 => Some(
            "1 foto está em lâminas. Ela sai das lâminas e o layout delas é refeito. O arquivo \
             continua no computador."
                .to_owned(),
        ),
        n => Some(format!(
            "{n} fotos estão em lâminas. Elas saem das lâminas e o layout delas é refeito. O \
             arquivo continua no computador."
        )),
    }
}
