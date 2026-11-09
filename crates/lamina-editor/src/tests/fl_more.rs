//! Cenários da spec `faixa-de-layouts` (FL-B8, B10 a B13; o FL-B9 lê as telas, no
//! `lamina-app`) e os casos-limite da seção 6.

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{FrameContent, PhotoId, Project};
use lamina_layout::{LayoutCandidate, design_key};

use super::common::Base;
use super::fl::{album, clean_at, open, place, shown, steps};
use super::tl_common::{H, V, built, editor_at, spec};
use crate::motion::{Slots, panel_key};
use crate::screens::ScreenPlan;
use crate::text::ALL_REJECTED;
use crate::{Command, Editor, ExportOptions, Part, export_project};

fn photos_of(project: &Project, index: usize) -> Vec<PhotoId> {
    crate::edits::spread_photos(project, index)
}

#[test]
fn fl_b8_favoritar_e_rejeitar_pela_faixa() {
    let mut editor = clean_at(album(3), 1, 1);
    open(&mut editor);
    let panel = editor.panel(Part::Spread);
    let aspects = panel.selection.clone().unwrap().aspects;
    let key = |c: &LayoutCandidate| design_key(c, &aspects);
    let (fourth, sixth) = (key(&panel.options[3]), key(&panel.options[5]));
    assert!(editor.hover_option(Some(3)));
    assert!(editor.mark_option(3, true));
    let panel = editor.panel(Part::Spread);
    assert_eq!(key(&panel.options[0]), fourth, "a 4 vai para o topo");
    assert!(panel.cards()[0].favorite, "com a estrela");
    assert_eq!(editor.band().hover, None, "a prévia é limpa");
    let at = panel.options.iter().position(|c| key(c) == sixth).unwrap();
    assert!(editor.mark_option(at, false));
    let panel = editor.panel(Part::Spread);
    assert!(
        panel.options.iter().all(|c| key(c) != sixth),
        "a 6 sai da lista"
    );
    assert!(
        steps(&editor).is_empty(),
        "as marcas não entram no histórico"
    );
    let used = panel.in_use.unwrap();
    let used_key = key(&panel.options[used]);
    assert!(editor.mark_option(used, false));
    assert_eq!(steps(&editor).len(), 1);
    assert!(
        steps(&editor)[0].starts_with("Trocar modelo"),
        "aplica a seguinte"
    );
    let prefs = editor.preferences();
    assert!(prefs.favorites.contains(&fourth));
    assert!(prefs.rejected.contains(&sixth) && prefs.rejected.contains(&used_key));
    assert!(editor.band().open, "a faixa continua aberta");
}

/// A lâmina 2 com 2 fotos e um texto, e 4 fotos de outras lâminas para selecionar.
fn two_photos_and_a_text() -> (Editor, Vec<PhotoId>) {
    let mut first = editor_at(album(3), 1);
    let two = photos_of(first.session().unwrap().project(), 1)[..2].to_vec();
    first.set_selection(two);
    assert!(first.run(Command::BuildFromSelection));
    assert!(first.new_text(None));
    let project = first.session().unwrap().project().clone();
    let chosen = [photos_of(&project, 0), photos_of(&project, 2)].concat()[..4].to_vec();
    (editor_at(project, 1), chosen)
}

#[test]
fn fl_b10_com_fotos_selecionadas_no_navegador() {
    let (mut editor, chosen) = two_photos_and_a_text();
    editor.set_selection(chosen.clone());
    open(&mut editor);
    let panel = editor.panel(Part::Spread);
    assert_eq!(
        panel.selection.as_ref().unwrap().photos,
        chosen,
        "as opções são para as 4"
    );
    assert_eq!(panel.in_use, None);
    assert!(editor.hover_option(Some(2)));
    let view = editor.view_project().unwrap().spreads[1].clone();
    let photos: Vec<PhotoId> = view
        .frames
        .iter()
        .filter_map(|f| match f.content {
            FrameContent::Photo { photo, .. } => Some(photo),
            _ => None,
        })
        .collect();
    assert_eq!(photos, chosen, "só as 4 nos quadros da opção 3");
    assert_eq!(view.frames.len(), 4, "as 2 fotos e o texto saem");
    let rects: Vec<_> = view.frames.iter().map(|f| f.rect).collect();
    assert_eq!(rects, panel.options[2].frames);
    assert!(editor.apply_option(Part::Spread, 2));
    assert!(steps(&editor)[0].starts_with("Montar lâmina"));
    assert!(editor.selection().is_empty(), "a seleção fica vazia");
    assert_eq!(
        editor.session().unwrap().project().spreads[1],
        view,
        "igual à prévia"
    );
}

