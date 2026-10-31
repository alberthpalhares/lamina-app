//! Cenários da spec `copia-de-seguranca` (CS): a cópia do álbum com alterações não salvas, para
//! recuperar o trabalho depois de uma queda.

use std::path::{Path, PathBuf};

use super::tl_common::{H, V, built, spec};
use crate::backup::{BACKUP_FOLDER, Backups};
use crate::recents::Clock;
use crate::{Command, Editor, Session};
use lamina_core::Project;

/// Um álbum de 4 lâminas (fotos fictícias) gravado em `<dir>/album.lamina`.
fn album(dir: &Path) -> PathBuf {
    let photos: Vec<(u32, u32)> = (0..16).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 4);
    project.name = "casamento".to_owned();
    Session::new_unsaved(project)
        .save_as(&dir.join("album.lamina"))
        .unwrap()
}

/// O editor com o álbum aberto e as cópias em `<dir>/recuperacao`.
fn editor(dir: &Path, file: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.set_backups(Backups::new(dir.join(BACKUP_FOLDER)));
    editor.open(file).unwrap();
    editor
}

/// Grava a cópia pedida agora, como a janela faz a cada minuto; `true` se havia uma.
fn tick(editor: &mut Editor) -> bool {
    editor
        .backup_step()
        .map(|job| job.write().unwrap())
        .is_some()
}

/// O projeto como está no arquivo do fotógrafo.
fn on_disk(file: &Path) -> Project {
    Session::open(file).unwrap().project().clone()
}

fn copies(dir: &Path) -> usize {
    std::fs::read_dir(dir.join(BACKUP_FOLDER)).map_or(0, |d| d.count())
}

/// Uma queda: a janela some sem fechar direito, e o Lamina abre de novo.
fn reopen(dir: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.set_backups(Backups::new(dir.join(BACKUP_FOLDER)));
    editor
}

#[test]
fn cs_b1_com_alteracoes_grava_a_copia_uma_vez_por_mudanca() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let original = on_disk(&file);
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    assert_eq!(copies(dir.path()), 2, "a cópia e o que a acompanha");
    assert!(!tick(&mut editor), "sem mudança nova, não grava de novo");
    assert!(editor.run(Command::NextLayout));
    assert!(tick(&mut editor));
    assert_eq!(on_disk(&file), original, "o arquivo do fotógrafo não mudou");
}

#[test]
fn cs_b2_salvar_apaga_a_copia() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    editor.save_to(None).unwrap();
    assert!(!tick(&mut editor));
    assert_eq!(copies(dir.path()), 0);
}

#[test]
fn cs_b3_album_sem_alteracoes_nao_tem_copia() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let mut editor = editor(dir.path(), &file);
    assert!(editor.go_to_spread(2), "navegar não é alteração");
    assert!(!tick(&mut editor));
    assert_eq!(copies(dir.path()), 0);
    assert!(reopen(dir.path()).found_backups().is_empty());
}

#[test]
fn cs_b4_depois_de_uma_queda_recupera_na_lamina_em_que_estava() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let original = on_disk(&file);
    let mut editor = editor(dir.path(), &file);
    assert!(editor.go_to_spread(2));
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    drop(editor);

    let mut editor = reopen(dir.path());
    let found = editor.found_backups();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "casamento");
    let clock = Clock {
        now: found[0].when,
        offset: -10_800,
    };
    let question = found[0].question(clock);
    assert!(
        question.starts_with(
            "O Lamina fechou sem salvar \"casamento\". Recuperar as alterações de hoje, "
        ),
        "{question}"
    );
    assert!(editor.recover(&found[0]));
    let session = editor.session().unwrap();
    assert_eq!(session.current(), 2);
    let changed = session.project().spreads[2].clone();
    assert_ne!(changed, original.spreads[2], "com a alteração");
    assert!(session.is_dirty(), "recuperado, ainda não salvo");
    assert_eq!(
        session.file(),
        Some(std::path::absolute(&file).unwrap().as_path())
    );
    // Ctrl+S grava no arquivo do álbum, e a cópia vai embora.
    editor.save_to(None).unwrap();
    assert_eq!(on_disk(&file).spreads[2], changed);
    assert!(!tick(&mut editor));
    assert_eq!(copies(dir.path()), 0);
}

#[test]
fn cs_b5_descartar_apaga_a_copia_e_o_album_fica_como_estava() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let original = on_disk(&file);
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    drop(editor);

    let mut editor = reopen(dir.path());
    let found = editor.found_backups();
    editor.discard_backup(&found[0]);
    assert!(editor.found_backups().is_empty());
    assert_eq!(copies(dir.path()), 0);
    assert_eq!(on_disk(&file), original);
}

