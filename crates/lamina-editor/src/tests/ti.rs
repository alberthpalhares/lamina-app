//! Cenários da spec `tela-inicial` (TI) que rodam só com a lista: entrar, sair, fixar, arquivo que
//! sumiu, lista ilegível, menu, busca e a velocidade dos cartões. Os que abrem e salvam álbuns de
//! verdade moram em `ti_files.rs`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};

use crate::recents::{
    Clock, MAX_PINNED, MAX_RECENTS, PINNED_FULL, RECENTS_UNREADABLE, Recent, RecentsStore, cards,
    counts_text, missing_files, recent_menu, search, show_search, welcome_buttons, when_text,
};
use crate::{Command, Editor};

/// 19 de outubro de 2026, 14:32 em Brasília (UTC−3).
pub const NOW: Clock = Clock {
    now: 1_792_431_120,
    offset: -3 * 3600,
};

pub fn entry(path: &Path, name: &str) -> Recent {
    Recent {
        path: path.to_path_buf(),
        name: name.to_owned(),
        opened_at: NOW.now,
        spreads: 12,
        photos: 180,
        current: 0,
        pinned: false,
        fold: true,
        on_cover: false,
        cover_image: false,
    }
}

/// `n` álbuns `a00.lamina`… em `dir`, o último aberto por último (no topo).
fn filled(dir: &Path, n: usize) -> RecentsStore {
    let mut store = RecentsStore::default();
    for i in 0..n {
        store.put(entry(
            &dir.join(format!("a{i:02}.lamina")),
            &format!("a{i:02}"),
        ));
    }
    store
}

fn names(store: &RecentsStore) -> Vec<String> {
    store.list().iter().map(|r| r.name.clone()).collect()
}

#[test]
fn ti_b2_o_mesmo_arquivo_aparece_uma_vez_so() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RecentsStore::default();
    store.put(entry(&dir.path().join("Fotos").join("Ana.lamina"), "Ana"));
    store.put(entry(&dir.path().join("Fotos").join("Bia.lamina"), "Bia"));
    store.put(entry(&dir.path().join("fotos").join("ANA.lamina"), "Ana"));
    if cfg!(windows) {
        // No Windows, maiúsculas não mudam o arquivo: um cartão só, agora no topo.
        assert_eq!(names(&store), ["Ana", "Bia"]);
    } else {
        assert_eq!(names(&store), ["Ana", "Bia", "Ana"]);
    }
    // O caminho é comparado já normalizado: `.` e `..` não criam outro cartão.
    let same = dir.path().join("Fotos").join(".").join("Bia.lamina");
    store.put(entry(&same, "Bia"));
    assert_eq!(store.list()[0].name, "Bia");
    assert_eq!(store.list().iter().filter(|r| r.name == "Bia").count(), 1);
}

#[test]
fn ti_b3_ate_20_saindo_o_mais_antigo_nao_fixado() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = filled(dir.path(), MAX_RECENTS);
    let oldest = store.list().len() - 1;
    assert_eq!(store.list()[oldest].name, "a00");
    store.pin(oldest).unwrap();
    store.put(entry(&dir.path().join("a20.lamina"), "a20"));
    let list = names(&store);
    assert_eq!(list.len(), MAX_RECENTS);
    assert_eq!(list[0], "a00", "o fixado continua, no topo");
    assert_eq!(list[1], "a20");
    assert!(!list.contains(&"a01".to_owned()), "{list:?}");
}

#[test]
fn ti_b4_fixados_primeiro_ate_8() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = filled(dir.path(), 3);
    assert_eq!(names(&store), ["a02", "a01", "a00"]);
    store.pin(1).unwrap();
    let shown = cards(store.list(), &[], NOW);
    let order: Vec<(&str, bool)> = shown.iter().map(|c| (c.name.as_str(), c.pinned)).collect();
    assert_eq!(order, [("a01", true), ("a02", false), ("a00", false)]);
    // Abrir outro não tira o fixado do topo.
    store.put(entry(&dir.path().join("novo.lamina"), "novo"));
    assert_eq!(names(&store)[..2], ["a01", "novo"]);

    let mut full = filled(dir.path(), MAX_PINNED + 1);
    for _ in 0..MAX_PINNED {
        let first_loose = full.list().iter().position(|r| !r.pinned).unwrap();
        full.pin(first_loose).unwrap();
    }
    let ninth = full.list().iter().position(|r| !r.pinned).unwrap();
    assert_eq!(full.pin(ninth), Err(PINNED_FULL.to_owned()));
    assert!(!full.list()[ninth].pinned);
    // Desafixar um abre a vaga.
    full.unpin(0);
    let ninth = full
        .list()
        .iter()
        .position(|r| !r.pinned && r.name == "a00")
        .unwrap();
    assert_eq!(full.pin(ninth), Ok(()));
}

