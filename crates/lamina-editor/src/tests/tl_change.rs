//! Sugestões, trocar pelo clique e pelas setas, uma página só (pela IA), reordenadas e as setas
//! de 2026-10-28 (TL-B1 a B5, B14 a B16).

use lamina_core::{FrameContent, Orientation};
use lamina_layout::{Family, LayoutOptions, layout_options};

use super::tl_common::{H, V, built, editor_at, frames, page, spec, three_spreads};
use crate::commands::{Menu, Mode};
use crate::suggestions::{Unavailable, selection};
use crate::{Command, Editor, Key, Part, Shortcut};

#[test]
fn tl_b1_sugestoes_da_lamina_atual() {
    let photos = [H, H, H, V];
    let mut project = lamina_core::Project::new("teste", spec(30.0, 30.0));
    for (i, (w, h)) in photos.iter().enumerate() {
        project.add_photo(format!("f{i:02}.jpg"), *w, *h);
    }
    let build_options = lamina_layout::BuildOptions {
        window: true,
        ..Default::default()
    };
    let built = lamina_layout::auto_build(&project, 1, &build_options).unwrap();
    let index = 0;
    let editor = editor_at(built.project.clone(), index);
    let panel = editor.panel(Part::Spread);
    let sel = selection(&built.project, index, Part::Spread).unwrap();
    assert_eq!(sel.photos.len(), 4);
    // O painel da janela usa as listas novas (spec RM, beta 9.1), com os modelos de fábrica da
    // mistura (3 H e 1 V).
    let window_options = LayoutOptions {
        window: true,
        ..LayoutOptions::default()
    };
    let mut expected_lists = layout_options(
        &built.project.spec,
        sel.active,
        &sel.aspects,
        &window_options,
    )
    .unwrap();
    let models = lamina_layout::factory::factory_models(3, 1, 0, true);
    if !models.is_empty() {
        let at = (sel.active, built.project.gap);
        lamina_layout::add_models(
            &mut expected_lists,
            &built.project.spec,
            at,
            &sel.aspects,
            &models,
        )
        .unwrap();
    }
    assert_eq!(panel.options, expected_lists.in_order);
    // A escolha da montagem automática está no painel (a posição pode diferir: o painel insere os
    // modelos de fábrica da mistura, o build a biblioteca inteira; spec RM, beta 9.1).
    assert!(
        panel.in_use.is_some(),
        "a escolha da montagem automática está no painel"
    );
}

#[test]
fn tl_b2_trocar_pelo_clique() {
    let project = three_spreads();
    let mut editor = editor_at(project.clone(), 1);
    let before = project.spreads[1].clone();
    assert!(editor.apply_option(Part::Spread, 4));
    let panel = editor.panel(Part::Spread);
    assert_eq!(panel.in_use, Some(4));
    let chosen = &panel.options[4];
    let sel = panel.selection.as_ref().unwrap();
    let now = frames(&editor);
    for (photo, rect) in sel.photos.iter().zip(&chosen.frames) {
        assert!(now.contains(&(*photo, *rect)), "foto no quadro da opção");
    }
    let s = editor.session().unwrap();
    for f in &s.project().spreads[1].frames {
        assert!(
            matches!(&f.content, FrameContent::Photo { crop, .. } if *crop == Default::default())
        );
    }
    assert!(s.is_dirty());
    s.project().validate().unwrap();
    // Desfazer em outra lâmina volta para a lâmina da edição.
    assert!(editor.run(Command::FirstSpread));
    assert!(editor.run(Command::Undo));
    let s = editor.session().unwrap();
    assert_eq!(s.current(), 1);
    assert_eq!(s.project().spreads[1], before);
    assert!(editor.run(Command::Redo));
    assert_eq!(editor.panel(Part::Spread).in_use, Some(4));
}

