//! Cenários da spec `modelos` (MD) no editor: a montagem automática com modelos (B8) e a velocidade com
//! muitos modelos (B12). Os outros estão em `md.rs`.

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{FrameContent, RelRect};
use lamina_layout::{BuildOptions, FamilyKind, ModelSlot, Preferences, auto_build};

use super::md::{HHV, abertura, album, symmetric, with_models};
use super::tl_common::{H, V, built, editor_at, spec};
use crate::{ModelsStore, Part};

#[test]
fn md_b8_o_auto_build_usa_o_modelo() {
    let (mut editor, _dir) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    let model = editor.models()[0].clone();
    let photos: Vec<(u32, u32)> = HHV.iter().copied().cycle().take(12).collect();
    let base = built(spec(30.0, 30.0), &photos, 4);
    let mut prefs = Preferences::default();
    prefs.favorites.insert("modelo:Abertura".into());
    let options = BuildOptions {
        models: vec![model],
        preferences: prefs,
        ..BuildOptions::default()
    };
    let out = auto_build(&base, 4, &options).unwrap();
    let uses: Vec<bool> = out
        .report
        .iter()
        .map(|r| r.family.kind() == FamilyKind::Model)
        .collect();
    assert!(uses.iter().any(|u| *u), "{uses:?}");
    assert!(!uses.windows(2).any(|w| w[0] && w[1]), "{uses:?}");
    let first = uses.iter().position(|u| *u).unwrap();
    let texts = out.project.spreads[first]
        .frames
        .iter()
        .filter(|f| matches!(f.content, FrameContent::Text(_)))
        .count();
    assert_eq!(texts, 1, "a lâmina no modelo ganha o texto dele");
}

#[test]
fn md_b8_pelo_novo_album() {
    let base = super::common::Base::with_photos(12, (300, 200));
    let (first, _) = crate::create_album(&base.new_album(Some(3)), &base.cache(), &|_| {}).unwrap();
    let model = crate::model_from_spread(&first.project, 0, "Abertura").unwrap();
    let mut prefs = Preferences::default();
    prefs.favorites.insert("modelo:Abertura".into());
    let new = crate::NewAlbum {
        models: vec![model],
        preferences: prefs,
        ..base.new_album(Some(3))
    };
    let (built, _) = crate::create_album(&new, &base.cache(), &|_| {}).unwrap();
    assert!(
        built
            .project
            .spreads
            .iter()
            .any(|s| s.model.as_deref() == Some("modelo:Abertura")),
        "{:?}",
        built
            .report
            .iter()
            .map(|r| r.family.kind())
            .collect::<Vec<_>>()
    );
}

#[test]
fn md_b12_muitos_modelos_continuam_rapidos() {
    let base = album(&[&[H, H, V, H]]);
    let model = |n: usize| {
        let mut m = symmetric();
        m.name = format!("Modelo {n}");
        m.slots.push(ModelSlot {
            rect: RelRect::new(0.4, 0.85, 0.2, 0.1).unwrap(),
            ..m.slots[0].clone()
        });
        m
    };
    let mut extra = Duration::MAX;
    for _ in 0..TRIES {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ModelsStore::load(&dir.path().join("modelos.json"));
        for n in 0..30 {
            store.save(model(n)).unwrap();
        }
        let plain = editor_at(base.clone(), 0);
        let started = Instant::now();
        let without = plain.panel(Part::Spread);
        let alone = started.elapsed();
        let mut editor = editor_at(base.clone(), 0);
        editor.set_models(store);
        let started = Instant::now();
        let with = editor.panel(Part::Spread);
        extra = extra.min(started.elapsed().saturating_sub(alone));
        assert!(with.options.len() > without.options.len());
        if extra < budget(50) {
            break;
        }
    }
    assert!(extra < budget(50), "os modelos custaram {extra:?} a mais");
}
