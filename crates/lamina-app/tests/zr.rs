//! Cenários da spec `zoom-e-tamanho-real` (ZR) que olham as telas: o balão de dica dentro da
//! janela e a linha de ajuda na paleta destacada (ZR-B1), e a saída animada com as fileiras de
//! baixo deslizando (ZR-B8).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

use lamina_editor::palette_tips::{PALETTE_BUTTONS, TIP_DELAY_MS};

fn ui(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn zr_b1_balao_na_janela_e_linha_de_ajuda_na_destacada() {
    // Desde a spec BO, os botões moram na barra de cima e a paleta destacada saiu: o balão vale
    // sempre, e os comandos de Alinhar, Igualar e Distribuir viraram menus (BO-B4). Desde a DG, o
    // balão é o do `IconButton`.
    let palette = ui("palette.slint");
    let buttons = ui("widgets/buttons.slint");
    assert!(buttons.contains("Tip {"), "o balão, dentro da janela");
    assert!(
        !palette.contains("detached"),
        "não há mais paleta destacada"
    );
    let bar = ui("organize.slint");
    // As dicas das ferramentas e do zoom vêm do editor, uma por botão, na ordem da paleta.
    let used: Vec<usize> = bar
        .split("tip: PaletteUi.tips[")
        .skip(1)
        .filter_map(|p| p.split(']').next()?.trim().parse().ok())
        .collect();
    let mut sorted = used.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, (0..9).collect::<Vec<_>>());
    assert_eq!(
        PALETTE_BUTTONS.len(),
        20,
        "os outros 11 são as linhas dos menus"
    );
    // O atraso do balão é o da tabela do editor.
    let widgets = ui("widgets/buttons.slint");
    assert!(widgets.contains(&format!("interval: {TIP_DELAY_MS}ms;")));
}

#[test]
fn zr_b8_a_saida_anima_e_as_fileiras_voltam() {
    let stage = ui("stage.slint");
    assert!(stage.contains("width: root.canvas-width * root.view-zoom;"));
    assert!(stage.contains(
        "animate view-zoom, view-pan-x, view-pan-y { duration: OrganizeUi.view-ms * 1ms;"
    ));
    let bottom = ui("bottom.slint");
    assert!(bottom.contains("height: OrganizeUi.on ? 0px : rest.preferred-height;"));
    assert!(
        bottom.contains("animate height { duration: Motion.on ? Motion.navigate-ms * 1ms : 0ms;")
    );
    assert!(
        !bottom.contains("!OrganizeUi.on:"),
        "nenhuma fileira entra e sai por if no modo"
    );
    let organize = ui("organize.slint");
    assert!(organize.contains("text: \"Voltar\";") && !organize.contains("\"Concluir\""));
}
