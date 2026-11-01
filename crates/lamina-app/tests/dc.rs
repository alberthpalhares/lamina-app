//! Cenários da spec `design-menu-de-contexto` (DC) que olham as telas: um menu só, os tokens e o
//! Esc que não vaza. O que é decisão vai nos testes do `lamina-editor` (`tests/dc.rs`).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

fn ui(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn dc_b5_um_menu_so() {
    let strip = ui("photo_strip.slint");
    assert!(!strip.contains("ContextMenuArea"));
    assert!(!strip.contains("MenuItem"));
    assert!(strip.contains("PhotoMenuUi.menu-at("));
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
        .unwrap();
    assert!(
        src.contains(".on_menu_at("),
        "a grade pede o mesmo ContextUi"
    );
    assert!(ui("app.slint").contains("if ContextUi.open: ContextMenu {"));
}

#[test]
fn dc_b7_so_tokens() {
    let menu = ui("context.slint");
    assert!(menu.contains("background: Theme.raised;"));
    assert!(!menu.contains("Theme.panel"));
    assert!(menu.contains("drop-shadow-blur: Elevation.menu-blur;"));
    assert!(menu.contains("drop-shadow-offset-y: Elevation.menu-y;"));
    assert!(menu.contains("drop-shadow-color: Elevation.menu-color;"));
    assert!(menu.contains("Theme.text-disabled"));
    assert!(menu.contains("border-radius: Radius.medium;"));
    // Fora da escala, só os 28 px das linhas, e os limites da largura (regra 3).
    assert!(menu.contains("root.data.kind == 2 ? Space.s2 : 28px"));
    assert!(menu.contains("max(root.least-width, min(340px, list.preferred-width))"));
    for line in menu.lines().map(str::trim) {
        let spacing = line.starts_with("spacing:") || line.starts_with("padding");
        assert!(!(spacing && line.ends_with("px;")), "{line}");
    }
}

#[test]
fn dc_b10_esc_nao_vaza() {
    let menu = ui("context.slint");
    // O foco fica num popup, e as teclas param nele: nenhuma chega à janela.
    assert!(menu.contains("close-policy: no-auto-close;"));
    assert!(menu.contains("focus-on-tab-navigation: false;"));
    assert!(menu.contains("ContextUi.key(\"esc\");"));
    // E nada tira o foco dele enquanto está aberto, nem a seleção que o clique direito muda.
    let app = ui("app.slint");
    let guard = &app[app.find("public function focus-main()").unwrap()..];
    assert!(guard[..guard.find("keys.focus();").unwrap()].contains("if (!ContextUi.open)"));
    let handler = &menu[menu.find("key-pressed(e) => {").unwrap()..];
    let body = &handler[..handler.find("main := MenuPanel").unwrap()];
    assert!(body.trim_end().ends_with('}'));
    assert!(body.contains("accept"));
    assert!(!body.contains("reject"));
}
