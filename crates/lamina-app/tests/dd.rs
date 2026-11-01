//! Cenários da spec `design-dialogos` (DD) que olham as telas: a moldura única, o teclado, os
//! botões, as listas e os tokens. O que é decisão vai nos testes do `lamina-editor`
//! (`tests/dd.rs`). A conferência do desenho é a captura da janela (seção 9 da spec).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("ui")
}

fn ui(file: &str) -> String {
    fs::read_to_string(ui_dir().join(file)).unwrap()
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

/// O texto do componente `name`, até o `}` da coluna 0.
fn component(text: &str, name: &str) -> String {
    let start = text
        .find(&format!("component {name} "))
        .unwrap_or_else(|| panic!("{name} não existe"));
    let rest = &text[start..];
    let end = rest.find("\n}").unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// Os diálogos: o arquivo e o componente.
const DIALOGS: &[(&str, &str)] = &[
    ("about.slint", "AboutDialog"),
    ("build_album.slint", "BuildAlbumDialog"),
    ("calibrate.slint", "CalibrateDialog"),
    ("clear_all.slint", "ClearAllDialog"),
    ("cover.slint", "CoverDialog"),
    ("dialogs.slint", "ConfirmDialog"),
    ("dialogs.slint", "MessageDialog"),
    ("dialogs.slint", "BusyOverlay"),
    ("dialogs.slint", "RecoverDialog"),
    ("export.slint", "ExportDialog"),
    ("models.slint", "ModelNameDialog"),
    ("models.slint", "ModelsDialog"),
    ("new_project.slint", "NewProjectDialog"),
    ("photo_menu.slint", "RemoveAskDialog"),
    ("relink.slint", "RelinkDialog"),
    ("shortcuts.slint", "ShortcutsDialog"),
    ("spec.slint", "PresetNameDialog"),
    ("spec.slint", "PresetsDialog"),
    ("welcome.slint", "ClearRecentsDialog"),
];

/// A página de novidades (spec PV) e a galeria de componentes não são diálogos desta spec.
fn outside_the_frame(name: &str) -> bool {
    matches!(name, "surfaces.slint" | "news.slint" | "gallery.slint")
}

#[test]
fn dd_b1_uma_moldura_so() {
    for (file, name) in DIALOGS {
        let text = component(&ui(file), name);
        assert!(
            text.starts_with(&format!("component {name} inherits Dialog")),
            "{name} não herda de Dialog"
        );
    }
    let mut files = Vec::new();
    slint_files(&ui_dir(), &mut files);
    for path in files {
        let name = path.file_name().unwrap().to_str().unwrap();
        if outside_the_frame(name) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("inherits Shade"), "{name} usa Shade");
        let panel = |l: &str| l.trim_start().starts_with("Panel {") || l.contains(": Panel {");
        assert!(!text.lines().any(panel), "{name} usa Panel");
    }
}

#[test]
fn dd_b2_o_foco_fica_dentro() {
    let frame = component(&ui("widgets/surfaces.slint"), "Dialog");
    assert!(frame.contains("popup := PopupWindow {"));
    assert!(frame.contains("close-policy: no-auto-close;"));
    assert!(frame.contains("focus-on-tab-navigation: false;"));
    // O popup nasce maior que qualquer tela, e o véu segue a janela.
    assert!(frame.contains("width: 16000px;"));
    assert!(frame.contains("width: root.width;"));
    // O popup encosta no canto da janela: o véu desconta a barra de menus desenhada nela.
    assert!(frame.contains("y: root.absolute-position.y;"));
    assert!(frame.contains("popup.show();"));
    // Quem abriu por último pega o foco de volta numa troca de diálogos.
    assert!(frame.contains("root.order == DialogFocus.last"));
    // O andamento também é da moldura, para ficar por cima e segurar as teclas.
    assert!(component(&ui("dialogs.slint"), "BusyOverlay").contains("escape: false;"));
}

#[test]
fn dd_b3_esc_em_todos_menos_recuperar_e_andamento() {
    assert!(
        component(&ui("widgets/surfaces.slint"), "Dialog").contains("Key.Escape && root.escape")
    );
    for (file, name) in DIALOGS {
        let text = component(&ui(file), name);
        if matches!(*name, "RecoverDialog" | "BusyOverlay") {
            assert!(text.contains("escape: false;"), "{name}");
        } else {
            assert!(text.contains("dismissed =>"), "{name} não trata o Esc");
        }
    }
}

/// A linha do botão de texto `button` no diálogo.
fn button_line<'a>(text: &'a str, button: &str) -> &'a str {
    text.lines()
        .find(|l| l.contains(&format!("text: {button};")))
        .unwrap_or_else(|| panic!("sem o botão {button}"))
}

