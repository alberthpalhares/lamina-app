//! Cenários da spec `tela-inicial` (TI) com álbuns de verdade: abrir, salvar, salvar como, fechar
//! e a miniatura gravada ao salvar.

use std::path::{Path, PathBuf};

use super::common::Base;
use super::ti::{NOW, entry};
use super::tl_common::{H, V, built, spec};
use crate::recents::{RecentsStore, cards, welcome_buttons, write_thumbnail};
use crate::{Command, Editor, Session, Shortcut};

/// Um álbum de `spreads` lâminas (fotos fictícias) gravado em `<dir>/<name>`.
fn album(dir: &Path, name: &str, spreads: usize) -> PathBuf {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    let mut project = built(spec(30.0, 30.0), &photos, spreads);
    project.name = name.trim_end_matches(".lamina").to_owned();
    Session::new_unsaved(project)
        .save_as(&dir.join(name))
        .unwrap()
}

#[test]
fn ti_b1_abrir_um_album_poe_ele_no_topo_da_lista() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path(), "casamento.lamina", 12);
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    assert!(editor.go_to_spread(4));
    let list = editor.recents().list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "casamento");
    let card = &cards(list, &[], NOW)[0];
    assert_eq!(card.counts, "12 lâminas · 48 fotos");
    assert!(editor.close_album());
    assert_eq!(editor.recents().list()[0].current, 4, "a lâmina 5");
}

#[test]
fn ti_b1_abrir_outro_e_sair_guardam_a_lamina_em_que_parou() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (
        album(dir.path(), "a.lamina", 4),
        album(dir.path(), "b.lamina", 4),
    );
    let mut editor = Editor::new();
    editor.open(&a).unwrap();
    assert!(editor.go_to_spread(2));
    editor.open(&b).unwrap();
    assert!(editor.go_to_spread(3));
    // Sair do Lamina: a janela pede para guardar antes de fechar.
    editor.remember_current();
    let current: Vec<(String, usize)> = editor
        .recents()
        .list()
        .iter()
        .map(|r| (r.name.clone(), r.current))
        .collect();
    assert_eq!(current, [("b".to_owned(), 3), ("a".to_owned(), 2)]);
}

#[test]
fn ti_b5_abrir_pelo_cartao_volta_na_lamina_em_que_parou() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path(), "viagem.lamina", 7);
    let mut editor = Editor::new();
    let mut old = entry(&file, "viagem");
    old.current = 8;
    editor.change_recents(|s| s.put(old));
    editor.open(&file).unwrap();
    assert_eq!(
        editor.session().unwrap().current(),
        6,
        "a lâmina 7, a última"
    );
    // Um álbum que não estava na lista abre no começo.
    let other = album(dir.path(), "outro.lamina", 3);
    editor.open(&other).unwrap();
    assert_eq!(editor.session().unwrap().current(), 0);
}

#[test]
fn ti_b8_miniatura_ao_salvar() {
    let base = Base::with_photos(8, (600, 400));
    let file = base.save(base.project(2), "album/casamento.lamina");
    let mut editor = Editor::new();
    editor.set_recents(RecentsStore::load(&base.path("config/recentes.json")));
    editor.open(&file).unwrap();
    editor.save_to(None).unwrap();
    let (project, target) = editor.thumbnail_job().unwrap();
    write_thumbnail(&project, &base.cache(), &target).unwrap();
    let image = image::open(&target).unwrap();
    assert_eq!(image.width().max(image.height()), 320);
    assert_eq!(Some(target.clone()), editor.recents().thumb_path(&file));
    // Sai da lista: a miniatura é apagada.
    editor.change_recents(|s| s.remove(0));
    assert!(!target.exists());
}

#[test]
fn ti_b8_miniatura_que_nao_sai_nao_impede_salvar() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path(), "a.lamina", 2);
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    // Sem pasta de configuração (lista só na memória), não há onde guardar a miniatura.
    assert!(editor.thumbnail_job().is_none());
    assert_eq!(editor.save_to(None).unwrap(), file);
}

#[test]
fn ti_b9_salvar_como_guarda_os_dois() {
    let dir = tempfile::tempdir().unwrap();
    let a = album(dir.path(), "a.lamina", 3);
    let mut editor = Editor::new();
    editor.open(&a).unwrap();
    let b = editor.save_to(Some(&dir.path().join("b.lamina"))).unwrap();
    let paths: Vec<PathBuf> = editor
        .recents()
        .list()
        .iter()
        .map(|r| r.path.clone())
        .collect();
    assert_eq!(paths, [b, a]);
}

#[test]
fn ti_b9_album_novo_entra_na_lista_ao_ser_salvo() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = Editor::new();
    editor.adopt(built(spec(30.0, 30.0), &[H, H, V, H], 1));
    assert!(
        editor.recents().list().is_empty(),
        "sem arquivo, fora da lista"
    );
    let file = editor
        .save_to(Some(&dir.path().join("novo.lamina")))
        .unwrap();
    assert_eq!(editor.recents().list()[0].path, file);
}

#[test]
fn ti_b12_fechar_o_album_volta_a_tela_inicial() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path(), "festa.lamina", 6);
    let mut editor = Editor::new();
    editor.open(&file).unwrap();
    assert!(editor.go_to_spread(3));
    assert!(editor.available(Command::CloseAlbum));
    assert!(editor.close_album());
    assert!(editor.session().is_none());
    assert_eq!(editor.recents().list()[0].current, 3);
    assert!(welcome_buttons(true).bar.contains(&Command::NewAlbum));
    assert!(!editor.available(Command::CloseAlbum));
    assert!(!editor.close_album(), "sem álbum, nada a fechar");
}

#[test]
fn ti_b12_fechar_album_e_limpar_recentes_no_registro() {
    assert_eq!(Command::CloseAlbum.name(), "arquivo.fechar");
    assert_eq!(Command::CloseAlbum.label(), "Fechar álbum");
    assert_eq!(
        Command::from_shortcut(Shortcut::ctrl('w')),
        Some(Command::CloseAlbum)
    );
    assert_eq!(Command::ClearRecents.name(), "arquivo.limpar-recentes");
    assert_eq!(
        Command::ClearRecents.label(),
        "Limpar recentes não fixados…"
    );
    let dir = tempfile::tempdir().unwrap();
    let mut editor = Editor::new();
    assert!(!editor.available(Command::ClearRecents), "lista vazia");
    editor.change_recents(|s| {
        s.put(entry(&dir.path().join("a.lamina"), "a"));
        s.put(entry(&dir.path().join("b.lamina"), "b"));
        s.pin(1).unwrap();
    });
    assert!(editor.available(Command::ClearRecents));
    editor.change_recents(RecentsStore::clear_unpinned);
    let left: Vec<&str> = editor
        .recents()
        .list()
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(left, ["a"], "o fixado fica");
    assert!(!editor.available(Command::ClearRecents));
}
