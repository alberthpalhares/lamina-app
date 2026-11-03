//! Cenários da spec `aparencia-da-janela` (AJ) que não precisam da janela: as escolhas, o
//! `janela.json`, a leitura do sistema e o que elas não tocam. As telas ficam em
//! `crates/lamina-app/src/window_look/tests.rs` e `crates/lamina-app/tests/aj.rs`.

use std::path::Path;

use lamina_export::ExportOptions;

use super::common::Base;
use super::tl_common::{editor_at, three_spreads};
use crate::shortcuts::sheet;
use crate::window_look::{
    BackdropChoice, MotionChoice, ThemeChoice, gnome_reduces, mac_reduces, menu_differs, scale_of,
    system_is_dark, text_size,
};
use crate::{
    Command, Editor, Install, Menu, Platform, STRIPS_VERSION, Shortcut, StripsStore, export_project,
};

fn with_file(text: &str) -> (tempfile::TempDir, Editor, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, text).unwrap();
    let mut editor = Editor::new();
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    (dir, editor, file)
}

fn json(file: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

#[test]
fn aj_b1_abre_com_e_sem_album_e_aparece_nos_atalhos() {
    let command = Command::Preferences;
    assert_eq!(command.name(), "editar.preferencias");
    assert_eq!(command.menu(), Menu::Edit);
    assert_eq!(Command::from_shortcut(Shortcut::ctrl(',')), Some(command));
    assert!(Editor::new().available(command));
    assert!(editor_at(three_spreads(), 0).available(command));
    let groups = sheet(Platform::Windows);
    let rows = groups
        .iter()
        .find(|g| g.title == "Editar")
        .map(|g| g.rows.iter().map(|r| r.does).collect::<Vec<_>>())
        .unwrap();
    assert!(rows.contains(&"Preferências…"), "{rows:?}");
}

#[test]
fn aj_b4_grava_na_hora_sem_mexer_nas_faixas() {
    let (_dir, mut editor, file) = with_file(r#"{"versao": 3, "fotos": false, "painel": false}"#);
    editor.set_theme_choice(ThemeChoice::Light);
    assert_eq!(editor.window_look().theme, ThemeChoice::Light);
    let saved = json(&file);
    assert_eq!(saved["tema"], "claro");
    assert_eq!(saved["fotos"], false);
    assert_eq!(saved["painel"], false);
}

#[test]
fn aj_b5_voltar_ao_padrao() {
    let (_dir, mut editor, file) = with_file("{}");
    editor.set_theme_choice(ThemeChoice::System);
    editor.set_backdrop_choice(BackdropChoice::Gray);
    editor.set_text_size(130);
    editor.set_motion_choice(MotionChoice::Off);
    editor.reset_window_look();
    let look = editor.window_look();
    assert_eq!(look.theme, ThemeChoice::Dark);
    assert_eq!(look.backdrop, BackdropChoice::Theme);
    assert_eq!(look.text_size, 100);
    assert_eq!(look.motion, MotionChoice::System);
    let saved = json(&file);
    assert_eq!(saved["tema"], "escuro");
    assert_eq!(saved["fundo_lamina"], "tema");
    assert_eq!(saved["tamanho_texto"], 100);
    assert_eq!(saved["movimento"], "sistema");
    assert_eq!(saved["reduzir_movimento"], false);
}

#[test]
fn aj_b6_nao_mexe_no_album() {
    let base = Base::with_photos(4, (600, 400));
    let file = base.save(base.project(2), "album/festa.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    assert!(!editor.session().unwrap().is_dirty());
    let undo_before = editor.session().unwrap().can_undo();
    editor.set_theme_choice(ThemeChoice::Light);
    editor.set_backdrop_choice(BackdropChoice::Gray);
    editor.set_text_size(130);
    editor.set_motion_choice(MotionChoice::Off);
    let session = editor.session().unwrap();
    assert!(!session.is_dirty());
    assert_eq!(session.can_undo(), undo_before);
}

#[test]
fn aj_b7_nao_gravou_vale_na_sessao_com_aviso() {
    let dir = tempfile::tempdir().unwrap();
    // A pasta no lugar do arquivo: não dá para gravar.
    let file = dir.path().join("janela.json");
    std::fs::create_dir(&file).unwrap();
    let mut editor = Editor::new();
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    editor.set_theme_choice(ThemeChoice::Light);
    assert_eq!(editor.window_look().theme, ThemeChoice::Light);
    let notice = editor.take_notice().unwrap();
    assert!(
        notice.starts_with("Não foi possível guardar as preferências da janela:"),
        "{notice}"
    );
}

#[test]
fn aj_b8_o_comando_de_movimento_passa_pelos_dois_campos() {
    let (_dir, mut editor, file) = with_file("{}");
    let command = Command::ToggleReduceMotion;
    assert_eq!(editor.window_look().motion, MotionChoice::System);
    assert!(editor.run(command));
    assert_eq!(editor.window_look().motion, MotionChoice::Off);
    assert_eq!(json(&file)["movimento"], "sem");
    assert_eq!(json(&file)["reduzir_movimento"], true);
    assert!(editor.run(command));
    assert_eq!(editor.window_look().motion, MotionChoice::System);
    assert_eq!(json(&file)["movimento"], "sistema");
    assert_eq!(json(&file)["reduzir_movimento"], false);
    editor.set_motion_choice(MotionChoice::On);
    assert!(editor.run(command));
    assert_eq!(editor.window_look().motion, MotionChoice::Off);
}

#[test]
fn aj_b9_o_padrao_e_escuro_mesmo_com_o_sistema_claro() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    let mut editor = Editor::new();
    editor.set_strips(StripsStore::load_for(&file, Install::New));
    let theme = editor.window_look().theme;
    assert_eq!(theme, ThemeChoice::Dark);
    assert!(theme.is_dark(Some(false)));
}

#[test]
fn aj_b10_seguir_o_sistema() {
    let theme = ThemeChoice::System;
    assert!(!theme.is_dark(Some(false)));
    assert!(theme.is_dark(Some(true)));
    assert!(theme.is_dark(None));
    assert!(!ThemeChoice::Light.is_dark(Some(true)));
}

#[test]
fn aj_b15_a_exportacao_nao_muda() {
    let base = Base::with_photos(4, (600, 400));
    let file = base.save(base.project(2), "album/festa.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    let options = ExportOptions::default();
    let export = |editor: &Editor, name: &str| {
        let folder = base.path(name);
        let project = editor.session().unwrap().project().clone();
        export_project(&project, &folder, &options, &|_, _| {}).unwrap();
        let mut files: Vec<_> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        files
            .iter()
            .map(|f| std::fs::read(f).unwrap())
            .collect::<Vec<_>>()
    };
    let first = export(&editor, "antes");
    editor.set_theme_choice(ThemeChoice::Light);
    editor.set_backdrop_choice(BackdropChoice::Gray);
    editor.set_text_size(130);
    editor.set_motion_choice(MotionChoice::Off);
    let second = export(&editor, "depois");
    assert!(!first.is_empty());
    assert_eq!(first, second);
}

#[test]
fn aj_b16_o_arquivo_do_album_nao_muda() {
    let base = Base::with_photos(4, (600, 400));
    let file = base.save(base.project(2), "album/festa.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    editor.set_theme_choice(ThemeChoice::Light);
    editor.set_backdrop_choice(BackdropChoice::Gray);
    editor.set_text_size(130);
    editor.save_to(None).unwrap();
    let saved: serde_json::Value = json(&file);
    let text = saved.to_string();
    for field in ["tema", "fundo_lamina", "tamanho_texto", "movimento"] {
        assert!(!text.contains(&format!("\"{field}\"")), "{field}");
    }
    assert_eq!(saved["schema_version"], 8);
}

#[test]
fn aj_b20_tamanhos_validos() {
    for (from, to) in [
        (90.0, 90),
        (130.0, 130),
        (500.0, 130),
        (10.0, 90),
        (95.0, 100),
        (104.0, 100),
    ] {
        assert_eq!(text_size(from), to, "{from}");
    }
    let (_dir, editor, _file) = with_file(r#"{"tamanho_texto": 500}"#);
    assert_eq!(editor.window_look().text_size, 130);
    assert_eq!(scale_of(130), 1.3);
    assert_eq!(scale_of(100), 1.0);
}

#[test]
fn aj_b22_movimento_com_as_tres_escolhas() {
    for system in [Some(true), Some(false), None] {
        assert!(MotionChoice::Off.reduces(system));
        assert!(!MotionChoice::On.reduces(system));
    }
    assert!(MotionChoice::System.reduces(Some(true)));
    assert!(!MotionChoice::System.reduces(Some(false)));
    assert!(!MotionChoice::System.reduces(None));
}

#[test]
fn aj_b23_le_de_novo_sem_gravar() {
    let (_dir, mut editor, file) = with_file("{}");
    editor.set_motion_choice(MotionChoice::System);
    let before = std::fs::read(&file).unwrap();
    editor.set_system_motion(Some(false));
    assert!(!editor.reduce_motion());
    assert!(editor.durations().slide > 0);
    editor.set_system_motion(Some(true));
    assert!(editor.reduce_motion());
    assert_eq!(editor.durations().slide, 0);
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn aj_b24_leitura_do_sistema() {
    assert_eq!(mac_reduces("1\n"), Some(true));
    assert_eq!(mac_reduces("0\n"), Some(false));
    assert_eq!(mac_reduces("erro"), None);
    assert_eq!(gnome_reduces("false\n"), Some(true));
    assert_eq!(gnome_reduces("true\n"), Some(false));
    assert_eq!(gnome_reduces(""), None);
}

#[test]
fn aj_b25_arquivo_antigo() {
    let (_d, on, _f) = with_file(r#"{"versao": 3, "fotos": false, "reduzir_movimento": true}"#);
    let look = on.window_look();
    assert_eq!(look.theme, ThemeChoice::Dark);
    assert_eq!(look.backdrop, BackdropChoice::Theme);
    assert_eq!(look.text_size, 100);
    assert_eq!(look.motion, MotionChoice::Off);
    assert!(on.reduce_motion());
    assert!(!on.strips().photos);
    let (_d, off, _f) = with_file(r#"{"versao": 3, "reduzir_movimento": false}"#);
    assert_eq!(off.window_look().motion, MotionChoice::System);
    assert_eq!(off.strips().version, STRIPS_VERSION);
}

#[test]
fn aj_b26_voltar_a_um_lamina_anterior() {
    let (_dir, mut editor, file) = with_file("{}");
    editor.set_theme_choice(ThemeChoice::Light);
    editor.set_motion_choice(MotionChoice::Off);
    let saved = json(&file);
    assert_eq!(saved["reduzir_movimento"], true);
    // A versão anterior lê só o que conhece, sem aviso, e grava sem o que não conhece.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Before {
        versao: u8,
        reduzir_movimento: bool,
    }
    let before: Before = serde_json::from_value(saved).unwrap();
    assert!(before.reduzir_movimento);
    assert_eq!(before.versao, STRIPS_VERSION);
    let rewritten = serde_json::to_string(&before).unwrap();
    assert!(!rewritten.contains("tema"));
    let (_d, back, _f) = with_file(&rewritten);
    assert_eq!(back.window_look().theme, ThemeChoice::Dark);
    assert_eq!(back.window_look().motion, MotionChoice::Off);
}

#[test]
fn aj_b27_valor_estranho_nao_derruba_o_resto() {
    let text = r#"{"versao": 3, "tema": "alto-contraste", "fundo_lamina": 3, "movimento": null,
        "tamanho_texto": "grande", "fotos": false,
        "tamanho_real": [{"monitor": "M", "px_por_mm": 4.0}]}"#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, text).unwrap();
    let store = StripsStore::load_for(&file, Install::Existing);
    assert_eq!(store.warning, None);
    let calibrations = store.calibrations.len();
    assert_eq!(store.strips.theme, ThemeChoice::Dark);
    assert_eq!(store.strips.backdrop, BackdropChoice::Theme);
    assert_eq!(store.strips.text_size, 100);
    assert_eq!(store.strips.motion_choice(), MotionChoice::System);
    assert!(!store.strips.photos);
    // As calibrações continuam lendo como leriam sem os campos estranhos.
    let clean = dir.path().join("limpo.json");
    std::fs::write(
        &clean,
        r#"{"versao": 3, "fotos": false,
        "tamanho_real": [{"monitor": "M", "px_por_mm": 4.0}]}"#,
    )
    .unwrap();
    let plain = StripsStore::load_for(&clean, Install::Existing);
    assert_eq!(calibrations, plain.calibrations.len());
    assert_eq!(calibrations, 1);
}

#[test]
fn aj_b39_a_barra_de_menus_fica_como_abriu() {
    // Aberto em escuro: ela só difere no tema claro.
    assert!(!menu_differs(true, Some(true), Some(false)));
    assert!(menu_differs(false, Some(true), Some(false)));
    // Aberto seguindo o sistema: ela acompanha o sistema.
    assert!(!menu_differs(false, None, Some(false)));
    assert!(menu_differs(true, None, Some(false)));
    // Sem saber nada, não avisa.
    assert!(!menu_differs(false, None, None));
    // No Windows, o sistema sempre responde.
    assert!(cfg!(not(windows)) || system_is_dark().is_some());
}

#[test]
fn aj_b28_preferencias_nao_gravam_em_outro_arquivo() {
    let (dir, mut editor, file) = with_file("{}");
    editor.set_theme_choice(ThemeChoice::Light);
    editor.set_backdrop_choice(BackdropChoice::Gray);
    editor.set_text_size(130);
    editor.set_motion_choice(MotionChoice::Off);
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![file.file_name().unwrap().to_owned()]);
}
