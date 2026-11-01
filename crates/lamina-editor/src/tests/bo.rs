//! Cenários da spec `barra-do-organizar` (BO): o que cabe na barra, o que acende com a seleção,
//! as dicas dos menus apagados e os comandos dos menus.

use crate::Command;
use crate::organize_bar::{BAR_MENUS_ORDER, BarFit, BarGroups, BarMenu, bar_fit, bar_groups};

#[test]
fn bo_b1_o_que_cabe_na_barra() {
    let all = BarFit {
        fit: true,
        menus: true,
        fields: true,
    };
    assert_eq!(bar_fit(1600.0), all);
    assert_eq!(
        bar_fit(1100.0),
        BarFit {
            fields: false,
            ..all
        }
    );
    assert_eq!(
        bar_fit(900.0),
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
fn bo_b2_quando_cada_grupo_acende() {
    let groups = |n| bar_groups(n);
    let off = BarGroups {
        fields: false,
        align: false,
        distribute: false,
    };
    assert_eq!(groups(0), off);
    assert_eq!(
        groups(1),
        BarGroups {
            fields: true,
            ..off
        }
    );
    assert_eq!(
        groups(2),
        BarGroups {
            fields: true,
            align: true,
            distribute: false
        }
    );
    assert_eq!(
        groups(3),
        BarGroups {
            fields: true,
            align: true,
            distribute: true
        }
    );
}

#[test]
fn bo_b3_as_dicas_dos_menus_apagados() {
    assert_eq!(
        BarMenu::Align.tip(1),
        "Alinhar (selecione 2 quadros ou mais)"
    );
    assert_eq!(
        BarMenu::Distribute.tip(1),
        "Distribuir (selecione 3 quadros ou mais)"
    );
    assert_eq!(BarMenu::Equal.tip(2), "Igualar");
}

#[test]
fn bo_b4_os_menus_rodam_os_mesmos_comandos() {
    let all: Vec<Command> = BAR_MENUS_ORDER
        .iter()
        .flat_map(|m| m.commands().iter().copied())
        .collect();
    assert_eq!(all.len(), 11);
    let mut names: Vec<&str> = all.iter().map(|c| c.name()).collect();
    assert!(names.iter().all(|n| n.starts_with("organizar.alinhar-")
        || n.starts_with("organizar.igualar-")
        || n.starts_with("organizar.distribuir-")));
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 11, "cada um uma vez");
}
