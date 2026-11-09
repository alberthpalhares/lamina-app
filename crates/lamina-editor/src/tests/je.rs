//! Cenários da spec `janela-enxuta` (JE) que rodam sem janela: a versão do estado da janela, a
//! instalação nova, o painel temporário, as dicas da barra de cima e o quadro vazio na tela.

use std::path::Path;

use lamina_core::FrameContent;

use super::tl_common::{editor_at, three_spreads};
use crate::{
    Area, BAR_COMMANDS, Command, Editor, FrameKind, Install, Key, Menu, STRIPS_UNREADABLE,
    STRIPS_VERSION, Shortcut, Strips, StripsStore, UpdatesStore, bar_tip, spread_view,
};

fn json(file: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// O editor com o painel escondido e salvo assim, e o texto do arquivo.
fn hidden_and_saved(dir: &Path) -> (Editor, std::path::PathBuf, String) {
    let file = dir.join("janela.json");
    let text = r#"{"versao": 2, "laminas": false, "linha_do_tempo": true, "fotos": true, "painel": false}"#;
    std::fs::write(&file, text).unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert!(!editor.panel_shown());
    (editor, file, text.to_owned())
}

/// Os dois primeiros quadros da lâmina 1 são fotos.
fn photo_frames(editor: &Editor) -> (usize, usize) {
    let spread = &editor.session().unwrap().project().spreads[0];
    let photos: Vec<usize> = spread
        .frames
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.content, FrameContent::Photo { .. }))
        .map(|(i, _)| i)
        .collect();
    (photos[0], photos[1])
}

#[test]
fn je_b1_o_quadro_vazio_chega_a_tela_como_vazio() {
    let mut editor = editor_at(three_spreads(), 0);
    assert!(
        editor.run(Command::NewPhotoFrame),
        "um quadro vazio na lâmina"
    );
    let project = editor.session().unwrap().project().clone();
    let area = Area {
        width: 1200.0,
        height: 700.0,
        margin: 10.0,
    };
    let view = spread_view(&project, 0, area, &[]).unwrap();
    let kinds: Vec<&FrameKind> = view.frames.iter().map(|f| &f.kind).collect();
    assert_eq!(
        kinds
            .iter()
            .filter(|k| matches!(k, FrameKind::Empty))
            .count(),
        1,
        "só o quadro novo é vazio"
    );
    assert!(
        kinds
            .iter()
            .filter(|k| !matches!(k, FrameKind::Empty))
            .all(|k| matches!(k, FrameKind::Photo { .. })),
        "os outros continuam fotos"
    );
}

#[test]
fn je_b3_instalacao_nova_comeca_com_o_painel_oculto() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config").join("janela.json");
    let updates = UpdatesStore::load(&dir.path().join("config").join("atualizacoes.json"));
    let install = Install::from_last_version(updates.prefs.last_version.as_deref());
    assert_eq!(install, Install::New);
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, install));
    let s = editor.strips();
    assert!(!editor.panel_shown(), "o painel começa escondido");
    // A faixa Lâminas começa aberta (spec FX, regra 10; antes, a JE a desligava).
    assert!(s.spreads, "a faixa Lâminas começa aberta");
    // Lâminas e Sequência dividem a faixa, em abas (spec DJ, regra 9).
    assert!(!s.timeline && s.photos);
    assert_eq!(s.version, STRIPS_VERSION);
    assert_eq!(s, Strips::fresh());
    assert!(!file.exists(), "nada é gravado até a primeira mudança");
    assert_eq!(editor.take_notice(), None);
}

