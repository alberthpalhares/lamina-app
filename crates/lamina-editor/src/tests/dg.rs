//! Cenários da spec `design-organizar` (DG): o que cabe na barra, a dica, o título, os atalhos, a
//! outra cor e a unidade das réguas. O que só dá para conferir nos `.slint` está em
//! `lamina-app/tests/dg.rs`.

use lamina_core::CoverMeasures;

use super::tl_common::{editor_at, three_spreads};
use crate::new_project::{Unit, picker_color};
use crate::organize_bar::{BarFit, bar_fit};
use crate::shortcuts::sheet;
use crate::{Command, Platform};

const CHOOSE: &str =
    "Clique num quadro · Ctrl+clique soma · arraste no vazio para selecionar vários · Esc volta";

fn organizing() -> crate::Editor {
    let mut editor = editor_at(three_spreads(), 2);
    assert!(editor.run(Command::Organize));
    editor
}

#[test]
fn dg_b1_o_que_cabe_na_barra() {
    let all = BarFit {
        fit: true,
        menus: true,
        fields: true,
    };
    assert_eq!(bar_fit(1400.0), all);
    assert_eq!(
        bar_fit(1026.0),
        BarFit {
            fields: false,
            ..all
        },
        "no notebook com o painel, os menus ficam"
    );
    assert_eq!(
        bar_fit(800.0),
        BarFit {
            fit: true,
            menus: false,
            fields: false
        }
    );
    assert_eq!(
        bar_fit(640.0),
        BarFit {
            fit: false,
            menus: false,
            fields: false
        }
    );
}

#[test]
fn dg_b5_a_dica_fala_do_modo() {
    let outside = editor_at(three_spreads(), 2).hint();
    assert!(outside.contains("trocam o layout"));

    let mut editor = organizing();
    assert_eq!(editor.hint(), CHOOSE);
    editor.organize_click(0, false, false);
    assert!(editor.hint().starts_with("Delete tira a foto"));
    editor.click_nothing();
    assert_eq!(editor.hint(), CHOOSE);
    // Dois quadros também são seleção: a dica de como selecionar sai.
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    assert_ne!(editor.hint(), CHOOSE);
}

#[test]
fn dg_b6_o_titulo_do_painel() {
    let mut editor = organizing();
    assert_eq!(editor.organize_title(), "Organizar a lâmina 3");
    let measures = CoverMeasures::defaults(&editor.session().unwrap().album().spec);
    assert!(editor.create_cover(measures, None, &[]));
    assert!(editor.on_cover());
    assert_eq!(editor.organize_title(), "Organizar a capa");
    let mut outside = editor_at(three_spreads(), 2);
    assert_eq!(outside.organize_title(), "");
    assert!(outside.run(Command::Organize));
    assert!(outside.run(Command::Organize));
    assert_eq!(outside.organize_title(), "");
}

#[test]
fn dg_b7_laco_fora_da_lista_de_atalhos() {
    let groups = sheet(Platform::Windows);
    let rows: Vec<&str> = groups
        .iter()
        .flat_map(|g| &g.rows)
        .map(|r| r.does)
        .collect();
    assert!(rows.contains(&"Selecionar vários arrastando no vazio"));
    assert!(!rows.iter().any(|r| r.to_lowercase().contains("laço")));
}

#[test]
fn dg_b8_outra_cor_muda_o_fundo_e_desfaz() {
    let mut editor = editor_at(three_spreads(), 0);
    let before = editor.look().unwrap().left;
    let chosen = crate::parse_color("#F5F0E6").unwrap();
    editor.set_background(chosen, None);
    assert_eq!(editor.look().unwrap().left, chosen);
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.look().unwrap().left, before);
    // O quadro de cores chega ao mesmo lugar.
    let clicked = picker_color(0.5, 0.0);
    editor.set_background(clicked, None);
    assert_eq!(editor.look().unwrap().left, clicked);
}

#[test]
fn dg_b9_a_unidade_das_reguas() {
    assert_eq!(Unit::Cm.label(), "cm");
    assert_eq!(Unit::Inch.label(), "pol");
    assert_eq!(Unit::Mm.label(), "mm");
}