#[test]
fn tl_b3_setas_na_lamina_inteira() {
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.apply_option(Part::Spread, 0));
    let at = |e: &Editor| e.panel(Part::Spread).in_use;
    assert!(editor.run(Command::NextLayout));
    assert_eq!(at(&editor), Some(1));
    assert!(editor.run(Command::NextLayout));
    assert_eq!(at(&editor), Some(2));
    assert!(editor.run(Command::PreviousLayout));
    assert_eq!(at(&editor), Some(1));
    assert!(editor.apply_option(Part::Spread, 0));
    assert!(editor.run(Command::PreviousLayout));
    let last = editor.panel(Part::Spread).options.len() - 1;
    assert_eq!(at(&editor), Some(last), "da 1ª, o ↑ vai para a última");
}

#[test]
fn tl_b4_uma_pagina_so_pela_ia() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H, H, H, V], 1), 0);
    // Uma opção com 2 fotos em cada página e nada sobre a lombada.
    let panel = editor.panel(Part::Spread);
    let two_two = panel.options.iter().position(|c| match &c.family {
        Family::PerPage {
            left: Some(l),
            right: Some(r),
        } => l.groups.iter().sum::<usize>() == 2 && r.groups.iter().sum::<usize>() == 2,
        _ => false,
    });
    assert!(editor.apply_option(Part::Spread, two_two.unwrap()));
    // A opção seguinte à em uso de uma página, como a ferramenta `aplicar_layout` faz.
    let other = |e: &Editor, part| {
        let panel = e.panel(part);
        panel.in_use.map_or(0, |p| (p + 1) % panel.options.len())
    };

    let right = page(&editor, false);
    let left = page(&editor, true);
    assert!(editor.apply_option(Part::Left, other(&editor, Part::Left)));
    assert_eq!(
        page(&editor, false),
        right,
        "a esquerda não mexe na direita"
    );
    assert_ne!(page(&editor, true), left, "a esquerda muda");
    let left = page(&editor, true);
    assert!(editor.apply_option(Part::Right, other(&editor, Part::Right)));
    assert_eq!(page(&editor, true), left, "a direita não mexe na esquerda");
    assert_ne!(page(&editor, false), right);

    // Com uma foto sobre a lombada, só a lâmina inteira troca.
    let across = editor
        .panel(Part::Spread)
        .options
        .iter()
        .position(|c| {
            c.frames
                .iter()
                .any(|r| r.x() < 0.49 && r.x() + r.width() > 0.51)
        })
        .unwrap();
    assert!(editor.apply_option(Part::Spread, across));
    assert!(!editor.apply_option(Part::Left, 0) && !editor.apply_option(Part::Right, 0));
    let reason = editor.panel(Part::Left).unavailable.unwrap();
    assert_eq!(reason, Unavailable::CrossesSpine);
    assert_eq!(
        reason.to_string(),
        "Esta lâmina tem foto sobre a lombada: troque o layout da lâmina inteira."
    );
}

#[test]
fn tl_b5_opcao_reordenada() {
    let project = built(spec(30.0, 30.0), &[H, H, V, V], 1);
    let mut editor = editor_at(project, 0);
    assert!(editor.run(Command::ToggleReordered));
    let panel = editor.panel(Part::Spread);
    assert!(!panel.options.is_empty());
    let pick = panel
        .options
        .iter()
        .position(|c| c.order != vec![0, 1, 2, 3])
        .unwrap();
    let chosen = panel.options[pick].frames.clone();
    // As fotos da lâmina na ordem original (a das capturas), antes de aplicar.
    let before: Vec<_> = frames(&editor).into_iter().map(|(p, _)| p).collect();
    let mut original = before.clone();
    original.sort_by_key(|id| {
        let project = editor.session().unwrap().project();
        project.photos.iter().position(|p| p.id == *id)
    });
    assert!(editor.apply_option(Part::Spread, pick));
    // A opção traz um quadro por foto, na ordem original: a foto i vai para o quadro i
    // (auditoria do Marco 2, M7: antes só a orientação era conferida).
    let placed = frames(&editor);
    for (i, photo) in original.iter().enumerate() {
        let (_, rect) = placed.iter().find(|(p, _)| p == photo).unwrap();
        let want = chosen[i];
        assert!(
            (rect.x() - want.x()).abs() < 1e-6
                && (rect.y() - want.y()).abs() < 1e-6
                && (rect.width() - want.width()).abs() < 1e-6,
            "foto {i}: {rect:?} em vez de {want:?}"
        );
    }
    let s = editor.session().unwrap();
    let trim = s.project().spec.trim_rect();
    for (photo, rect) in frames(&editor) {
        let frame_aspect = rect.width() * trim.width.get() / (rect.height() * trim.height.get());
        let p = s.project().photo(photo).unwrap();
        let wanted = Orientation::from_aspect_ratio(p.aspect_ratio());
        assert_eq!(
            Orientation::from_aspect_ratio(frame_aspect),
            wanted,
            "quadro da orientação certa"
        );
    }
}

