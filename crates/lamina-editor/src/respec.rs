//! Trocar a especificação de um álbum pronto (spec PR, regras 6 e 7): outro tamanho, outra
//! sangria, outras pontas. Cada lâmina é refeita na proporção nova com o mesmo desenho
//! (`design_key`), as mesmas fotos e os mesmos centros de recorte, como ao mudar o espaço (AP).

use lamina_core::{AlbumSpec, FrameContent, PageLimits, Project};
use lamina_layout::design_key;

use crate::edits::{layout_with, spread_photos};
use crate::suggestions::{Part, apply, is_in_use, lists, selection};
use crate::{Editor, EditorError};

/// O desenho em uso na lâmina `index`, se ela tem fotos e o desenho está nas listas.
pub(crate) fn current_design(project: &Project, index: usize) -> Option<String> {
    let sel = selection(project, index, Part::Spread).ok()?;
    let lists = lists(project, &sel).ok()?;
    lists
        .in_order
        .iter()
        .chain(&lists.reordered)
        .find(|c| is_in_use(project, index, &sel, c))
        .map(|c| design_key(c, &sel.aspects))
}

/// Troca a especificação e refaz as lâminas. Devolve as lâminas (contadas a partir de 1) que
/// mudaram de desenho porque o desenho antigo não coube.
pub fn respec(project: &mut Project, spec: AlbumSpec) -> Result<Vec<usize>, EditorError> {
    let designs: Vec<Option<String>> = (0..project.spreads.len())
        .map(|i| current_design(project, i))
        .collect();
    project.spec = spec;
    // A marca de uma página que deixou de existir sai (spec PB, regra 3).
    crate::blank::clear_lost_marks(project);
    let mut changed = Vec::new();
    for (index, design) in designs.into_iter().enumerate() {
        let photos = spread_photos(project, index);
        if photos.is_empty() {
            continue;
        }
        // Lâmina à mão: os quadros ficam como estão; só os recortes voltam ao zoom 1, com os
        // mesmos centros (spec TX, regra 8).
        if project.spreads[index].manual || project.spreads[index].model.is_some() {
            for frame in &mut project.spreads[index].frames {
                if let FrameContent::Photo { crop, .. } = &mut frame.content {
                    *crop = crate::suggestions::keep_center(*crop);
                }
            }
            continue;
        }
        let kept = match selection(project, index, Part::Spread) {
            Ok(sel) => {
                let lists = lists(project, &sel)?;
                let all: Vec<_> = lists.in_order.iter().chain(&lists.reordered).collect();
                let same = design
                    .as_ref()
                    .and_then(|k| all.iter().find(|c| design_key(c, &sel.aspects) == *k));
                match same.or(all.first()) {
                    Some(chosen) => {
                        let chosen = (*chosen).clone();
                        apply(project, index, &sel, &chosen)?;
                        same.is_some()
                    }
                    None => false,
                }
            }
            // Fotos numa página que deixou de existir (pontas, regra 7): ficam na mesma lâmina,
            // arrumadas na página que sobrou.
            Err(_) => {
                crate::blank::free_stuck_photos(project, index, &photos);
                layout_with(project, index, &photos)?;
                false
            }
        };
        if !kept {
            changed.push(index + 1);
        }
    }
    Ok(changed)
}

/// "2 lâminas mudaram de desenho: 4 e 7." (nada se todas mantiveram o desenho).
pub fn changed_message(changed: &[usize]) -> Option<String> {
    let list: Vec<String> = changed.iter().map(ToString::to_string).collect();
    match list.as_slice() {
        [] => None,
        [one] => Some(format!("1 lâmina mudou de desenho: {one}.")),
        [rest @ .., last] => Some(format!(
            "{} lâminas mudaram de desenho: {} e {last}.",
            list.len(),
            rest.join(", ")
        )),
    }
}

impl Editor {
    /// "Aplicar" na janela "Especificação do álbum": uma edição só ("Trocar a especificação"),
    /// que um Ctrl+Z desfaz inteira. Devolve o aviso das lâminas que mudaram de desenho.
    pub fn respec(
        &mut self,
        spec: AlbumSpec,
        limits: PageLimits,
    ) -> Result<Option<String>, EditorError> {
        self.exit_crop();
        self.select_frame(None);
        let session = self.session_mut().ok_or(EditorError::NoProject)?;
        let mut changed = Vec::new();
        session.edit("Trocar a especificação · todas as lâminas", |p| {
            changed = respec(p, spec)?;
            p.page_limits = limits;
            Ok::<(), EditorError>(())
        })?;
        Ok(changed_message(&changed))
    }
}
