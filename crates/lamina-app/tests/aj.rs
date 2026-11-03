//! Cenários da spec `aparencia-da-janela` (AJ) que olham o texto das telas. O desenho fica em
//! `src/window_look/tests.rs`, e a lógica, no `lamina-editor` (`tests/aj.rs`).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ui(file: &str) -> String {
    fs::read_to_string(manifest().join("ui").join(file)).unwrap()
}

fn slint_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            slint_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "slint") {
            out.push(path);
        }
    }
}

fn all_slint() -> Vec<(String, String)> {
    let mut files = Vec::new();
    slint_files(&manifest().join("ui"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            (name, fs::read_to_string(&path).unwrap())
        })
        .collect()
}

/// O texto do componente `name`, até o `}` da coluna 0.
fn component(text: &str, name: &str) -> String {
    let start = text
        .find(&format!("component {name} "))
        .unwrap_or_else(|| panic!("{name} não existe"));
    let rest = &text[start..];
    let end = rest.find("\n}").unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// O elemento que contém `mark`: da linha do `if`/abertura até fechar as chaves.
fn element_with(text: &str, mark: &str) -> String {
    let at = text.find(mark).unwrap_or_else(|| panic!("sem {mark}"));
    let open = text[..at].rfind('{').unwrap();
    let mut depth = 0;
    let mut end = open;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    text[open..=end].to_owned()
}

#[test]
fn aj_b2_a_moldura_e_a_de_todos_com_o_veu() {
    let text = component(&ui("preferences.slint"), "PreferencesDialog");
    assert!(text.starts_with("component PreferencesDialog inherits Dialog"));
    assert!(text.contains("dismissed =>"));
    // Correção do dono (2026-11-02): sem cópia desfocada — o véu é o de todos.
    let with_copy: Vec<String> = all_slint()
        .into_iter()
        .filter(|(_, text)| text.contains("has-behind"))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(with_copy, Vec::<String>::new());
}

#[test]
fn aj_b8_reduzir_movimento_saiu_de_visualizar() {
    let app = ui("app.slint");
    assert!(!app.contains("\"ver.reduzir-movimento\""));
    assert!(!app.contains("reduce-motion"));
    assert!(app.contains("\"editar.preferencias\""));
    let edit = &app[app.find("title: \"Editar\"").unwrap()..app.find("title: \"Lâmina\"").unwrap()];
    assert!(edit.contains("Preferências…  (Ctrl+,)"));
}

#[test]
fn aj_b11_a_barra_de_titulo_e_pedida_de_novo_ao_voltar() {
    let src = manifest().join("src");
    let hook = fs::read_to_string(src.join("importing.rs")).unwrap();
    assert!(hook.contains("crate::window_look::on_event"));
    let look = fs::read_to_string(src.join("window_look.rs")).unwrap();
    let on_event =
        &look[look.find("pub fn on_event").unwrap()..look.find("pub fn open(app").unwrap()];
    assert!(on_event.contains("Focused(true)") && on_event.contains("read_scheme"));
    assert!(look.contains("set_theme"));
}

#[test]
fn aj_b19_o_que_fica_direto_sobre_o_fundo_nao_usa_a_cor_do_tema() {
    let stage = ui("stage.slint");
    let hint = element_with(&stage, "text: root.hint;");
    assert!(hint.contains("color: Backdrop.text-2"), "{hint}");
    let empty = component(&ui("importing.slint"), "EmptyAlbum");
    assert!(!empty.contains("color: Theme."), "EmptyAlbum: {empty}");
    let roll = ui("roll.slint");
    for mark in [
        "Lâmina nova\\n",
        "border-color: Backdrop.strong;\n        border-radius: Radius.hair;",
    ] {
        let element = element_with(&roll, mark);
        assert!(!element.contains("color: Theme.text"), "{mark}: {element}");
    }
    let gap = element_with(&roll, "root.hit.kind == 3 || root.gripping");
    let _ = gap;
    assert!(roll.contains("background: Backdrop.strong;"));
    assert!(stage.contains("background: root.has-project ? Backdrop.color : transparent;"));
    assert!(ui("app.slint").contains("background: Theme.stage;"));
}

#[test]
fn aj_b37_no_meio_e_sem_responder_ao_mouse() {
    let frame = component(&ui("widgets/surfaces.slint"), "Dialog");
    assert!(!frame.contains("at-side"));
    // Correção do dono (2026-11-02): o véu escuro no lugar da foto desfocada.
    assert!(frame.contains("background: Theme.veil;"));
    assert!(frame.contains("TouchArea {"));
}

#[test]
fn aj_b38_o_movimento_se_demonstra_na_opcao() {
    let text = ui("preferences.slint");
    let demo = component(&text, "MotionDemo");
    assert!(demo.contains("running: root.playing;"));
    assert!(demo.contains("PrefsUi.demo-ms"));
    assert!(
        !demo.contains("Motion."),
        "a demonstração não segue o Motion"
    );
    // Tab de uma opção à outra: o aviso de quem perde o foco chega depois e não apaga o de quem ganhou.
    assert!(text.contains("else if (root.focus-at == i)"));
    let dialog = component(&text, "PreferencesDialog");
    assert!(dialog.contains("playing: motion.showing >= 0;"));
    assert!(
        dialog
            .contains("slides: motion.showing == 1 || (motion.showing == 0 && !PrefsUi.reduced);")
    );
}

#[test]
fn aj_b39_o_tema_vai_junto_ao_criar_a_janela() {
    let main = fs::read_to_string(manifest().join("src/main.rs")).unwrap();
    assert!(main.contains(".with_theme(opening_theme)"));
    assert!(main.contains("window_look::opened_dark(chosen_theme)"));
}

#[test]
fn aj_b40_botoes_e_caixas_usam_o_fundo_da_letra_branca() {
    let buttons = ui("widgets/buttons.slint");
    assert!(buttons.contains("Theme.accent-fill-hover : Theme.accent-fill"));
    assert!(buttons.contains("root.selected ? Theme.accent-fill :"));
    assert!(ui("widgets/controls.slint").contains("Theme.accent-fill : Theme.text-disabled"));
}

#[test]
fn aj_b30_a_barra_amarela_nao_depende_do_tema() {
    // Ativa quando a spec AV chegar à `main`: até lá, os tokens ainda não existem.
    let palette = ui("theme/palette.slint");
    if !palette.contains("attention") {
        return;
    }
    let bar = ui("update_bar.slint");
    assert!(!bar.contains("Theme."), "a barra usa o tema");
}

#[test]
fn aj_b31_o_html_de_aprovacao_nao_le_as_preferencias() {
    // Ativa quando a spec AH chegar à `main`: até lá, o visualizador ainda não existe.
    let viewer = manifest().join("../lamina-export/src/approval/viewer.html");
    let Ok(text) = fs::read_to_string(viewer) else {
        return;
    };
    for word in ["janela.json", "Theme", "tamanho_texto", "fundo_lamina"] {
        assert!(!text.contains(word), "{word}");
    }
}

#[test]
fn aj_b33_o_logotipo_escolhe_a_imagem_pelo_tema() {
    let choice = r#"Theme.dark ? @image-url("../icons/lamina-logo-escuro.svg") : @image-url("../icons/lamina-logo.svg")"#;
    assert!(ui("about.slint").contains(choice));
    assert!(ui("topbar.slint").contains(choice));
    assert!(manifest().join("icons/lamina-logo.svg").is_file());
}

#[test]
fn aj_b34_nenhuma_tela_importa_o_slider_padrao() {
    for (name, text) in all_slint() {
        let imports_it = text
            .lines()
            .filter(|l| l.starts_with("import ") && l.contains("std-widgets.slint"))
            .any(|l| l.contains("Slider"));
        assert!(!imports_it, "{name}");
    }
}

#[test]
fn aj_b35_a_largura_que_guarda_texto_cresce_com_ele() {
    let size = ui("new_project_size.slint");
    assert!(size.contains(r#"Label { text: "Encadernadora"; width: 90px * Type.k; }"#));
    assert!(size.contains(r#"Label { text: "Tamanho"; width: 90px * Type.k; }"#));
    assert!(size.contains("min-width: root.label-width * Type.k;"));
    let build = ui("build_album.slint");
    assert!(build.contains(r#"text: "Variedade dos desenhos"; width: 170px * Type.k;"#));
    assert!(build.contains(r#"text: "Número de lâminas"; width: 170px * Type.k;"#));
    assert!(ui("widgets/surfaces.slint").contains("width: root.label-width * Type.k;"));
    assert!(ui("cover.slint").contains("(CoverUi.has-cover ? 1 : Type.k)"));
}

#[test]
fn aj_b36_janela_pequena_em_130() {
    let stage = ui("stage.slint");
    let hint = element_with(&stage, "text: root.hint;");
    assert!(hint.contains("overflow: elide;") && hint.contains("width: parent.width"));
    assert!(
        stage.contains("visible: (parent.width - root.canvas-width) / 2 >= self.x + self.width;")
    );
    let summary = component(&ui("new_project_size.slint"), "SizeSummary");
    assert!(summary.contains("min-width: 200px;") && !summary.contains("\n    width: 320px"));
    assert!(
        ui("welcome.slint")
            .contains("min(root.image-width + 400px + Space.s6, root.width - 2 * Space.s5)")
    );
}