#[test]
fn je_b4_quem_ja_usa_fica_como_estava() {
    let dir = tempfile::tempdir().unwrap();
    let updates_file = dir.path().join("atualizacoes.json");
    std::fs::write(&updates_file, r#"{"ultima_versao": "0.1.0-beta.4"}"#).unwrap();
    let last = UpdatesStore::load(&updates_file).prefs.last_version;
    let install = Install::from_last_version(last.as_deref());
    assert_eq!(install, Install::Existing);
    let cases = [
        (Some(r#"{"laminas": true, "painel": false}"#), false),
        (Some(r#"{"laminas": true, "linha_do_tempo": true}"#), true),
        (None, true),
    ];
    for (i, (text, panel)) in cases.into_iter().enumerate() {
        let file = dir.path().join(format!("janela-{i}.json"));
        if let Some(text) = text {
            std::fs::write(&file, text).unwrap();
        }
        let mut editor = editor_at(three_spreads(), 0);
        editor.set_strips(StripsStore::load_for(&file, install));
        assert_eq!(editor.panel_shown(), panel, "caso {i}");
        assert_eq!(editor.take_notice(), None, "caso {i}");
    }
    // O arquivo vale mais que a falta do `atualizacoes.json`.
    let file = dir.path().join("so-janela.json");
    std::fs::write(&file, r#"{"painel": true}"#).unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::New));
    assert!(editor.panel_shown());
}

#[test]
fn je_b5_a_faixa_de_laminas_antiga_e_desligada_uma_vez() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(
        &file,
        r#"{"laminas": true, "linha_do_tempo": false, "fotos": true, "painel": true, "reduzir_movimento": true}"#,
    )
    .unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    let s = editor.strips();
    // Desde a spec FX (regra 11), um arquivo antigo abre a faixa Lâminas uma vez.
    assert!(s.spreads, "a faixa vem aberta");
    assert!(!s.timeline && s.photos && s.panel && s.reduce_motion);
    let alt1 = Command::from_shortcut(Shortcut::alt('1')).unwrap();
    assert!(editor.run(alt1));
    let saved = json(&file);
    assert_eq!(saved["versao"], STRIPS_VERSION);
    assert_eq!(saved["laminas"], false);
    let mut next = editor_at(three_spreads(), 0);
    next.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert!(!next.strips().spreads, "a escolha fica");
    // Um arquivo de um Lamina mais novo vale como está.
    std::fs::write(&file, r#"{"versao": 4, "laminas": false}"#).unwrap();
    let store = StripsStore::load_for(&file, Install::Existing);
    assert!(!store.strips.spreads);
    assert_eq!(store.strips.version, 4);
}

#[test]
fn je_b6_abrir_o_painel_guarda_a_escolha() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config").join("janela.json");
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::New));
    assert!(!editor.panel_shown());
    assert!(editor.run(Command::from_name("ver.painel").unwrap()));
    let saved = json(&file);
    assert_eq!(saved["painel"], true);
    assert_eq!(saved["versao"], STRIPS_VERSION);
    let mut next = editor_at(three_spreads(), 0);
    next.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert!(
        next.panel_shown(),
        "a segunda abertura vem com o painel aberto"
    );
    assert!(next.run(Command::TogglePanel));
    assert_eq!(json(&file)["painel"], false);
}

#[test]
fn je_b7_painel_temporario_sem_gravar_nada() {
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, file, text) = hidden_and_saved(dir.path());
    let (first, second) = photo_frames(&editor);
    editor.select_frame(Some(first));
    assert!(editor.run(Command::AdjustPhoto));
    assert!(editor.panel_shown(), "Ajustar a foto… mostra o painel");
    assert!(editor.take_details_request(), "na aba Detalhes");
    editor.select_frame(Some(second));
    assert!(!editor.panel_shown(), "outro quadro encerra o pedido");
    editor.select_frame(Some(first));
    assert!(!editor.panel_shown(), "o pedido que acabou não volta");
    assert!(editor.run(Command::NewText));
    assert!(editor.selected_text().is_some());
    assert!(editor.panel_shown(), "T mostra o painel com o texto novo");
    let esc = Command::from_shortcut(Shortcut::key(Key::Escape)).unwrap();
    assert!(editor.run(esc));
    assert!(!editor.panel_shown(), "o Esc o esconde");
    assert!(editor.run(Command::Organize));
    assert!(editor.panel_shown(), "o modo organizar mostra o painel");
    editor.select_frame(Some(second));
    assert!(
        editor.panel_shown(),
        "selecionar no modo não encerra o pedido"
    );
    assert!(editor.run(Command::Organize));
    assert!(!editor.panel_shown(), "sair do modo encerra o pedido");
    assert!(!editor.strips().panel, "a escolha salva continua oculta");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        text,
        "nada gravado"
    );
}

#[test]
fn je_b7_duplo_clique_num_texto_pede_o_painel() {
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, _, _) = hidden_and_saved(dir.path());
    assert!(editor.run(Command::NewText));
    let text = editor.selected_frame().unwrap();
    assert!(editor.run(Command::Deselect));
    assert!(!editor.panel_shown());
    editor.double_click(Some(text));
    assert!(
        editor.panel_shown(),
        "duplo clique num texto mostra o painel"
    );
    assert!(editor.run(Command::NextSpread));
    assert!(!editor.panel_shown(), "trocar de lâmina encerra o pedido");
}

