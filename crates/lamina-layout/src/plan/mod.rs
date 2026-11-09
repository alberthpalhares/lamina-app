//! Plano de lâminas (spec `specs/plano-de-laminas.md`): quais fotos vão em cada lâmina, em ordem
//! cronológica, agrupadas por momento e com ritmo (destaque seguido de lâminas densas).

mod allocate;
mod distribute;
mod moments;
mod suggest;

use lamina_core::{Edges, Photo, PhotoId};
use thiserror::Error;

use allocate::{allocate, capacities, min_spreads};
use distribute::distribute;
use moments::Moments;
pub use suggest::{PHOTOS_PER_SPREAD, suggest_spreads};

/// Opções do plano. Os padrões vêm da spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanOptions {
    /// Intervalo, em segundos, acima do qual começa um novo momento (padrão 20 min).
    pub moment_gap_secs: i64,
    /// Fotos por página ativa (padrão 4).
    pub max_per_page: usize,
}

impl Default for PlanOptions {
    fn default() -> Self {
        PlanOptions {
            moment_gap_secs: 20 * 60,
            max_per_page: 4,
        }
    }
}

/// Papel de uma lâmina no ritmo do momento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadRole {
    /// Primeira lâmina do momento, com poucas fotos grandes.
    Highlight,
    /// Lâmina que segue o destaque, com as demais fotos.
    Dense,
    /// O momento cabe numa lâmina só.
    Single,
}

/// Uma lâmina do plano.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadPlan {
    /// Índice do momento (0, 1, 2…).
    pub moment: usize,
    pub role: SpreadRole,
    /// Fotos da lâmina, em ordem cronológica.
    pub photos: Vec<PhotoId>,
}

/// Erros do plano, com as mensagens da spec.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanError {
    #[error("Nenhuma foto para diagramar.")]
    NoPhotos,
    #[error("O álbum precisa de pelo menos 1 lâmina.")]
    NoSpreads,
    #[error("Fotos por página: use de 1 a {max}, não {value}.")]
    InvalidMaxPerPage { value: usize, max: usize },
    #[error("Há {photos} fotos para {spreads} lâminas: use no máximo {photos} lâminas.")]
    MoreSpreadsThanPhotos { photos: usize, spreads: usize },
    #[error(
        "São {photos} fotos e cabem {capacity} em {spreads} lâminas: use pelo menos {min_spreads} lâminas."
    )]
    TooManyPhotos {
        photos: usize,
        capacity: usize,
        spreads: usize,
        min_spreads: usize,
    },
}

/// Monta o plano de `spreads` lâminas para `photos`.
pub fn plan_spreads(
    photos: &[Photo],
    spreads: usize,
    edges: Edges,
    options: &PlanOptions,
) -> Result<Vec<SpreadPlan>, PlanError> {
    plan_spreads_ordered(photos, spreads, edges, options, &[])
}

/// Como [`plan_spreads`], mas com a ordem das fotos escolhida (spec OF, beta 9.1).
pub fn plan_spreads_ordered(
    photos: &[Photo],
    spreads: usize,
    edges: Edges,
    options: &PlanOptions,
    order: &[PhotoId],
) -> Result<Vec<SpreadPlan>, PlanError> {
    validate(photos.len(), spreads, edges, options)?;
    let caps = capacities(spreads, edges, options.max_per_page);
    let ordered = moments::ordered(photos, order);
    let mut moments = Moments::split(&ordered, options.moment_gap_secs);
    moments.merge_until(spreads);
    // Se um momento não couber nem tomando lâminas dos outros, ele se une ao vizinho mais
    // próximo. Com um momento só sempre cabe (validado acima).
    let counts = loop {
        match allocate(&moments.sizes(), &caps) {
            Ok(counts) => break counts,
            Err(_) if moments.len() == 1 => {
                return Err(too_many(photos.len(), &caps, edges, options));
            }
            Err(i) => moments.merge_with_closest_neighbor(i),
        }
    };

    let mut plan = Vec::with_capacity(spreads);
    let mut first_spread = 0;
    for (moment, (group, k)) in moments.groups.iter().zip(counts).enumerate() {
        let sizes = distribute(group.len(), &caps[first_spread..first_spread + k]);
        let mut photos_left = group.iter().map(|&i| ordered[i].id);
        for (j, size) in sizes.into_iter().enumerate() {
            let role = match (k, j) {
                (1, _) => SpreadRole::Single,
                (_, 0) => SpreadRole::Highlight,
                _ => SpreadRole::Dense,
            };
            let photos = photos_left.by_ref().take(size).collect();
            plan.push(SpreadPlan {
                moment,
                role,
                photos,
            });
        }
        first_spread += k;
    }
    Ok(plan)
}

/// Erros de entrada da seção 6 da spec.
/// Confere se `spreads` lâminas servem para `photos` fotos, com as mensagens da spec (a janela
/// "Novo álbum" mostra o erro antes de montar; spec JP, seção 6).
pub fn validate(
    photos: usize,
    spreads: usize,
    edges: Edges,
    options: &PlanOptions,
) -> Result<(), PlanError> {
    if photos == 0 {
        return Err(PlanError::NoPhotos);
    }
    if spreads == 0 {
        return Err(PlanError::NoSpreads);
    }
    if spreads > photos {
        return Err(PlanError::MoreSpreadsThanPhotos { photos, spreads });
    }
    if !(1..=crate::layouts::MAX_PER_PAGE).contains(&options.max_per_page) {
        return Err(PlanError::InvalidMaxPerPage {
            value: options.max_per_page,
            max: crate::layouts::MAX_PER_PAGE,
        });
    }
    let caps = capacities(spreads, edges, options.max_per_page);
    if photos > caps.iter().sum() {
        return Err(too_many(photos, &caps, edges, options));
    }
    Ok(())
}

fn too_many(photos: usize, caps: &[usize], edges: Edges, options: &PlanOptions) -> PlanError {
    PlanError::TooManyPhotos {
        photos,
        capacity: caps.iter().sum(),
        spreads: caps.len(),
        min_spreads: min_spreads(photos, edges, options.max_per_page),
    }
}

#[cfg(test)]
mod tests;
