//! Lâminas novas, apagadas e reordenadas (spec FQ, regras 6 e 7), com as pontas de página
//! simples rearranjadas.

use lamina_core::Project;

use crate::EditorError;
use crate::blank::clear_lost_marks;
use crate::edits::{layout_with, spread_photos};

/// Lâmina vazia nova depois de `after`; devolve a posição dela (regra 6).
pub fn insert_spread(project: &mut Project, after: usize) -> usize {
    project.add_spread();
    let at = (after + 1).min(project.spreads.len() - 1);
    if let Some(new) = project.spreads.pop() {
        project.spreads.insert(at, new);
    }
    at
}

/// Apaga a lâmina `index`; o álbum fica com pelo menos 1 (regra 6).
pub fn delete_spread(project: &mut Project, index: usize) -> Result<(), EditorError> {
    if project.spreads.len() <= 1 || index >= project.spreads.len() {
        return Err(EditorError::NoFrame);
    }
    project.spreads.remove(index);
    clear_lost_marks(project);
    fix_edges(project)
}

/// Muda a lâmina `from` para a posição `to` (regra 7).
pub fn move_spread(project: &mut Project, from: usize, to: usize) -> Result<(), EditorError> {
    let n = project.spreads.len();
    if from >= n || to >= n || from == to {
        return Err(EditorError::NoFrame);
    }
    let before = pages_by_id(project);
    let spread = project.spreads.remove(from);
    project.spreads.insert(to, spread);
    clear_lost_marks(project);
    fix_changed(project, &before)
}

type Pages = Vec<(lamina_core::SpreadId, (bool, bool))>;

fn pages_by_id(project: &Project) -> Pages {
    let edges = project.spec.edges();
    let n = project.spreads.len();
    project
        .spreads
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let a = edges.active_pages(i, n);
            (s.id, (a.left, a.right))
        })
        .collect()
}

/// Rearranja as lâminas cujas páginas ativas mudaram (chegaram ou saíram de uma ponta).
fn fix_changed(project: &mut Project, before: &Pages) -> Result<(), EditorError> {
    let after = pages_by_id(project);
    for (index, (id, pages)) in after.iter().enumerate() {
        let old = before.iter().find(|(b, _)| b == id).map(|(_, p)| *p);
        if old.is_some_and(|o| o != *pages) {
            let photos = spread_photos(project, index);
            crate::blank::free_stuck_photos(project, index, &photos);
            layout_with(project, index, &photos)?;
        }
    }
    Ok(())
}

/// Depois de apagar, as novas pontas podem ter mudado de páginas ativas.
fn fix_edges(project: &mut Project) -> Result<(), EditorError> {
    let last = project.spreads.len() - 1;
    for index in [0, last] {
        let active = project
            .spec
            .edges()
            .active_pages(index, project.spreads.len());
        let on_inactive = project.spreads[index].frames.iter().any(|f| {
            let left = f.rect.x() + f.rect.width() <= 0.5 + 1e-6;
            let right = f.rect.x() >= 0.5 - 1e-6;
            (!active.left && !right) || (!active.right && !left)
        });
        if on_inactive {
            let photos = spread_photos(project, index);
            layout_with(project, index, &photos)?;
        }
    }
    Ok(())
}

/// Põe as lâminas `new` depois da última ("Montar o álbum…", spec BR, regra 7): a antiga última,
/// se deixou de ser ponta com página simples, é rearranjada com a 1ª opção do motor.
pub fn append_spreads(
    project: &mut Project,
    new: Vec<lamina_core::Spread>,
) -> Result<(), EditorError> {
    let before = pages_by_id(project);
    project.spreads.extend(new);
    clear_lost_marks(project);
    fix_changed(project, &before)
}

/// Lâmina nova na posição `at`, com as páginas dela em branco (spec PB, regra 6). A vizinha que
/// deixou de ser ponta é rearranjada.
pub fn insert_blank(project: &mut Project, at: usize) -> Result<(), EditorError> {
    let before = pages_by_id(project);
    project.add_spread();
    let at = at.min(project.spreads.len() - 1);
    if let Some(new) = project.spreads.pop() {
        project.spreads.insert(at, new);
    }
    let active = project.spec.edges().active_pages(at, project.spreads.len());
    project.spreads[at].blank = lamina_core::BlankPages {
        left: active.left,
        right: active.right,
    };
    clear_lost_marks(project);
    fix_changed(project, &before)
}