#[test]
fn je_b8_o_botao_painel_com_o_painel_temporario() {
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, file, text) = hidden_and_saved(dir.path());
    let (first, _) = photo_frames(&editor);
    editor.select_frame(Some(first));
    assert!(editor.run(Command::AdjustPhoto));
    assert!(editor.panel_shown());
    assert!(editor.run(Command::TogglePanel));
    assert!(!editor.panel_shown(), "esconde e encerra o pedido");
    assert!(!editor.strips().panel);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
    assert!(editor.run(Command::TogglePanel));
    assert!(editor.panel_shown());
    assert_eq!(json(&file)["painel"], true);
}

#[test]
fn je_b9_o_que_sai_da_barra_continua_no_registro() {
    let names: Vec<&str> = BAR_COMMANDS.iter().map(|c| c.name()).collect();
    // O botão da faixa de modelos entrou (pedido do dono 2026-11-08; spec JE).
    assert_eq!(
        names,
        [
            "arquivo.fechar",
            "ver.faixa-layouts",
            "ver.faixas",
            "ver.painel",
            "arquivo.exportar"
        ],
        "desde a spec FX, o botão das faixas de baixo no lugar do de fotos"
    );
    let kept = [
        ("arquivo.novo", Shortcut::ctrl('n'), Menu::File),
        ("arquivo.abrir", Shortcut::ctrl('o'), Menu::File),
        ("arquivo.salvar", Shortcut::ctrl('s'), Menu::File),
        (
            "lamina.novo-texto",
            Shortcut::key(Key::Char('t')),
            Menu::Spread,
        ),
    ];
    for (name, key, menu) in kept {
        let command = Command::from_name(name).unwrap();
        assert_eq!(command.shortcut(), Some(key), "{name}");
        assert_eq!(command.menu(), menu, "{name}");
        assert_eq!(Command::from_shortcut(key), Some(command), "{name}");
    }
}

#[test]
fn je_b10_botoes_com_estado_e_dica() {
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, _, _) = hidden_and_saved(dir.path());
    let tips = |e: &Editor| {
        (
            e.strips().photos,
            bar_tip(Command::TogglePhotos, e.strips().photos),
            e.panel_shown(),
            bar_tip(Command::TogglePanel, e.panel_shown()),
        )
    };
    assert_eq!(
        tips(&editor),
        (
            true,
            "Esconder fotografias (Alt+3)".to_owned(),
            false,
            "Mostrar painel (Alt+4)".to_owned()
        )
    );
    assert!(editor.run(Command::TogglePanel));
    assert!(editor.run(Command::TogglePhotos));
    assert_eq!(
        tips(&editor),
        (
            false,
            "Mostrar fotografias (Alt+3)".to_owned(),
            true,
            "Esconder painel (Alt+4)".to_owned()
        )
    );
    // O botão das faixas de baixo (spec FX, regra 8) no lugar do de fotos.
    assert_eq!(
        bar_tip(Command::ToggleStrips, true),
        "Esconder as faixas de baixo (Alt+0)"
    );
    assert_eq!(
        bar_tip(Command::CloseAlbum, false),
        "Início: fechar o álbum (Ctrl+W)"
    );
    assert_eq!(bar_tip(Command::Export, false), "Exportar (Ctrl+E)");
    for command in BAR_COMMANDS {
        let key = command.shortcut().unwrap().text();
        assert!(
            bar_tip(command, true).ends_with(&format!("({key})")),
            "{command:?}: o atalho é o do registro"
        );
    }
}

#[test]
fn je_b11_arquivo_ilegivel_volta_ao_padrao_novo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, "isto não é json").unwrap();
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    assert_eq!(editor.strips(), Strips::fresh());
    assert_eq!(
        editor.take_notice().as_deref(),
        Some("Não foi possível ler as preferências da janela; voltaram ao padrão.")
    );
    assert_eq!(
        STRIPS_UNREADABLE,
        "Não foi possível ler as preferências da janela; voltaram ao padrão."
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "isto não é json",
        "o arquivo fica até a mudança"
    );
    assert!(editor.run(Command::ToggleTimeline));
    let saved = json(&file);
    assert_eq!(saved["versao"], STRIPS_VERSION);
    assert_eq!(saved["linha_do_tempo"], true);
    assert_eq!(saved["painel"], false);
}
