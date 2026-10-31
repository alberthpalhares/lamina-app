//! Cenários da spec `design-tela-inicial` (DI): o que a tela inicial destaca, o álbum em andamento,
//! a grade e as frases. Sem janela: a tela só desenha o que sai daqui.

use std::path::Path;
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};

use super::common::Base;
use super::ti::{NOW, entry};
use crate::recents::{
    RESUME_SIDE, Recent, RecentsStore, THUMB_SIDE, card_label, card_tip, cards, grid_cards,
    grid_columns, no_results, read_thumbnail, resume, resume_height, welcome_buttons,
    write_big_thumbnail, write_thumbnail,
};
use crate::{Command, Editor};

/// Um álbum aberto `ago` segundos antes de agora.
fn opened(dir: &Path, name: &str, ago: i64) -> Recent {
    Recent {
        opened_at: NOW.now - ago,
        ..entry(&dir.join(format!("{name}.lamina")), name)
    }
}

fn single_page_start(spec: &lamina_core::AlbumSpec) -> lamina_core::AlbumSpec {
    let mut json = serde_json::to_value(spec).unwrap();
    json["edges"]["start"] = serde_json::json!("single_page");
    serde_json::from_value(json).unwrap()
}

#[test]
fn di_b1_primeira_vez_o_novo_album_e_o_principal() {
    let buttons = welcome_buttons(false);
    assert_eq!(buttons.body, [Command::NewAlbum, Command::OpenExample]);
    assert_eq!(buttons.bar, [Command::Open]);
}

#[test]
fn di_b2_com_recentes_continuar_e_o_principal() {
    let dir = tempfile::tempdir().unwrap();
    let mut album = opened(dir.path(), "Ana", 0);
    album.spreads = 6;
    album.current = 2;
    let list = [album];
    let shown = cards(&list, &[], NOW);
    let current = resume(&list, &shown, "").unwrap();
    assert_eq!(current.button, "Continuar na lâmina 3");
    assert_eq!(current.sentence, "Você parou na lâmina 3 de 6");
    assert_eq!(current.meta, "6 lâminas · 180 fotos · aberto hoje, 14:32");
    let buttons = welcome_buttons(true);
    assert!(buttons.body.is_empty());
    assert_eq!(
        buttons.bar,
        [Command::OpenExample, Command::Open, Command::NewAlbum]
    );
    // Com menos lâminas hoje, vale a última (TI, regra 3).
    let mut shrunk = list[0].clone();
    shrunk.current = 9;
    let shrunk = [shrunk];
    let shown = cards(&shrunk, &[], NOW);
    assert_eq!(
        resume(&shrunk, &shown, "").unwrap().button,
        "Continuar na lâmina 6"
    );
}

#[test]
fn di_b3_uma_lista_so_com_os_fixados_primeiro() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RecentsStore::default();
    store.put(opened(dir.path(), "antigo", 300));
    store.put(opened(dir.path(), "fixado", 200));
    store.put(opened(dir.path(), "meio", 100));
    store.put(opened(dir.path(), "ultimo", 0));
    store.toggle_pin(2).unwrap();
    let shown = cards(store.list(), &[], NOW);
    let current = resume(store.list(), &shown, "").unwrap();
    let names: Vec<&str> = grid_cards(&shown, Some(&current), "")
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, ["fixado", "meio", "antigo"]);
}

#[test]
fn di_b4_quem_e_o_album_em_andamento() {
    let dir = tempfile::tempdir().unwrap();
    let gone = opened(dir.path(), "sumiu", 0);
    let mut pinned = opened(dir.path(), "fixado", 100);
    pinned.pinned = true;
    let other = opened(dir.path(), "outro", 200);
    // Na ordem da tela: o fixado primeiro.
    let list = [pinned, gone.clone(), other];
    let shown = cards(&list, std::slice::from_ref(&gone.path), NOW);
    let current = resume(&list, &shown, "").unwrap();
    assert_eq!(shown[current.index].name, "fixado");
    let grid = grid_cards(&shown, Some(&current), "");
    let names: Vec<&str> = grid.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["sumiu", "outro"]);
    assert!(grid[0].missing);
}

#[test]
fn di_b5_album_em_branco_e_todos_sumidos() {
    let dir = tempfile::tempdir().unwrap();
    let mut blank = opened(dir.path(), "branco", 0);
    blank.spreads = 0;
    let list = [blank.clone()];
    let shown = cards(&list, &[], NOW);
    let current = resume(&list, &shown, "").unwrap();
    assert_eq!(current.button, "Abrir");
    assert_eq!(current.sentence, "");
    // Todos sumiram: nenhum em andamento, e o principal volta a ser o novo álbum.
    let shown = cards(&list, &[blank.path], NOW);
    assert!(resume(&list, &shown, "").is_none());
}

