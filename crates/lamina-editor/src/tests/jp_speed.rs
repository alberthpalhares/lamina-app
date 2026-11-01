//! Abrir e navegar rápido (JP-B13; AGENTS.md, regra 13).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_layout::Reuse;

use super::common::Base;
use crate::{Cache, NewAlbum, ScreenPlan, Session, SpecFields, create_album, load_screen};

/// Lê as fotos pedidas, em paralelo, como o trabalhador da janela; devolve quantas chegaram.
fn read(session: &Session, plan: &mut ScreenPlan, index: usize, cache: &Cache) -> usize {
    use rayon::prelude::*;
    let asked = plan.go(session.project(), index);
    let project = session.project();
    let read: Vec<_> = asked
        .photos
        .par_iter()
        .filter_map(|id| Some((*id, load_screen(project.photo(*id).ok()?, cache)?)))
        .collect();
    for (id, _) in &read {
        plan.arrived(*id, false);
    }
    read.len()
}

#[test]
fn jp_b13_abrir_e_navegar_rapido() {
    let base = Base::with_photos(40, (3000, 2000));
    let cache = base.cache();
    let new = NewAlbum {
        folder: base.path("fotos"),
        name: Some("casamento".into()),
        spec: SpecFields::default().to_spec().unwrap(),
        spreads: Some(10),
        reuse: Reuse::Medium,
        preferences: lamina_layout::Preferences::default(),
        learned: lamina_layout::Learned::default(),
        models: Vec::new(),
        limits: lamina_core::PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
        style: Default::default(),
        files: Vec::new(),
        build: true,
    };
    // Monta uma vez: deixa miniaturas e imagens de tela no cache.
    let (built, _) = create_album(&new, &cache, &|_| {}).unwrap();
    let file = base.save(built.project, "album/casamento.lamina");

    let mut open = Duration::MAX;
    let mut step = Duration::MAX;
    for _ in 0..TRIES {
        let started = Instant::now();
        let mut session = Session::open(&file).unwrap();
        let mut plan = ScreenPlan::default();
        let loaded = read(&session, &mut plan, 0, &cache);
        open = open.min(started.elapsed());
        assert!(loaded > 0);

        // A pior troca da rodada: a spec pede cada troca abaixo da meta (auditoria do Marco 2,
        // M7: antes valia a troca mais rápida).
        let mut worst = Duration::ZERO;
        for next in 1..session.spread_count() {
            let started = Instant::now();
            assert!(session.go_to(next));
            read(&session, &mut plan, next, &cache);
            worst = worst.max(started.elapsed());
        }
        step = step.min(worst);
        if open < budget(500) && step < budget(100) {
            break;
        }
    }
    assert!(open < budget(500), "abrir levou {open:?}");
    assert!(step < budget(100), "trocar de lâmina levou {step:?}");
}
