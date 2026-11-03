//! Auto Build (spec `specs/auto-build.md`): monta o álbum inteiro. O plano de lâminas decide as
//! fotos de cada lâmina e, para cada uma, um layout é escolhido entre as primeiras sugestões da
//! variedade, cuidando de ritmo e variedade, com um relatório do porquê.

mod choose;
mod window;

use lamina_core::{CoreError, Crop, FrameContent, PhotoId, Project};
use rayon::prelude::*;
use thiserror::Error;

use crate::layouts::{
    Family, LayoutCandidate, LayoutError, LayoutOptions, add_models, layout_options_learned,
};
use crate::learned::Learned;
use crate::plan::{PlanError, PlanOptions, SpreadPlan, SpreadRole, plan_spreads};
use crate::prefs::{Preferences, design_key};
use window::{WINDOW, Window, window};

/// Quanto o Auto Build aceita repetir o mesmo desenho geral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reuse {
    /// Pouca repetição: mais variedade.
    Low,
    #[default]
    Medium,
    /// Repete à vontade: o álbum fica mais uniforme.
    High,
}

/// Opções do Auto Build.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildOptions {
    pub reuse: Reuse,
    pub plan: PlanOptions,
    /// Favoritos e rejeitados do fotógrafo (spec TL, regra 8). Vazio = escolha de sempre.
    pub preferences: Preferences,
    /// Modelos do fotógrafo que entram na disputa de cada lâmina (spec MD, regra 9).
    pub models: Vec<crate::layouts::SpreadModel>,
    /// O que o fotógrafo mais escolhe (spec AL, regra 5). Vazio = escolha de sempre.
    pub learned: Learned,
}

/// Uma linha do relatório: o que foi escolhido para a lâmina e por quê.
#[derive(Debug, Clone, PartialEq)]
pub struct SpreadReport {
    pub moment: usize,
    pub role: SpreadRole,
    pub photos: usize,
    pub family: Family,
    /// Nota da sugestão escolhida (spec VL, regra 6).
    pub score: f64,
    /// Posição da escolhida na lista de sugestões (1 = a primeira).
    pub position: usize,
    /// Motivo, em português.
    pub reason: String,
    /// Identificador do desenho escolhido (spec TL, regra 6).
    pub design: String,
}

/// O álbum montado e o relatório.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoBuild {
    pub project: Project,
    pub report: Vec<SpreadReport>,
}

/// Erros do Auto Build: os do plano, da variedade e do núcleo passam com as mesmas mensagens.
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("A foto {0} não tem tamanho válido.")]
    InvalidPhoto(String),
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// Monta `spreads` lâminas com as fotos de `project` (as lâminas antigas são substituídas).
pub fn auto_build(
    project: &Project,
    spreads: usize,
    options: &BuildOptions,
) -> Result<AutoBuild, BuildError> {
    if let Some(bad) = project
        .photos
        .iter()
        .find(|p| p.width_px == 0 || p.height_px == 0)
    {
        let name = bad.path.file_name().unwrap_or(bad.path.as_os_str());
        return Err(BuildError::InvalidPhoto(
            name.to_string_lossy().into_owned(),
        ));
    }
    let edges = project.spec.edges();
    let plan = plan_spreads(&project.photos, spreads, edges, &options.plan)?;
    let suggestions = suggestions(project, &plan, options)?;
    let mut out = project.clone();
    out.spreads.clear();
    let mut chosen: Vec<Family> = Vec::with_capacity(plan.len());
    let mut report = Vec::with_capacity(plan.len());
    for (spread, Window { list, boost }) in plan.iter().zip(&suggestions) {
        let pick = choose::pick(list, boost, spread, &chosen, options.reuse);
        // A variedade nunca devolve lista vazia (seria `GapTooLarge`); conferido mesmo assim.
        let candidate = list.get(pick.index).ok_or(LayoutError::GapTooLarge)?;
        place(&mut out, &spread.photos, candidate)?;
        let family = candidate.family.clone();
        let aspects: Vec<f64> = spread
            .photos
            .iter()
            .filter_map(|id| out.photo(*id).ok().map(lamina_core::Photo::aspect_ratio))
            .collect();
        report.push(SpreadReport {
            design: design_key(candidate, &aspects),
            moment: spread.moment,
            role: spread.role,
            photos: spread.photos.len(),
            family: family.clone(),
            score: candidate.score.total,
            position: pick.index + 1,
            reason: pick.reason,
        });
        chosen.push(family);
    }
    Ok(AutoBuild {
        project: out,
        report,
    })
}