#[test]
fn dd_b4_onde_o_foco_entra() {
    for (file, name) in DIALOGS {
        let text = component(&ui(file), name);
        if *name != "BusyOverlay" {
            assert!(text.contains("child-focus: true;"), "{name}");
        }
    }
    // Campo primeiro.
    assert!(component(&ui("models.slint"), "ModelNameDialog").contains("FirstLineEdit {"));
    assert!(component(&ui("spec.slint"), "PresetNameDialog").contains("FirstLineEdit {"));
    assert!(component(&ui("export.slint"), "ExportDialog").contains("FirstComboBox {"));
    assert!(ui("shortcuts.slint").contains("FirstLineEdit {"));
    assert!(ui("new_project.slint").contains("FirstLineEdit {"));
    assert!(component(&ui("build_album.slint"), "BuildAlbumDialog").contains("SpreadsField {"));
    // Pergunta: o principal.
    let confirm = component(&ui("dialogs.slint"), "ConfirmDialog");
    assert!(button_line(&confirm, "\"Salvar\"").contains("takes-focus: true"));
    // O que apaga: Cancelar.
    for (file, name) in [
        ("clear_all.slint", "ClearAllDialog"),
        ("photo_menu.slint", "RemoveAskDialog"),
        ("welcome.slint", "ClearRecentsDialog"),
    ] {
        let text = component(&ui(file), name);
        assert!(
            button_line(&text, "\"Cancelar\"").contains("takes-focus: true"),
            "{name}"
        );
    }
    let export = component(&ui("export.slint"), "ExportDialog");
    let replace = export.split("if root.step == 2:").nth(2).unwrap();
    assert!(button_line(replace, "\"Cancelar\"").contains("takes-focus: true"));
    // Só leitura: OK.
    let about = component(&ui("about.slint"), "AboutDialog");
    assert!(button_line(&about, "\"OK\"").contains("takes-focus: true"));
    // Quem recebe o foco antes da moldura é o filho: ela só pega o foco se ninguém pediu.
    assert!(component(&ui("widgets/surfaces.slint"), "Dialog").contains("if (!root.child-focus)"));
}

#[test]
fn dd_b5_um_principal_por_dialogo_e_abas_sem_ciano() {
    for (file, name) in DIALOGS {
        // Estes trocam o principal por etapa; cada etapa é conferida abaixo.
        if matches!(*name, "ExportDialog" | "NewProjectDialog" | "RelinkDialog") {
            continue;
        }
        let text = component(&ui(file), name);
        assert!(text.matches("primary: true").count() <= 1, "{name}");
    }
    let export = component(&ui("export.slint"), "ExportDialog");
    for step in export.split("if root.step == ").skip(1) {
        assert!(step.matches("primary: true").count() <= 1);
    }
    let wizard = component(&ui("new_project.slint"), "NewProjectDialog");
    for button in ["Aplicar", "Próximo", "Criar"] {
        assert!(wizard.contains(&format!("text: \"{button}\";")));
    }
    let size = ui("new_project_size.slint");
    assert!(!size.contains("primary:"), "as abas não usam o ciano");
    assert!(size.contains("FilterTab { text: \"Tamanhos prontos\""));
    assert!(size.contains("FilterTab { text: \"Personalizado\""));
}

#[test]
fn dd_b6_o_que_apaga_nao_e_ciano() {
    for (file, button) in [
        ("clear_all.slint", "ClearUi.action"),
        ("photo_menu.slint", "PhotoMenuUi.confirm-text"),
        ("export.slint", "\"Substituir\""),
        ("dialogs.slint", "\"Descartar\""),
        ("welcome.slint", "\"Limpar\""),
        ("models.slint", "\"Apagar\""),
        ("spec.slint", "\"Apagar\""),
    ] {
        let text = ui(file);
        let line = button_line(&text, button);
        assert!(line.contains("danger: true"), "{button} em {file}");
        assert!(!line.contains("primary"), "{button} em {file}");
    }
}