#[test]
fn fl_b11_a_previa_nunca_chega_ao_arquivo() {
    let base = Base::with_photos(8, (600, 400));
    let file = base.path("album.lamina");
    let mut editor = Editor::new();
    editor.adopt_saved(base.project(2), &file).unwrap();
    let saved = std::fs::read(&file).unwrap();
    open(&mut editor);
    assert!(editor.hover_option(Some(4)));
    let written = editor.save_to(None).unwrap();
    assert_eq!(written.file_name(), file.file_name());
    assert!(
        !editor.band().open,
        "o Ctrl+S fecha a faixa antes de salvar"
    );
    assert_eq!(std::fs::read(&file).unwrap(), saved, "igual byte a byte");

    let index = editor.session().unwrap().current();
    let real = editor.session().unwrap().project().clone();
    open(&mut editor);
    assert!(editor.hover_option(Some(4)));
    let view = editor.view_project().unwrap().into_owned();
    assert_ne!(
        view.spreads[index], real.spreads[index],
        "a prévia está na tela"
    );
    assert!(
        editor.close_band_for(Command::Export),
        "exportar fecha a faixa"
    );
    let options = ExportOptions::default();
    let (with, without) = (base.path("com"), base.path("sem"));
    export_project(
        editor.session().unwrap().project(),
        &with,
        &options,
        &|_, _| {},
    )
    .unwrap();
    export_project(&real, &without, &options, &|_, _| {}).unwrap();
    let mut names: Vec<_> = std::fs::read_dir(&without).unwrap().flatten().collect();
    names.sort_by_key(std::fs::DirEntry::file_name);
    assert!(!names.is_empty());
    for entry in names {
        let other = with.join(entry.file_name());
        assert_eq!(
            std::fs::read(other).unwrap(),
            std::fs::read(entry.path()).unwrap()
        );
    }
}

/// 60 lâminas, 600 fotos e a lâmina 31 com 12 fotos.
fn big_album() -> Editor {
    let photos: Vec<(u32, u32)> = (0..480).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 60);
    let extra: Vec<PhotoId> = (0..120)
        .map(|i| project.add_photo(format!("x{i:03}.jpg"), 3000, 2000 + (i % 3) * 500))
        .collect();
    let mut first = editor_at(project, 30);
    first.set_selection(extra[..12].to_vec());
    assert!(first.run(Command::BuildFromSelection));
    editor_at(first.session().unwrap().project().clone(), 30)
}

#[test]
fn fl_b12_parar_o_mouse_e_aplicar_rapido() {
    let mut editor = big_album();
    open(&mut editor);
    let key = panel_key(&editor.panel(Part::Spread), editor.session().unwrap());
    let (mut preview, mut screen) = (Duration::MAX, Duration::MAX);
    let mut plan = ScreenPlan::default();
    for _ in 0..TRIES {
        let mut slots = Slots::default();
        slots.update(&shown(&editor), place(&editor), false);
        let (mut worst, mut worst_screen) = (Duration::ZERO, Duration::ZERO);
        for i in 0..10 {
            let started = Instant::now();
            assert!(editor.hover_option(Some(i)));
            slots.update(&shown(&editor), place(&editor), false);
            worst = worst.max(started.elapsed());
            let cards = editor.panel(Part::Spread).cards();
            plan.go(&editor.view_project().unwrap(), 30);
            assert!(cards.len() >= 10);
            worst_screen = worst_screen.max(started.elapsed());
        }
        assert!(editor.hover_option(None));
        (preview, screen) = (preview.min(worst), screen.min(worst_screen));
        if preview < budget(10) && screen < budget(50) {
            break;
        }
    }
    assert!(preview < budget(10), "a prévia levou {preview:?}");
    assert!(screen < budget(50), "até a tela pronta levou {screen:?}");
    let session = editor.session().unwrap();
    assert_eq!(
        panel_key(&editor.panel(Part::Spread), session),
        key,
        "a prévia não muda a chave"
    );
    assert!(editor.hover_option(Some(3)) && editor.apply_option(Part::Spread, 3));
    let session = editor.session().unwrap();
    assert_eq!(
        panel_key(&editor.panel(Part::Spread), session),
        key,
        "aplicar também não"
    );
}