#[test]
fn tl_b14_setas_de_cima_e_de_baixo_trocam_o_layout() {
    let key = |k| Command::from_shortcut(Shortcut::key(k));
    assert_eq!(key(Key::Down), Some(Command::NextLayout));
    assert_eq!(key(Key::Up), Some(Command::PreviousLayout));
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.apply_option(Part::Spread, 0));
    let at = |e: &Editor| e.panel(Part::Spread).in_use;
    assert!(editor.run(key(Key::Down).unwrap()));
    assert_eq!(at(&editor), Some(1), "↓ vai para a opção seguinte");
    assert!(editor.run(key(Key::Up).unwrap()));
    assert_eq!(at(&editor), Some(0), "↑ volta");
    assert_eq!(editor.session().unwrap().current(), 1, "na mesma lâmina");
}

#[test]
fn tl_b15_setas_dos_lados_mudam_de_lamina() {
    let normal = |s| Command::from_shortcut_in(s, Mode::Normal);
    let organize = |s| Command::from_shortcut_in(s, Mode::Organize);
    let key = Shortcut::key;
    assert_eq!(normal(key(Key::Right)), Some(Command::NextSpread));
    assert_eq!(normal(key(Key::Left)), Some(Command::PreviousSpread));
    assert_eq!(normal(key(Key::PageDown)), Some(Command::NextSpread));
    assert_eq!(normal(key(Key::PageUp)), Some(Command::PreviousSpread));
    // No modo organizar as setas continuam movendo a seleção, e o Page Down muda de lâmina.
    assert_eq!(organize(key(Key::Right)), Some(Command::NudgeRight));
    assert_eq!(organize(key(Key::Left)), Some(Command::NudgeLeft));
    assert_eq!(organize(key(Key::PageDown)), Some(Command::NextSpread));
    // Ctrl+setas continuam movendo a lâmina de lugar.
    assert_eq!(
        normal(Shortcut::ctrl_key(Key::Right)),
        Some(Command::MoveSpreadRight)
    );

    let mut editor = editor_at(three_spreads(), 1);
    let s = editor.session().unwrap();
    let (before, dirty) = (s.project().clone(), s.is_dirty());
    assert!(editor.run(normal(key(Key::Right)).unwrap()));
    assert_eq!(
        editor.session().unwrap().current(),
        2,
        "→ vai para a próxima"
    );
    assert!(editor.run(normal(key(Key::Left)).unwrap()));
    assert!(editor.run(normal(key(Key::Left)).unwrap()));
    let s = editor.session().unwrap();
    assert_eq!(s.current(), 0, "← volta duas");
    assert_eq!(*s.project(), before, "nenhum layout mudou");
    assert_eq!(s.is_dirty(), dirty, "navegar não altera o álbum");
}

#[test]
fn tl_b16_sem_troca_de_uma_pagina_so_pelo_teclado() {
    for k in [Key::Down, Key::Up] {
        assert_eq!(
            Command::from_shortcut(Shortcut::shift(k)),
            None,
            "Shift+{k:?}"
        );
    }
    for name in [
        "layout.esquerda-proximo",
        "layout.esquerda-anterior",
        "layout.direita-proximo",
        "layout.direita-anterior",
    ] {
        assert_eq!(Command::from_name(name), None, "{name}");
    }
    let layout: Vec<&str> = Command::ALL
        .into_iter()
        .filter(|c| c.menu() == Menu::Layout)
        .map(Command::label)
        .collect();
    assert!(layout.iter().all(|l| !l.contains("página")), "{layout:?}");
}
