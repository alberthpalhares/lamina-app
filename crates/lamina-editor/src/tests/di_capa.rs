//! Cenários da spec `design-tela-inicial` (DI) que dependem da capa: onde o fotógrafo parou (regra
//! 13) e a imagem do álbum (regra 16).

use std::path::PathBuf;

use lamina_core::{CoverMeasures, Project};

use super::common::Base;
use super::ti::{NOW, entry};
use crate::Editor;
use crate::recents::{RecentsStore, cards, resume, thumbnail_image};

fn list_of(base: &Base) -> RecentsStore {
    RecentsStore::load(&base.path("config/recentes.json"))
}

/// Um editor com a lista em disco, com `project` salvo e aberto.
fn opened(base: &Base, project: Project) -> (Editor, PathBuf) {
    let file = base.save(project, "album/capa.lamina");
    let mut editor = Editor::new();
    editor.set_recents(list_of(base));
    editor.open(&file).unwrap();
    (editor, file)
}

/// Cria a capa, com a primeira foto do álbum ou vazia, e fica nela.
fn create_cover(editor: &mut Editor, with_photo: bool) {
    let album = editor.session().unwrap().album();
    let measures = CoverMeasures::defaults(&album.spec);
    let photos = [album.photos[0].id];
    let photos = if with_photo { &photos[..] } else { &[] };
    assert!(editor.create_cover(measures, None, photos));
    assert!(editor.on_cover());
}

#[test]
fn di_b2_fechado_na_capa_a_frase_e_o_botao_falam_da_capa() {
    let mut album = entry(&PathBuf::from("capa.lamina"), "Ana");
    album.spreads = 6;
    album.current = 2;
    album.on_cover = true;
    let list = [album];
    let shown = cards(&list, &[], NOW);
    let current = resume(&list, &shown, "").unwrap();
    assert_eq!(current.button, "Continuar na capa");
    assert_eq!(current.sentence, "Você parou na capa");
}

#[test]
fn di_b2_abrir_volta_para_a_capa_e_sair_dela_para_a_lamina() {
    let base = Base::with_photos(4, (600, 400));
    let (mut editor, file) = opened(&base, base.project(3));
    assert!(editor.go_to_spread(1));
    create_cover(&mut editor, true);
    editor.save_to(None).unwrap();
    assert!(editor.close_album());
    let saved = editor.recents().find(&file).unwrap();
    assert!(saved.on_cover);
    assert_eq!(saved.current, 1, "a lâmina continua guardada");

    // Em outra execução do programa, com a lista lida do disco.
    let mut editor = Editor::new();
    editor.set_recents(list_of(&base));
    editor.open(&file).unwrap();
    assert!(editor.on_cover(), "abre na capa");
    assert_eq!(editor.session().unwrap().last_spread(), 1);

    assert!(editor.go_to_spread(2));
    assert!(editor.close_album());
    assert!(!editor.recents().find(&file).unwrap().on_cover);
    editor.open(&file).unwrap();
    assert!(!editor.on_cover());
    assert_eq!(editor.session().unwrap().last_spread(), 2);
}

#[test]
fn di_b2_a_capa_que_nao_foi_salva_nao_fica_na_lista() {
    let base = Base::with_photos(4, (600, 400));
    let (mut editor, file) = opened(&base, base.project(3));
    create_cover(&mut editor, true);
    assert!(editor.close_album(), "fechar sem salvar");
    assert!(!editor.recents().find(&file).unwrap().on_cover);
}

#[test]
fn di_b2_sem_capa_no_arquivo_abre_na_lamina_guardada() {
    let base = Base::with_photos(4, (600, 400));
    let (mut editor, file) = opened(&base, base.project(3));
    assert!(editor.close_album());
    let mut promised = editor.recents().find(&file).unwrap().clone();
    promised.current = 1;
    promised.on_cover = true;
    editor.change_recents(|s| s.put(promised));
    editor.open(&file).unwrap();
    assert!(!editor.on_cover());
    assert_eq!(editor.session().unwrap().last_spread(), 1);
    assert!(!editor.recents().find(&file).unwrap().on_cover);
}

#[test]
fn di_b2_a_lista_de_antes_da_capa_nao_para_na_capa() {
    let base = Base::with_photos(1, (600, 400));
    let path = base.path("recentes.json");
    let old = serde_json::json!({ "versao": 1, "albuns": [{
        "caminho": base.path("a.lamina"), "nome": "a", "aberto_em": 0, "laminas": 3, "fotos": 9,
        "lamina_atual": 1, "fixado": false
    }] });
    std::fs::write(&path, old.to_string()).unwrap();
    let store = RecentsStore::load(&path);
    assert!(store.warning.is_none());
    assert!(!store.list()[0].on_cover);
    assert!(!store.list()[0].cover_image);
}

#[test]
fn di_b15_com_capa_a_imagem_e_a_capa_aberta_sem_a_sombra() {
    let base = Base::with_photos(4, (600, 400));
    let (mut editor, file) = opened(&base, base.project(3));
    assert!(editor.recents().find(&file).unwrap().fold);
    create_cover(&mut editor, true);
    editor.save_to(None).unwrap();

    let saved = editor.recents().find(&file).unwrap();
    assert!(saved.cover_image);
    assert!(!saved.fold, "a capa não dobra ali como a lâmina");
    let (project, _) = editor.thumbnail_job().unwrap();
    let thumb = thumbnail_image(&project, &base.cache()).unwrap().unwrap();
    let canvas = project.cover_canvas().unwrap().unwrap();
    let cover = thumbnail_image(&canvas, &base.cache()).unwrap().unwrap();
    assert_eq!(thumb, cover, "a capa inteira: verso, lombada e frente");
    let mut no_cover = (*project).clone();
    no_cover.cover = None;
    let first = thumbnail_image(&no_cover, &base.cache()).unwrap().unwrap();
    assert_ne!(thumb, first);
}

#[test]
fn di_b15_a_capa_ainda_vazia_nao_vira_a_imagem() {
    let base = Base::with_photos(4, (600, 400));
    let (mut editor, file) = opened(&base, base.project(3));
    create_cover(&mut editor, false);
    editor.save_to(None).unwrap();

    let saved = editor.recents().find(&file).unwrap();
    assert!(!saved.cover_image);
    assert!(saved.fold);
    let (project, _) = editor.thumbnail_job().unwrap();
    let mut no_cover = (*project).clone();
    no_cover.cover = None;
    assert_eq!(
        thumbnail_image(&project, &base.cache()),
        thumbnail_image(&no_cover, &base.cache())
    );
}

#[test]
fn di_b15_album_sem_laminas_com_capa_mostra_a_capa() {
    let base = Base::with_photos(4, (600, 400));
    let (mut editor, _) = opened(&base, base.project(3));
    create_cover(&mut editor, true);
    let (project, _) = editor.thumbnail_job().unwrap();
    let mut only_cover = (*project).clone();
    only_cover.spreads.clear();
    assert!(matches!(
        thumbnail_image(&only_cover, &base.cache()),
        Ok(Some(_))
    ));

    let mut album = entry(&PathBuf::from("capa.lamina"), "Ana");
    album.spreads = 0;
    let blank = cards(std::slice::from_ref(&album), &[], NOW);
    assert!(blank[0].blank, "sem lâminas e sem capa: o desenho da BR");
    assert_eq!(
        resume(&[album.clone()], &blank, "").unwrap().button,
        "Abrir"
    );
    album.cover_image = true;
    assert!(!cards(&[album], &[], NOW)[0].blank);
}