#[test]
fn fl_b13_movimento_da_faixa() {
    let mut editor = editor_at(album(3), 1);
    let d = editor.durations();
    assert_eq!((d.band, d.slide), (200, 180));
    assert!(editor.run(Command::ToggleReduceMotion));
    let d = editor.durations();
    assert_eq!(
        (d.band, d.slide, d.fade),
        (0, 0, 0),
        "a lâmina muda de uma vez"
    );
    open(&mut editor);
    assert!(
        editor.hover_option(Some(1)),
        "a prévia continua, sem movimento"
    );
}

#[test]
fn faixa_sem_cartoes_mostra_so_o_aviso() {
    let mut project = Project::new("vazia", spec(30.0, 30.0));
    project.add_photo("a.jpg", 3000, 2000);
    project.add_spread();
    let mut editor = editor_at(project, 0);
    open(&mut editor);
    let panel = editor.panel(Part::Spread);
    assert!(panel.cards().is_empty());
    assert_eq!(panel.notice(), "Esta lâmina não tem fotos.");
    assert!(!editor.hover_option(Some(0)), "sem cartão, sem prévia");
    let mut editor = editor_at(album(8), 2);
    let all: Vec<PhotoId> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.id)
        .collect();
    editor.set_selection(all[..25].to_vec());
    let panel = editor.panel(Part::Spread);
    assert!(panel.cards().is_empty());
    assert_eq!(panel.notice(), "Selecione até 24 fotos para uma lâmina.");
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H, V], 1), 0);
    while editor.panel(Part::Spread).notice().is_empty() {
        assert!(editor.mark_option(0, false));
    }
    assert_eq!(editor.panel(Part::Spread).notice(), ALL_REJECTED);
}

#[test]
fn a_previa_some_quando_a_lista_muda_e_nao_vale_com_a_faixa_fechada() {
    let mut editor = clean_at(album(3), 1, 0);
    assert!(!editor.hover_option(Some(2)), "faixa fechada: nada");
    open(&mut editor);
    assert!(editor.hover_option(Some(2)));
    let other = photos_of(editor.session().unwrap().project(), 0)[..2].to_vec();
    editor.set_selection(other);
    assert_eq!(editor.band().hover, None, "outra lista de cartões");
    let i = editor.session().unwrap().current();
    let view = editor.view_project().unwrap().spreads[i].clone();
    assert_eq!(view, editor.session().unwrap().project().spreads[i]);
}

#[test]
fn teclas_do_menu_layout_deixam_a_faixa_aberta_e_o_resto_fecha() {
    let mut editor = clean_at(album(3), 1, 0);
    for command in [Command::Mirror, Command::Favorite, Command::ToggleReordered] {
        open(&mut editor);
        assert!(editor.hover_option(Some(1)));
        assert!(!editor.close_band_for(command), "{command:?}");
        assert_eq!(editor.band().hover, None);
        assert!(editor.close_band());
    }
    open(&mut editor);
    assert!(
        !editor.close_band_for(Command::Deselect),
        "o Esc é do próprio comando"
    );
    assert!(!editor.close_band_for(Command::ToggleLayoutBand));
    assert!(editor.close_band_for(Command::Save));
    assert!(!editor.band().open);
}

#[test]
fn as_miniaturas_da_faixa_tem_os_quadros_do_clique() {
    for chosen in [false, true] {
        let mut editor = clean_at(album(3), 1, 0);
        if chosen {
            let project = editor.session().unwrap().project();
            let other = [photos_of(project, 0), photos_of(project, 2)].concat()[..3].to_vec();
            editor.set_selection(other);
        }
        let panel = editor.panel(Part::Spread);
        let mut seen = Vec::new();
        editor.for_each_preview(&panel, |i, project| {
            let exact = editor.preview(&panel, i).unwrap();
            let shape = |p: &Project| -> Vec<_> {
                p.spreads[1]
                    .frames
                    .iter()
                    .map(|f| (f.rect, f.content.clone()))
                    .collect()
            };
            assert_eq!(shape(project), shape(&exact), "opção {i}");
            assert_eq!(project.spreads[0], exact.spreads[0]);
            seen.push(i);
        });
        assert_eq!(
            seen,
            (0..panel.options.len()).collect::<Vec<_>>(),
            "um cartão por opção"
        );
    }
}