/// As primeiras sugestões "na ordem" de cada lâmina, calculadas em paralelo. Com favoritos, a
/// lista inteira é calculada para achá-los mesmo depois das primeiras.
fn suggestions(
    project: &Project,
    plan: &[SpreadPlan],
    build: &BuildOptions,
) -> Result<Vec<Window>, BuildError> {
    let prefs = &build.preferences;
    let edges = project.spec.edges();
    let options = LayoutOptions {
        max_candidates: if prefs.favorites.is_empty() {
            WINDOW
        } else {
            LayoutOptions::default().max_candidates
        },
        reordered: false,
        gap: project.gap,
        mounting: project.mounting,
    };
    let results: Vec<Result<Window, BuildError>> = plan
        .par_iter()
        .enumerate()
        .map(|(i, spread)| {
            let aspects = spread
                .photos
                .iter()
                .map(|id| Ok(project.photo(*id)?.aspect_ratio()))
                .collect::<Result<Vec<f64>, CoreError>>()?;
            let active = edges.active_pages(i, plan.len());
            let mut lists =
                layout_options_learned(&project.spec, active, &aspects, &options, &build.learned)?;
            let at = (active, project.gap);
            add_models(&mut lists, &project.spec, at, &aspects, &build.models)?;
            Ok(window(lists.in_order, &aspects, prefs, &build.learned))
        })
        .collect();
    // Em paralelo, mas o erro devolvido é o da primeira lâmina com problema (determinismo).
    results.into_iter().collect()
}

/// Cria a lâmina com um quadro por foto, na ordem do plano, com recorte centralizado.
fn place(
    project: &mut Project,
    photos: &[PhotoId],
    candidate: &LayoutCandidate,
) -> Result<(), BuildError> {
    let spread = project.add_spread();
    for (photo, rect) in photos.iter().zip(&candidate.frames) {
        let content = FrameContent::Photo {
            photo: *photo,
            crop: Crop::default(),
        };
        project.add_frame(spread, *rect, content)?;
    }
    if let Some(look) = &candidate.model {
        dress(project, spread, look)?;
        if let (Family::Model { name, mirrored }, Some(s)) =
            (&candidate.family, project.spreads.last_mut())
        {
            s.model = Some(crate::prefs::model_key(name, *mirrored));
        }
    }
    Ok(())
}

/// Borda, cantos e textos de um modelo na lâmina recém-montada (spec MD, regra 6).
fn dress(
    project: &mut Project,
    spread: lamina_core::SpreadId,
    look: &crate::layouts::ModelLook,
) -> Result<(), BuildError> {
    if let Some(s) = project.spreads.iter_mut().find(|s| s.id == spread) {
        for (frame, (border, radius)) in s.frames.iter_mut().zip(&look.styles) {
            frame.border = *border;
            frame.radius = *radius;
        }
    }
    for t in &look.texts {
        project.add_frame(spread, t.rect, FrameContent::Text(t.text.clone()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "tests/al.rs"]
mod tests_al;

#[cfg(test)]
#[path = "tests/more.rs"]
mod tests_more;
#[cfg(test)]
#[path = "tests/np.rs"]
mod tests_np;
#[cfg(test)]
#[path = "tests/prefs.rs"]
mod tests_prefs;