#[test]
fn dd_b9_o_aviso_fecha_e_volta_se_acontecer_de_novo() {
    let bar = component(&ui("topbar.slint"), "NoticeBar");
    assert!(bar.contains("tip: \"Fechar este aviso\";"));
    assert!(bar.contains("clicked => { root.dismiss(); }"));
    assert!(ui("app.slint").contains("dismiss => { root.notice-dismiss(); }"));
    // Fechar só limpa o que está na tela; um aviso novo passa pelo mesmo `notice` e aparece.
    let refresh =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/refresh.rs")).unwrap();
    assert!(refresh.contains("self.notice.clear();"));
    assert!(refresh.contains("w.set_notice(self.notice.clone().into());"));
}

#[test]
fn dd_b10_a_caixa_de_detalhes_tem_a_altura_do_texto() {
    let details = component(&ui("dialogs.slint"), "DetailsBox");
    assert!(details.contains("height: min(content.preferred-height + 2 * Space.s2, 200px);"));
    assert!(details.contains("read-only: true;"));
    assert!(!details.contains("TextEdit"));
}

#[test]
fn dd_b11_a_pergunta_de_salvar_usa_o_nome_sem_o_ponto() {
    let app = ui("app.slint");
    let start = app.find("if root.show-confirm: ConfirmDialog {").unwrap();
    let block = &app[start..start + 200];
    assert!(block.contains("name: root.album-name;"));
    assert!(!block.contains("project-title"));
}

/// As alturas de cada parte, em px, como estão nos arquivos.
const CHECK: u32 = 24;
const HINT: u32 = 17;
const TEXT: u32 = 18;
const FIELD: u32 = 30;
const CHOICE: u32 = 72;
const DISCLOSURE: u32 = 28;
const FORMAT: u32 = 148;
const S3: u32 = 12;

fn stack(rows: &[u32]) -> u32 {
    rows.iter().sum::<u32>() + S3 * u32::try_from(rows.len() - 1).unwrap()
}

#[test]
fn dd_b12_o_novo_album_cabe_em_1366_por_668() {
    let surfaces = ui("widgets/surfaces.slint");
    assert!(surfaces.contains("wizard-height: 560px;"));
    assert!(surfaces.contains("wizard: 960px;"));
    // O painel: 560 cabem nos 628 da janela de 668 (menos 40 de folga); menos as folgas de dentro
    // (24 × 2), o título e o rodapé, com 16 entre as partes.
    let panel = 560u32;
    let body = panel - 2 * 24 - TEXT - 32 - 2 * 16;
    let one = stack(&[TEXT + 12, FIELD, FORMAT, FIELD]);
    let two = stack(&[
        CHECK + 4 + HINT,
        TEXT,
        CHOICE,
        FIELD,
        FIELD,
        FIELD,
        DISCLOSURE,
        CHECK,
        FIELD,
    ]);
    let three = stack(&[FIELD, 32, CHECK, CHECK, FIELD, 32, HINT * 2]);
    let previews = TEXT + 3 * 120 + 2 * 8;
    for (name, height) in [
        ("etapa 1", one),
        ("etapa 2 aberta", two),
        ("etapa 3", three),
        ("prévias", previews),
    ] {
        assert!(height <= body, "{name}: {height} px não cabe em {body} px");
    }
}

#[test]
fn dd_b13_mais_opcoes_comeca_recolhido() {
    let wizard = ui("new_project.slint");
    let step = component(&wizard, "StepTwo");
    assert!(
        step.contains("property <bool> more-open;"),
        "sem valor: começa fechado"
    );
    assert!(step.contains("text: \"Mais opções\";"));
    for field in ["Layouts sem borda", "Mosaicos"] {
        let at = step.find(field).unwrap();
        assert!(
            step.find("if root.more-open: HorizontalLayout").unwrap() < at,
            "{field}"
        );
    }
    assert!(step.contains("if root.more-open: HorizontalLayout"));
    // Os valores continuam como hoje.
    let state = ui("new_project_ui.slint");
    assert!(state.contains("in-out property <bool> bleed-page: true;"));
    assert!(state.contains("in-out property <bool> mosaics: true;"));
    assert!(state.contains("in-out property <int> reuse: 1;"));
}

