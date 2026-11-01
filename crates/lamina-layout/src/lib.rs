//! # lamina-layout
//!
//! O motor "smart" de diagramação.
//!
//! - [`plan`]: plano de lâminas, quais fotos vão em cada lâmina (spec `plano-de-laminas`).
//! - [`layouts`]: opções de arranjo para as fotos de uma lâmina (specs `gerador-de-layouts` e
//!   `variedade-de-layouts`).
//! - [`grid`]: gerador de grades em coordenadas relativas.
//!
//! - [`build`]: Auto Build, o álbum inteiro com um layout por lâmina (spec `auto-build`).

pub mod build;
pub mod layouts;
pub mod learned;
pub mod plan;
pub mod prefs;

pub use build::{AutoBuild, BuildError, BuildOptions, Reuse, SpreadReport, auto_build};
pub use layouts::{
    Align, Arrangement, Breathing, DEFAULT_GAP, Family, FamilyKind, FullBleed, LayoutCandidate,
    LayoutError, LayoutLists, LayoutOptions, MAX_PER_PAGE, ModelLook, ModelSlot, ModelText,
    PageLayout, Scope, Score, SpreadModel, SupportSide, add_models, assign, layout_candidates,
    layout_options, layout_options_learned,
};
pub use learned::{Learned, MAX_LEARNED_BONUS, WEIGHT_PER_USE, learned_bonus};
pub use plan::{
    PHOTOS_PER_SPREAD, PlanError, PlanOptions, SpreadPlan, SpreadRole, plan_spreads,
    suggest_spreads, validate as validate_plan,
};
pub use prefs::{FAVORITE_BONUS, Preferences, Preferred, apply_preferences, design_key, model_key};

use lamina_core::{CoreError, RelRect};

/// Gera uma grade `rows × cols` de quadros iguais, com `gap` (fração da área) entre eles
/// e nas bordas.
pub fn grid(rows: u16, cols: u16, gap: f64) -> Result<Vec<RelRect>, CoreError> {
    if rows == 0 || cols == 0 {
        return Ok(Vec::new());
    }
    let (rows_f, cols_f) = (f64::from(rows), f64::from(cols));
    let width = (1.0 - gap * (cols_f + 1.0)) / cols_f;
    let height = (1.0 - gap * (rows_f + 1.0)) / rows_f;
    let mut frames = Vec::with_capacity(usize::from(rows) * usize::from(cols));
    for row in 0..rows {
        for col in 0..cols {
            let x = gap + f64::from(col) * (width + gap);
            let y = gap + f64::from(row) * (height + gap);
            frames.push(RelRect::new(x, y, width, height)?);
        }
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grade_2x2_sem_espaco_divide_em_quartos() {
        let frames = grid(2, 2, 0.0).unwrap();
        assert_eq!(frames.len(), 4);
        assert_eq!(frames[3], RelRect::new(0.5, 0.5, 0.5, 0.5).unwrap());
    }

    #[test]
    fn grade_com_espaco_respeita_as_bordas() {
        let frames = grid(1, 3, 0.02).unwrap();
        let last = frames[2];
        assert!((frames[0].x() - 0.02).abs() < 1e-9);
        assert!((last.x() + last.width() - 0.98).abs() < 1e-9);
    }

    #[test]
    fn espaco_grande_demais_e_erro() {
        assert!(matches!(grid(1, 4, 0.3), Err(CoreError::InvalidRelRect(_))));
    }
}
