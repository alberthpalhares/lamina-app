//! Cenários da spec `nome-do-album` (NA): trocar o nome do álbum, desfazer, limpar o nome,
//! recusar caractere que não serve de arquivo e levar o nome ao arquivo e aos recentes.

use lamina_core::CoreError;

use super::common::Base;
use crate::Human;
use crate::recents::RecentsStore;
use crate::{Editor, MAX_ALBUM_NAME};

/// Editor com um álbum de 2 lâminas salvo em `album/casamento.lamina`, com o nome `name`.
fn saved(base: &Base, name: &str) -> Editor {
    let mut project = base.project(2);
    project.name = name.to_owned();
    let file = base.save(project, "album/casamento.lamina");
    let mut editor = Editor::new();
    editor.set_recents(RecentsStore::load(&base.path("config/recentes.json")));
    editor.open(&file).unwrap();
    editor
}

fn name(editor: &Editor) -> &str {
    &editor.session().unwrap().project().name
}

fn steps(editor: &Editor) -> usize {
    editor.session().unwrap().history_list().steps.len()
}

#[test]
fn na_b1_renomear_o_album() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento Ana e Leo");
    assert!(!editor.session().unwrap().is_dirty());
    assert!(editor.rename_album("Ana e Leo, 2026").unwrap());
    assert_eq!(name(&editor), "Ana e Leo, 2026");
    let s = editor.session().unwrap();
    assert!(s.is_dirty());
    let list = s.history_list();
    assert_eq!(list.steps.last().unwrap().label, "Renomear álbum · álbum");
}

#[test]
fn na_b2_desfazer_o_nome() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento");
    assert!(editor.rename_album("Ana e Leo").unwrap());
    assert!(editor.session_mut().unwrap().undo().is_some());
    assert_eq!(name(&editor), "Casamento");
}

#[test]
fn na_b3_nome_vazio() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento");
    let before = steps(&editor);
    assert!(!editor.rename_album("   ").unwrap());
    assert!(!editor.rename_album("").unwrap());
    assert_eq!(name(&editor), "Casamento");
    assert_eq!(steps(&editor), before);
}

#[test]
fn na_b4_espacos_quebras_e_nome_longo() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento");
    assert!(editor.rename_album("  Ana\ne Leo  ").unwrap());
    assert_eq!(name(&editor), "Ana e Leo");
    let long = "a".repeat(150);
    assert!(editor.rename_album(&long).unwrap());
    assert_eq!(name(&editor).chars().count(), MAX_ALBUM_NAME);
    assert_eq!(name(&editor), "a".repeat(120));
}

#[test]
fn na_b5_o_mesmo_nome() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento");
    let before = steps(&editor);
    assert!(!editor.rename_album("Casamento").unwrap());
    assert!(!editor.rename_album("  Casamento ").unwrap());
    assert!(!editor.session().unwrap().is_dirty());
    assert_eq!(steps(&editor), before);
}

#[test]
fn na_b6_o_nome_vai_para_o_arquivo_e_para_os_recentes() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento");
    assert!(editor.rename_album("Ana e Leo").unwrap());
    let file = editor.save_to(None).unwrap();
    assert!(
        file.ends_with("album/casamento.lamina"),
        "o arquivo não muda de nome"
    );
    let mut again = Editor::new();
    again.open(&file).unwrap();
    assert_eq!(name(&again), "Ana e Leo");
    let names: Vec<&str> = editor
        .recents()
        .list()
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(names, ["Ana e Leo"]);
}

#[test]
fn na_b7_nome_com_caractere_invalido() {
    let base = Base::with_photos(4, (600, 400));
    let mut editor = saved(&base, "Casamento");
    let before = steps(&editor);
    // O `:` do bug do PDF, a barra, o `?`, o ponto final e o nome reservado do Windows.
    for bad in ["Ana: Leo", "Ana/Leo", "Ana?", "CON", "nome."] {
        assert!(
            matches!(editor.rename_album(bad), Err(CoreError::InvalidName(_))),
            "{bad}"
        );
    }
    let err = editor.rename_album("Ana: Leo").unwrap_err();
    assert_eq!(
        err.human().title,
        "O nome \"Ana: Leo\" tem caracteres que não servem para nome de arquivo."
    );
    assert_eq!(name(&editor), "Casamento");
    assert_eq!(steps(&editor), before);
    assert!(!editor.session().unwrap().is_dirty());
}