#[test]
fn dd_b16_listas_vazias_dizem_o_que_fazer() {
    for (file, name, list, selected) in [
        (
            "models.slint",
            "ModelsDialog",
            "ModelUi.models",
            "ModelUi.list-selected",
        ),
        (
            "spec.slint",
            "PresetsDialog",
            "PresetUi.user-presets",
            "PresetUi.list-selected",
        ),
    ] {
        let text = component(&ui(file), name);
        assert!(
            text.contains(&format!("if {list}.length == 0: EmptyState {{")),
            "{name}"
        );
        assert!(
            text.contains(&format!("if {selected} >= 0: HorizontalLayout {{")),
            "{name}"
        );
    }
}

#[test]
fn dd_b17_tudo_que_se_clica_recebe_foco() {
    let size = ui("new_project_size.slint");
    let wizard = ui("new_project.slint");
    let color = ui("new_project_color.slint");
    for text in [
        component(&size, "FormatButton"),
        component(&size, "BinderPicker"),
        component(&size, "Disclosure"),
        component(&wizard, "Choice"),
        component(&color, "ColorSwatch"),
    ] {
        assert!(text.contains("FocusStop {"), "{text}");
        assert!(text.contains("accessible-role:"), "{text}");
        assert!(text.contains("accessible-label:"), "{text}");
        assert!(text.contains("accessible-action-default"), "{text}");
    }
    let list = component(&ui("widgets/surfaces.slint"), "PickList");
    assert!(list.contains("Key.DownArrow"));
    assert!(list.contains("FocusRing {"));
    assert!(list.contains("accessible-item-selected:"));
}

#[test]
fn dd_b18_as_listas_da_janela_tem_uma_parada_de_tab() {
    let strip = component(&ui("spread_strip.slint"), "SpreadStrip");
    assert!(strip.contains("keys := FocusScope {"));
    assert!(strip.contains("if keys.has-focus: FocusRing {"));
    assert!(!strip.contains("Key.LeftArrow"), "← → seguem até a janela");

    let line = component(&ui("timeline.slint"), "TimelineStrip");
    assert!(line.contains("keys := FocusScope {"));
    assert!(line.contains("Key.LeftArrow") && line.contains("Key.RightArrow"));
    assert!(line.contains("root.go-to(root.mark.spread);"));
    assert!(line.contains("FocusRing {"));

    let history = component(&ui("history.slint"), "HistoryPanel");
    assert!(history.contains("Key.UpArrow") && history.contains("Key.DownArrow"));
    assert!(history.contains("root.go(root.mark);"));
    assert!(history.contains("shown: keys.has-focus && i == root.mark;"));
}

fn px_after(text: &str, property: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| {
            l.strip_prefix(property)
                .and_then(|r| r.strip_prefix(": "))
                .is_some_and(|r| {
                    r.starts_with(|c: char| c.is_ascii_digit())
                        && r.ends_with("px;")
                        && !r.starts_with("0px")
                })
        })
        .map(str::to_owned)
}

#[test]
fn dd_b19_so_tokens_sem_codigo_sobrando() {
    for (file, name) in DIALOGS {
        let text = component(&ui(file), name);
        for token in ["Theme.g4", "Theme.g6"] {
            assert!(!text.contains(token), "{token} em {name}");
        }
        for property in [
            "spacing",
            "padding",
            "padding-left",
            "padding-right",
            "padding-top",
            "padding-bottom",
        ] {
            assert_eq!(
                px_after(&text, property),
                None,
                "{property} em px em {name}"
            );
        }
    }
    for file in [
        "new_project.slint",
        "new_project_size.slint",
        "new_project_color.slint",
    ] {
        let text = ui(file);
        for token in ["Theme.g4", "Theme.g6"] {
            assert!(!text.contains(token), "{token} em {file}");
        }
        assert_eq!(px_after(&text, "spacing"), None, "{file}");
    }
    let spec = ui("spec.slint");
    assert!(!spec.contains("SpecDialog") && !spec.contains("SpecForm"));
    assert!(ui("widgets/surfaces.slint").contains("in property <length> label-width: 120px;"));
    assert!(ui("calibrate.slint").contains("NeutralSlider {"));
}

#[test]
fn dd_b20_o_sobre_mantem_o_logotipo_e_o_criador() {
    let about = component(&ui("about.slint"), "AboutDialog");
    assert!(about.contains("lamina-logo-escuro.svg"));
    assert!(about.contains("Criado por Alberth Klinsmann."));
}
