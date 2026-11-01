//! Cenário da spec `tirar-fotos-do-album` (TF) que olha a tela: o botão direito numa foto da grade
//! abre o menu (TF-B10).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

fn ui(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn tf_b10_botao_direito_na_grade_abre_o_menu() {
    let strip = ui("photo_strip.slint");
    // O menu é o mesmo da lâmina (spec DC, regra 1): a peça pede ao soltar o botão, também numa
    // foto já selecionada. Ao apertar, uma foto fora da seleção passa a ser a única.
    let down = strip
        .find("e.button == PointerEventButton.right && e.kind == PointerEventKind.down")
        .unwrap();
    let up = strip
        .find("e.button == PointerEventButton.right && e.kind == PointerEventKind.up")
        .unwrap();
    assert!(strip[down..up].contains("if (!root.lit) {"));
    assert!(!strip[down..up].contains("menu-at"));
    assert!(strip[up..].contains("PhotoMenuUi.menu-at("));
}