#[test]
fn cs_b6_fechar_direito_nao_deixa_copia() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    // "Não salvar" e sair do Lamina.
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    editor.finish_backups();
    assert_eq!(copies(dir.path()), 0);
    // "Não salvar" e fechar o álbum: a cópia vai embora no passo seguinte.
    let mut editor = self::editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    assert!(editor.close_album());
    assert!(!tick(&mut editor));
    assert_eq!(copies(dir.path()), 0);
}

#[test]
fn cs_b7_as_fotos_da_copia_sao_as_mesmas_do_album() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let mut editor = editor(dir.path(), &file);
    let before: Vec<PathBuf> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.path.clone())
        .collect();
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    drop(editor);

    let mut editor = reopen(dir.path());
    let found = editor.found_backups();
    assert!(editor.recover(&found[0]));
    let after: Vec<PathBuf> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.path.clone())
        .collect();
    assert_eq!(after, before);
}

#[test]
fn cs_b8_album_sem_arquivo_tambem_e_recuperado() {
    let dir = tempfile::tempdir().unwrap();
    let photos = [H, H, V, H];
    let mut project = built(spec(30.0, 30.0), &photos, 1);
    project.name = "novo".to_owned();
    let mut editor = Editor::new();
    editor.set_backups(Backups::new(dir.path().join(BACKUP_FOLDER)));
    editor.adopt(project);
    assert!(tick(&mut editor), "o álbum novo já nasce com alterações");
    drop(editor);

    let mut editor = reopen(dir.path());
    let found = editor.found_backups();
    assert_eq!(found[0].file, None);
    assert!(editor.recover(&found[0]));
    let session = editor.session().unwrap();
    assert_eq!(session.title(), "novo");
    assert!(session.file().is_none() && session.is_dirty());
}

#[test]
fn cs_b9_copia_estragada_nao_aparece_e_e_apagada() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join(BACKUP_FOLDER);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("album-0000000000000001.json"), "{ não é json").unwrap();
    std::fs::write(folder.join("album-0000000000000001.lamina"), "{}").unwrap();
    std::fs::write(
        folder.join("album-0000000000000002.json"),
        r#"{"arquivo":null,"nome":"x","quando":1,"lamina":0}"#,
    )
    .unwrap();
    let editor = reopen(dir.path());
    assert!(
        editor.found_backups().is_empty(),
        "uma sem JSON válido, a outra sem a cópia"
    );
    assert_eq!(copies(dir.path()), 0);
}

#[test]
fn cs_b10_copia_pedida_antes_de_salvar_nao_fica_para_tras() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    // A cópia sai para gravar fora da linha da janela, e o fotógrafo salva antes de ela gravar.
    let job = editor.backup_step().unwrap();
    editor.save_to(None).unwrap();
    assert!(editor.backup_step().is_none());
    job.write().unwrap();
    assert_eq!(copies(dir.path()), 0);
    assert!(reopen(dir.path()).found_backups().is_empty());
}

#[test]
fn cs_b11_abrir_o_album_pelo_arquivo_depois_de_uma_queda_ainda_pergunta() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    let changed = editor.session().unwrap().project().spreads.clone();
    drop(editor);

    // O duplo clique: o álbum abre sem alterações, e a janela troca de álbum antes de perguntar.
    let mut editor = reopen(dir.path());
    editor.open(&file).unwrap();
    assert!(!tick(&mut editor));
    assert!(!tick(&mut editor), "e também no minuto seguinte");
    assert_eq!(copies(dir.path()), 2, "a cópia e o que a acompanha");

    let found = editor.found_backups();
    assert_eq!(found.len(), 1);
    assert!(editor.recover(&found[0]));
    assert_eq!(editor.session().unwrap().project().spreads, changed);
    assert!(!tick(&mut editor), "a cópia recuperada já está no disco");
    // Salvar apaga a cópia recuperada (regra 2).
    editor.save_to(None).unwrap();
    assert!(!tick(&mut editor));
    assert_eq!(copies(dir.path()), 0);
}

#[test]
fn cs_b12_fechar_sem_responder_nao_perde_a_copia() {
    let dir = tempfile::tempdir().unwrap();
    let file = album(dir.path());
    let mut editor = editor(dir.path(), &file);
    assert!(editor.run(Command::Mirror));
    assert!(tick(&mut editor));
    drop(editor);

    let mut editor = reopen(dir.path());
    editor.open(&file).unwrap();
    assert_eq!(editor.found_backups().len(), 1, "a pergunta na tela");
    editor.finish_backups();
    assert_eq!(copies(dir.path()), 2);
    assert_eq!(reopen(dir.path()).found_backups().len(), 1);
}
