//! As telas da spec `design-organizar` (DG): o que só dá para conferir lendo os `.slint`. O que a
//! tela mostra sai do `lamina-editor` e é testado lá (`tests/dg.rs`).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ui(file: &str) -> String {
    fs::read_to_string(manifest().join("ui").join(file)).unwrap()
}

/// Os valores de `padding` e `spacing` escritos em pixels, um por linha que os tem.
fn loose_spaces(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            (l.starts_with("spacing:") || l.starts_with("padding")) && l.contains("px;")
        })
        .collect()
}

/// O trecho do componente `name` até o próximo `export component`.
fn bar(text: &str) -> &str {
    let from = text.find("export component OrganizeBar").unwrap();
    let rest = &text[from..];
    let to = rest[1..].find("export component").unwrap();
    &rest[..to]
}

#[test]
fn dg_b2_nada_repetido_e_na_ordem() {
    let text = ui("organize.slint");
    let bar = bar(&text);
    assert!(!bar.contains("ver.faixa-laminas"));
    assert!(!bar.contains("ver.painel"));
    // Os menus, antes do zoom, e o vão entre os dois lados.
    let menus = bar.find("text: \"Alinhar\"").unwrap();
    let gap = bar.find("horizontal-stretch: 1;").unwrap();
    let zoom = bar.find("organizar.menos-zoom").unwrap();
    assert!(menus < gap && gap < zoom);
    assert!(bar.contains("width: 120px;"), "o controle mais curto");
    assert!(bar.contains("\"Voltar (Esc)\""));
}

#[test]
fn dg_b3_a_barra_entra_nas_travas() {
    let exceptions = fs::read_to_string(manifest().join("../../xtask/src/icones.rs")).unwrap();
    assert!(!exceptions.contains("\"OrganizeBar\""));
    assert!(!ui("palette.slint").contains("component Icon"));
    let text = ui("organize.slint");
    assert!(!text.contains(" Icon {"), "os botões são da DF");
    assert_eq!(loose_spaces(&text), Vec::<&str>::new());
    assert!(bar(&text).contains("height: 40px;"));
    assert!(ui("stage.slint").contains("OrganizeUi.on ? 58px"));
}

#[test]
fn dg_b4_foco_por_tab_na_barra() {
    let text = ui("organize.slint");
    let start = text.find("component MenuButton").unwrap();
    let menu = &text[start..text.find("component BarFields").unwrap()];
    assert!(menu.contains("FocusScope"));
    assert!(menu.contains("FocusRing"));
    assert!(menu.contains("Key.Space") && menu.contains("Key.Return"));
    // Dentro do menu, as setas e o Tab andam pelas linhas.
    let rows = &text[text.find("component MenuRows").unwrap()..start];
    assert!(rows.contains("Key.Tab") && rows.contains("Key.Backtab"));
    assert!(rows.contains("Key.DownArrow") && rows.contains("Key.UpArrow"));
    assert!(rows.contains("FocusRing"));
    // O ⋯ é um botão da DF.
    assert!(bar(&text).contains("icon: Icons.more;"));
}

#[test]
fn dg_b7_laco_fora_dos_textos_da_tela() {
    for file in ["organize.slint", "look.slint"] {
        for line in ui(file)
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
        {
            assert!(!line.contains("laço"), "{file}: {line}");
        }
    }
}

#[test]
fn dg_b8_outra_cor_no_lugar_do_codigo() {
    let look = ui("look.slint");
    assert!(look.contains("\"Outra cor…\""));
    assert!(!look.contains("\"Usar\""));
    // O mesmo quadro de cores do Novo álbum.
    assert!(look.contains("ColorArea {"));
    assert!(ui("new_project.slint").contains("ColorArea {"));
}

#[test]
fn dg_b9_a_unidade_no_canto_das_reguas() {
    let text = ui("organize.slint");
    let rulers = &text[text.find("export component OrganizeRulers").unwrap()..];
    assert!(rulers.contains("text: OrganizeUi.unit;"));
    assert!(rulers.contains("background: Theme.bar;"));
}

#[test]
fn dg_b10_a_margem_de_seguranca_nao_usa_o_destaque() {
    let canvas = ui("canvas.slint");
    let from = canvas.find("// Zona segura de cada página.").unwrap();
    let safe = &canvas[from..from + canvas[from..].find("// Linhas do ímã").unwrap()];
    assert!(safe.contains("border-color: Fixed.margin;"));
    assert!(!safe.contains("Theme."));
}