#[test]
fn ti_b6_arquivo_que_sumiu() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    let mut store = RecentsStore::load(&config.join("recentes.json"));
    let gone = dir.path().join("sumiu.lamina");
    let mut old = entry(&gone, "Formatura");
    old.current = 4;
    old.pinned = true;
    store.put(old);
    let thumb = store.thumb_path(&gone).unwrap();
    std::fs::create_dir_all(thumb.parent().unwrap()).unwrap();
    std::fs::write(&thumb, b"png").unwrap();

    let missing = missing_files(std::slice::from_ref(&gone));
    assert_eq!(missing, std::slice::from_ref(&gone));
    let card = &cards(store.list(), &missing, NOW)[0];
    assert!(card.missing);
    assert_eq!(card.name, "Formatura");

    // "Procurar…": o arquivo movido troca o caminho e mantém o resto.
    let moved = dir.path().join("movido").join("formatura.lamina");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::write(&moved, b"{}").unwrap();
    store.relocate(0, &moved);
    let now = &store.list()[0];
    assert_eq!(now.path, std::path::absolute(&moved).unwrap());
    assert_eq!(
        (now.name.as_str(), now.current, now.pinned),
        ("Formatura", 4, true)
    );
    assert!(missing_files(std::slice::from_ref(&now.path)).is_empty());
    let moved_thumb = store.thumb_path(&moved).unwrap();
    assert!(moved_thumb.is_file() && !thumb.exists());

    // "Tirar da lista": some o cartão e a miniatura.
    store.remove(0);
    assert!(store.list().is_empty());
    assert!(!moved_thumb.exists());
}

#[test]
fn ti_b6_procurar_um_arquivo_que_ja_esta_na_lista_junta_os_dois() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = filled(dir.path(), 3);
    let target = store.list()[0].path.clone();
    // O cartão que procurou fica no lugar dele, com o caminho escolhido; o outro sai.
    store.relocate(2, &target);
    assert_eq!(names(&store), ["a01", "a00"]);
    assert_eq!(store.list()[1].path, target);
}

#[test]
fn ti_b7_lista_ilegivel_comeca_vazia_com_aviso() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("recentes.json");
    std::fs::write(&file, "isto não é json").unwrap();
    let store = RecentsStore::load(&file);
    assert!(store.list().is_empty());
    assert_eq!(store.warning.as_deref(), Some(RECENTS_UNREADABLE));
    assert_eq!(
        RECENTS_UNREADABLE,
        "Não foi possível ler a lista de álbuns recentes; ela começa vazia."
    );
    // A janela abre e mostra o aviso uma vez; o arquivo fica como estava.
    let mut editor = Editor::new();
    editor.set_recents(store);
    assert_eq!(editor.take_notice().as_deref(), Some(RECENTS_UNREADABLE));
    assert!(editor.available(Command::Open));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "isto não é json");
    // Só a próxima mudança regrava.
    editor.change_recents(|s| s.put(entry(&dir.path().join("a.lamina"), "a")));
    let again = RecentsStore::load(&file);
    assert_eq!(again.warning, None);
    assert_eq!(names(&again), ["a"]);
}

#[test]
fn ti_b7_nao_gravar_a_lista_vale_nesta_sessao_com_aviso() {
    let dir = tempfile::tempdir().unwrap();
    // Uma pasta no lugar do arquivo: a gravação falha.
    let file = dir.path().join("recentes.json");
    std::fs::create_dir_all(&file).unwrap();
    let mut editor = Editor::new();
    editor.set_recents(RecentsStore::load(&file));
    editor.take_notice();
    editor.change_recents(|s| s.put(entry(&dir.path().join("a.lamina"), "a")));
    let notice = editor.take_notice().unwrap();
    assert!(
        notice.starts_with("Não foi possível guardar a lista de álbuns recentes: "),
        "{notice}"
    );
    assert_eq!(names(editor.recents()), ["a"]);
}