#[test]
fn di_b6_a_busca_esconde_o_album_em_andamento() {
    let dir = tempfile::tempdir().unwrap();
    let list = [
        opened(dir.path(), "Ana", 0),
        opened(dir.path(), "Bia", 100),
        opened(dir.path(), "Caio", 200),
    ];
    let shown = cards(&list, &[], NOW);
    assert!(resume(&list, &shown, "  ana ").is_none());
    let found = grid_cards(&shown, None, "  ana ");
    let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Ana"]);
    // Com a busca em branco, é como sem busca.
    assert!(resume(&list, &shown, "   ").is_some());
}

#[test]
fn di_b7_colunas_da_grade() {
    assert_eq!(grid_columns(1318.0), 6);
    assert_eq!(grid_columns(1456.0), 7);
    // Nunca menos que uma, nem na janela mais estreita.
    assert_eq!(grid_columns(100.0), 1);
}

#[test]
fn di_b8_altura_da_imagem() {
    assert_eq!(resume_height(560.0), 200.0);
    assert_eq!(resume_height(668.0), 272.0);
    assert_eq!(resume_height(980.0), 400.0);
}

#[test]
fn di_b9_uma_miniatura_grande_so() {
    let base = Base::with_photos(4, (600, 400));
    let first = base.save(base.project(1), "album/a.lamina");
    let second = base.save(base.project(1), "album/b.lamina");
    let mut editor = Editor::new();
    editor.set_recents(RecentsStore::load(&base.path("config/recentes.json")));
    for file in [&first, &second] {
        editor.open(file).unwrap();
        let (project, small) = editor.thumbnail_job().unwrap();
        write_thumbnail(&project, &base.cache(), &small).unwrap();
        let big = editor.recents().big_thumb_path(file).unwrap();
        write_big_thumbnail(&project, &base.cache(), &big).unwrap();
    }
    let path = |file: &Path| editor.recents().big_thumb_path(file).unwrap();
    assert!(!path(&first).exists(), "a do álbum anterior sai");
    let last = path(&second);
    let big = read_thumbnail(&last).unwrap();
    assert_eq!(big.width.max(big.height), RESUME_SIDE);
    for file in [&first, &second] {
        let small = read_thumbnail(&editor.recents().thumb_path(file).unwrap()).unwrap();
        assert_eq!(small.width.max(small.height), THUMB_SIDE);
    }
    // Saindo da lista, a grande sai junto.
    let index = editor
        .recents()
        .list()
        .iter()
        .position(|r| r.path.ends_with("b.lamina"))
        .unwrap();
    editor.change_recents(|s| s.remove(index));
    assert!(!last.exists());
}

#[test]
fn di_b10_busca_sem_resultado() {
    assert_eq!(no_results(" xyz "), "Nenhum álbum com “xyz”.");
}

#[test]
fn di_b11_a_dica_do_cartao_diz_o_nome_e_a_pasta() {
    let dir = tempfile::tempdir().unwrap();
    let long =
        "Casamento de Ana Beatriz Figueiredo e João Pedro Albuquerque, versão final da gráfica";
    let list = [opened(dir.path(), long, 0)];
    let shown = cards(&list, &[], NOW);
    let folder = dir.path().display().to_string();
    assert_eq!(card_tip(&shown[0]), format!("{long} · {folder}"));
    assert_eq!(
        card_label(&shown[0]),
        format!("{long}, hoje, 14:32, 12 lâminas · 180 fotos")
    );
}

#[test]
fn di_b14_atalhos_falam_em_sequencia() {
    let groups = crate::shortcuts::sheet(crate::Platform::Windows);
    let gestures: Vec<String> = groups
        .iter()
        .flat_map(|g| g.rows.iter())
        .filter(|r| r.command.is_none())
        .flat_map(|r| r.keys.clone())
        .collect();
    assert!(!gestures.is_empty());
    assert!(gestures.iter().all(|g| !g.contains("linha do tempo")));
    assert!(gestures.iter().any(|g| g.contains("corte na Sequência")));
}

#[test]
fn di_b15_a_imagem_do_album_sem_capa_e_a_primeira_lamina_com_a_dobra() {
    let base = Base::with_photos(4, (600, 400));
    let file = base.save(base.project(2), "album/a.lamina");
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    assert!(editor.recents().find(&file).unwrap().fold);
    // A primeira lâmina com uma página só não tem dobra (DJ, regra 2).
    let mut project = base.project(2);
    project.spec = single_page_start(&project.spec);
    let single = base.save(project, "album/b.lamina");
    editor.open(&single).unwrap();
    assert!(!editor.recents().find(&single).unwrap().fold);
}

#[test]
fn di_a_tela_com_o_album_em_andamento_rapido() {
    let dir = tempfile::tempdir().unwrap();
    let list: Vec<Recent> = (0..20)
        .map(|i| opened(dir.path(), &format!("a{i:02}"), i))
        .collect();
    let goal = budget(20);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let started = Instant::now();
        let shown = cards(&list, &[], NOW);
        let current = resume(&list, &shown, "");
        let grid = grid_cards(&shown, current.as_ref(), "");
        best = best.min(started.elapsed());
        assert_eq!(grid.len(), 19);
        if best < goal {
            break;
        }
    }
    assert!(best < goal, "{best:?}");
}
