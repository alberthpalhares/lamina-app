//! Cenários AL-B1, AL-B6, AL-B7 e AL-B9 da spec `specs/aprendizado-de-layouts.md`: o Auto Build.

use lamina_core::{FrameContent, PhotoId, Project};

use super::tests::forty;
use super::window::window;
use super::*;
use crate::layouts::{FamilyKind, layout_options};
use crate::prefs::model_key;

/// As opções de cada lâmina do álbum montado, com as fotos dela.
fn options_of(built: &AutoBuild) -> Vec<(Vec<f64>, Vec<LayoutCandidate>)> {
    let project = &built.project;
    let edges = project.spec.edges();
    project
        .spreads
        .iter()
        .enumerate()
        .map(|(i, spread)| {
            let photos: Vec<PhotoId> = spread
                .frames
                .iter()
                .filter_map(|f| match &f.content {
                    FrameContent::Photo { photo, .. } => Some(*photo),
                    _ => None,
                })
                .collect();
            let aspects: Vec<f64> = photos
                .iter()
                .map(|p| project.photo(*p).unwrap().aspect_ratio())
                .collect();
            let active = edges.active_pages(i, project.spreads.len());
            let options = LayoutOptions {
                gap: project.gap,
                mounting: project.mounting,
                ..LayoutOptions::default()
            };
            let list = layout_options(&project.spec, active, &aspects, &options)
                .unwrap()
                .in_order;
            (aspects, list)
        })
        .collect()
}

/// O álbum com peso alto nos desenhos que perdem por menos de 6 pontos em cada lâmina.
fn taught(project: &Project) -> (AutoBuild, BuildOptions) {
    let base = auto_build(project, 10, &BuildOptions::default()).unwrap();
    let mut weights = Vec::new();
    for (report, (aspects, list)) in base.report.iter().zip(options_of(&base)) {
        for c in list.iter().take(12) {
            let (key, behind) = (design_key(c, &aspects), report.score - c.score.total);
            if key != report.design && behind > 0.0 && behind < 6.0 {
                weights.push((key, 12_000));
            }
        }
    }
    assert!(!weights.is_empty(), "nenhum desenho perto do escolhido");
    let options = BuildOptions {
        learned: Learned::from_weights(weights),
        ..BuildOptions::default()
    };
    (auto_build(project, 10, &options).unwrap(), options)
}

#[test]
fn al_b1_o_auto_build_sem_aprendizado_e_o_de_sempre() {
    let project = forty();
    let base = auto_build(&project, 10, &BuildOptions::default()).unwrap();
    let unknown = BuildOptions {
        learned: Learned::from_weights([("nenhum desenho".to_owned(), 50_000)]),
        ..BuildOptions::default()
    };
    let got = auto_build(&project, 10, &unknown).unwrap();
    assert_eq!(got.project, base.project);
    assert_eq!(got.report, base.report);
}

#[test]
fn al_b6_modelo_nao_ganha_bonus_no_auto_build() {
    let c = LayoutCandidate {
        family: Family::Model {
            name: "Casamento".to_owned(),
            mirrored: false,
        },
        breathing: crate::layouts::Breathing::default(),
        frames: Vec::new(),
        order: Vec::new(),
        crop: Vec::new(),
        coverage: 0.0,
        score: crate::layouts::Score::default(),
        model: None,
    };
    let key = model_key("Casamento", false);
    let learned = Learned::from_weights([(key, 200_000)]);
    let got = window(vec![c], &[], &Preferences::default(), &learned);
    assert_eq!(got.boost[0].learned, 0.0);
}

#[test]
fn al_b7_o_auto_build_aprende_sem_ficar_igual() {
    let project = forty();
    let (built, _) = taught(&project);
    assert!(
        built
            .report
            .iter()
            .any(|r| r.reason.contains("costuma usar")),
        "nenhuma lâmina usou o aprendido: {:?}",
        built.report
    );
    let families: Vec<&Family> = built.report.iter().map(|r| &r.family).collect();
    let outlines = |w: &[&Family]| w[0].same_outline(w[1]);
    let neighbors = families.windows(2).filter(|w| outlines(w)).count();
    assert_eq!(neighbors, 0, "vizinhas com o mesmo desenho: {families:?}");
    let edge = |f: &&Family| {
        matches!(
            f.kind(),
            FamilyKind::Across | FamilyKind::BleedPage | FamilyKind::FullBleed
        )
    };
    assert!(
        !families.windows(3).any(|w| w.iter().all(edge)),
        "três seguidas até a borda: {families:?}"
    );
}

#[test]
fn al_b9_o_auto_build_nao_muda_sozinho() {
    let project = forty();
    let (first, options) = taught(&project);
    let second = auto_build(&project, 10, &options).unwrap();
    assert_eq!(first.project, second.project);
    // O relógio não entra na conta (spec AL, regra 15).
    let source = include_str!("../../learned.rs");
    for clock in ["SystemTime", "chrono", "Instant"] {
        assert!(!source.contains(clock), "`{clock}` no aprendizado");
    }
}