#[test]
fn ti_b10_menu_abrir_recente() {
    let dir = tempfile::tempdir().unwrap();
    let store = filled(dir.path(), 15);
    let gone = store.list()[3].path.clone();
    let shown = cards(store.list(), &[gone], NOW);
    let menu = recent_menu(&shown);
    assert_eq!(menu.len(), 10);
    for (item, card) in menu.iter().zip(&shown) {
        assert_eq!(
            (item.index, item.label.as_str()),
            (card.index, card.name.as_str())
        );
        assert_eq!(item.enabled, card.index != 3, "{item:?}");
    }
}

#[test]
fn ti_b11_busca_sem_acento_nem_maiuscula() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RecentsStore::default();
    store.put(entry(&dir.path().join("f.lamina"), "Formatura São João"));
    store.put(entry(
        &dir.path().join("Clientes").join("c.lamina"),
        "Casamento Ana",
    ));
    let shown = cards(store.list(), &[], NOW);
    let found = |query: &str| -> Vec<String> {
        search(&shown, query)
            .iter()
            .map(|c| c.name.clone())
            .collect()
    };
    assert_eq!(found("sao joao"), ["Formatura São João"]);
    assert_eq!(found("SÃO JOÃO"), ["Formatura São João"]);
    assert_eq!(found("clientes"), ["Casamento Ana"], "pela pasta");
    assert_eq!(found("").len(), 2);
    assert!(!show_search(8) && show_search(9));
}

#[test]
fn ti_b13_montar_os_cartoes_e_rapido() {
    let dir = tempfile::tempdir().unwrap();
    let store = filled(dir.path(), MAX_RECENTS);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let started = Instant::now();
        let shown = cards(store.list(), &[], NOW);
        let menu = recent_menu(&shown);
        best = best.min(started.elapsed());
        assert_eq!((shown.len(), menu.len()), (MAX_RECENTS, 10));
        if best < budget(20) {
            break;
        }
    }
    assert!(best < budget(20), "montar os cartões levou {best:?}");
}

#[test]
fn ti_b14_sem_recentes_a_boas_vindas_da_pc() {
    // Corrigido pela spec DI: sem recentes, o novo álbum é o principal, com o exemplo ao lado.
    assert_eq!(
        welcome_buttons(false).body,
        [Command::NewAlbum, Command::OpenExample]
    );
    assert_eq!(
        welcome_buttons(true).bar,
        [Command::OpenExample, Command::Open, Command::NewAlbum]
    );
    assert!(cards(&[], &[], NOW).is_empty());
    assert!(Editor::new().available(Command::OpenExample));
}

#[test]
fn quando_foi_aberto_e_o_que_tem_em_palavras() {
    let at = |h: i64, m: i64| NOW.now - h * 3600 - m * 60;
    assert_eq!(when_text(NOW.now, NOW), "hoje, 14:32");
    assert_eq!(when_text(at(14, 32), NOW), "hoje, 00:00");
    assert_eq!(when_text(at(14, 33), NOW), "ontem");
    assert_eq!(when_text(NOW.now - 7 * 86_400, NOW), "12 de out.");
    assert_eq!(when_text(NOW.now - 365 * 86_400, NOW), "19 de out. de 2025");
    assert_eq!(counts_text(18, 240), "18 lâminas · 240 fotos");
    assert_eq!(counts_text(1, 1), "1 lâmina · 1 foto");
}

#[test]
fn lista_grava_e_le_com_chaves_em_portugues() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("recentes.json");
    let mut store = RecentsStore::load(&file);
    store.put(entry(&dir.path().join("a.lamina"), "Ana"));
    store.save().unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    for key in [
        "\"versao\": 1",
        "\"albuns\"",
        "\"caminho\"",
        "\"nome\": \"Ana\"",
        "\"aberto_em\"",
        "\"laminas\": 12",
        "\"fotos\": 180",
        "\"lamina_atual\": 0",
        "\"fixado\": false",
    ] {
        assert!(text.contains(key), "{key} em {text}");
    }
    assert_eq!(RecentsStore::load(&file).list(), store.list());
    let path: PathBuf = store.list()[0].path.clone();
    assert!(path.is_absolute());
}
