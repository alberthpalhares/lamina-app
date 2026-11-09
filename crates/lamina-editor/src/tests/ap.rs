//! Cenários da spec `aparencia` que rodam no editor: AP-B1, B2, B7, B8, B10 e B11 (os de arquivo
//! estão no `lamina-export`).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{Crop, FrameContent, Mm, PhotoId, Project, Rgb};
use lamina_layout::design_key;

use super::common::isolated;
use super::tl_common::{H, V, built, editor_at, spec, three_spreads};
use crate::edits::{layout_with, spread_photos};
use crate::suggestions::selection;
use crate::{BAD_COLOR, Command, Editor, Part, parse_color};

/// O desenho em uso na lâmina atual.
fn in_use_key(editor: &Editor) -> String {
    let panel = editor.panel(Part::Spread);
    let sel = panel.selection.unwrap();
    design_key(&panel.options[panel.in_use.unwrap()], &sel.aspects)
}

fn centers(editor: &Editor) -> Vec<(PhotoId, (f64, f64))> {
    let s = editor.session().unwrap();
    let mut out: Vec<_> = s.project().spreads[s.current()]
        .frames
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, crop } => Some((*photo, (crop.center_x, crop.center_y))),
            _ => None,
        })
        .collect();
    out.sort_by_key(|(p, _)| *p);
    out
}

#[test]
fn ap_b1_mudar_o_espaco_mantem_o_desenho() {
    let mut project = three_spreads();
    if let FrameContent::Photo { crop, .. } = &mut project.spreads[1].frames[0].content {
        *crop = Crop {
            zoom: 1.0,
            center_x: 0.4,
            center_y: 0.5,
        };
    }
    let before = project.spreads[1].clone();
    let mut editor = editor_at(project, 1);
    assert!(editor.apply_option(Part::Spread, 3));
    let key = in_use_key(&editor);
    let photos = centers(&editor);
    assert_eq!(editor.set_gap(Mm(10.0)), None);
    assert_eq!(in_use_key(&editor), key, "mesmo desenho");
    assert_eq!(centers(&editor), photos, "mesmas fotos e centros");
    let s = editor.session().unwrap();
    assert_eq!(s.project().spreads[1].gap, Some(Mm(10.0)));
    assert_eq!(
        selection(s.project(), 1, Part::Spread).unwrap().gap,
        Mm(10.0)
    );
    assert!(editor.run(Command::Undo));
    let s = editor.session().unwrap();
    assert_eq!(s.project().spreads[1].gap, None);
    assert_ne!(
        s.project().spreads[1],
        before,
        "o Ctrl+Z volta à troca de layout, antes do espaço"
    );
}

#[test]
fn ap_b2_espaco_para_o_album_inteiro() {
    let photos: Vec<(u32, u32)> = (0..40).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut editor = editor_at(built(spec(30.0, 30.0), &photos, 10), 0);
    let keys: Vec<String> = (0..10)
        .map(|i| {
            editor.session_mut().unwrap().go_to(i);
            in_use_key(&editor)
        })
        .collect();
    editor.session_mut().unwrap().go_to(0);
    assert_eq!(editor.set_gap(Mm(5.0)), None);
    assert!(editor.style_all());
    let s = editor.session().unwrap();
    assert_eq!(s.project().gap, Mm(5.0));
    assert!(s.project().spreads.iter().all(|sp| sp.gap.is_none()));
    // Com os modelos de fábrica nas listas (spec RM, beta 9.1), o desenho em uso pode casar com
    // um deles: a chave diz "fabrica:…" sem o desenho mudar. Conferimos os quadros, não a chave.
    let frames_keys: Vec<Vec<String>> = (0..10)
        .map(|i| {
            editor.session_mut().unwrap().go_to(i);
            editor.session().unwrap().project().spreads[i]
                .frames
                .iter()
                .map(|f| format!("{:?}", f.rect))
                .collect()
        })
        .collect();
    editor.session_mut().unwrap().go_to(0);
    assert_eq!(editor.set_gap(Mm(5.0)), None);
    assert!(editor.style_all());
    let s = editor.session().unwrap();
    assert_eq!(s.project().gap, Mm(5.0));
    assert!(s.project().spreads.iter().all(|sp| sp.gap.is_none()));
    for (i, key) in keys.iter().enumerate() {
        editor.session_mut().unwrap().go_to(i);
        // O desenho continua casando (pelo motor ou por um modelo de fábrica igual).
        assert!(
            &in_use_key(&editor) == key
                || editor.session().unwrap().project().spreads[i]
                    .frames
                    .iter()
                    .map(|f| format!("{:?}", f.rect))
                    .collect::<Vec<_>>()
                    == frames_keys[i],
            "lâmina {} com o mesmo desenho",
            i + 1
        );
        let gap = selection(editor.session().unwrap().project(), i, Part::Spread)
            .unwrap()
            .gap;
        assert_eq!(gap, Mm(5.0), "as próximas trocas usam 5 mm");
    }
}

