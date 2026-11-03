//! TL-B9 (spec `specs/trocar-layouts.md`): a montagem automática com favoritos e rejeitados.

use std::collections::HashMap;

use super::tests::{build, forty};
use super::*;
use crate::prefs::Preferences;

fn count(report: &[SpreadReport], design: &str) -> usize {
    report.iter().filter(|r| r.design == design).count()
}

fn with(project: &Project, prefs: Preferences) -> AutoBuild {
    let options = BuildOptions {
        preferences: prefs,
        ..BuildOptions::default()
    };
    auto_build(project, 10, &options).unwrap()
}

#[test]
fn tl_b9_auto_build_com_preferencias() {
    let project = forty();
    let base = build(&project, 10, Reuse::Medium);
    assert_eq!(
        with(&project, Preferences::default()),
        base,
        "sem preferências, igual"
    );

    // Rejeitar o desenho mais usado: ele some do álbum.
    let mut usage: HashMap<&str, usize> = HashMap::new();
    for r in &base.report {
        *usage.entry(r.design.as_str()).or_default() += 1;
    }
    let (most, _) = usage
        .iter()
        .max_by_key(|(d, n)| (**n, std::cmp::Reverse(**d)))
        .unwrap();
    let mut rejected = Preferences::default();
    rejected.reject(most);
    assert_eq!(count(&with(&project, rejected).report, most), 0);

    // Favoritar um desenho que não foi escolhido em lâminas parecidas: ele passa a aparecer mais.
    let dense: Vec<&SpreadReport> = base.report.iter().filter(|r| r.photos >= 4).collect();
    assert!(dense.len() >= 2, "o álbum tem lâminas densas");
    let mut tried = 0;
    for design in dense.iter().map(|r| r.design.clone()) {
        let mut favorite = Preferences::default();
        favorite.toggle_favorite(&design);
        let built = with(&project, favorite);
        if count(&built.report, &design) > count(&base.report, &design) {
            assert!(
                built
                    .report
                    .iter()
                    .any(|r| r.reason.contains("desenho favorito"))
            );
            return;
        }
        tried += 1;
    }
    panic!("nenhum dos {tried} desenhos favoritos apareceu mais vezes");
}