fn borders(editor: &Editor) -> Vec<f64> {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .map(|f| f.border.width.get())
        .collect()
}

#[test]
fn ap_b7_so_o_quadro_selecionado() {
    let mut editor = editor_at(three_spreads(), 1);
    editor.select_frame(Some(1));
    assert!(editor.set_border(Mm(1.0), Rgb::WHITE));
    let b = borders(&editor);
    assert_eq!(b.iter().filter(|w| **w > 0.0).count(), 1);
    assert!(b[1] > 0.0);
    editor.select_frame(None);
    assert!(editor.set_border(Mm(1.0), Rgb::WHITE));
    assert!(borders(&editor).iter().all(|w| *w > 0.0));
}

#[test]
fn ap_b8_aplicar_a_todas() {
    let cream = Rgb {
        r: 245,
        g: 240,
        b: 230,
    };
    let mut editor = editor_at(three_spreads(), 1);
    assert!(editor.set_background(cream, None));
    assert!(editor.set_border(Mm(1.0), Rgb::WHITE));
    assert!(editor.set_radius(Mm(3.0)));
    // Espaço e fundo da página direita também vão (auditoria do Marco 2, M7).
    assert_eq!(editor.set_gap(Mm(6.0)), None);
    assert!(editor.set_background(Rgb::WHITE, Some(lamina_core::PageSide::Right)));
    let current = editor.session().unwrap().project().spreads[1].clone();
    assert_eq!(current.gap, Some(Mm(6.0)));
    let before: Vec<_> = editor.session().unwrap().project().spreads.clone();
    assert!(editor.style_all());
    let p = editor.session().unwrap().project();
    for s in &p.spreads {
        assert_eq!(s.background, cream);
        assert_eq!(s.background_right, current.background_right);
        // O espaço passa a ser o do álbum, igual ao da lâmina de origem.
        assert_eq!(p.spread_gap(s), Mm(6.0));
        assert!(
            s.frames
                .iter()
                .all(|f| f.border.width == Mm(1.0) && f.radius == Mm(3.0))
        );
    }
    assert!(editor.run(Command::Undo));
    assert_eq!(
        editor.session().unwrap().project().spreads,
        before,
        "um Ctrl+Z volta todas"
    );
}

#[test]
fn ap_b10_cor_invalida_e_espaco_que_nao_cabe() {
    assert_eq!(parse_color("#12"), None);
    assert_eq!(
        parse_color("#1e1E1e"),
        Some(Rgb {
            r: 30,
            g: 30,
            b: 30
        })
    );
    assert_eq!(BAD_COLOR, "Use uma cor como #F5F0E6.");
    // 12 fotos numa página de 5 × 5 cm com 10 mm de zona segura: 20 mm não cabe.
    let mut project = Project::new("teste", spec(5.0, 5.0));
    let ids: Vec<PhotoId> = (0..12)
        .map(|i| project.add_photo(format!("f{i}.jpg"), 3000, 2000))
        .collect();
    project.add_spread();
    layout_with(&mut project, 0, &ids).unwrap();
    let mut editor = editor_at(project, 0);
    let notice = editor.set_gap(Mm(20.0)).unwrap();
    assert!(
        notice.starts_with("O espaço foi reduzido para "),
        "{notice}"
    );
    assert!(notice.ends_with(" mm: não cabia mais."), "{notice}");
    let s = editor.session().unwrap();
    assert!(s.project().spreads[0].gap.unwrap() < Mm(20.0));
    assert_eq!(
        spread_photos(s.project(), 0).len(),
        12,
        "as fotos continuam"
    );
}

#[test]
fn ap_b11_rapido() {
    let photos: Vec<(u32, u32)> = (0..40).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let project = built(spec(30.0, 30.0), &photos, 10);
    let (mut one, mut all) = (Duration::MAX, Duration::MAX);
    for _ in 0..TRIES {
        let mut editor = editor_at(project.clone(), 1);
        let (a, b) = isolated(|| {
            let started = Instant::now();
            assert_eq!(editor.set_gap(Mm(6.0)), None);
            let a = started.elapsed();
            let started = Instant::now();
            assert!(editor.style_all());
            (a, started.elapsed())
        });
        one = one.min(a);
        all = all.min(b);
        if one < budget(100) && all < budget(1000) {
            break;
        }
    }
    assert!(one < budget(100), "lâmina em {one:?}");
    assert!(all < budget(1000), "álbum em {all:?}");
}
