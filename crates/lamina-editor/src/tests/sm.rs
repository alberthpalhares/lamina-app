//! Cenários da spec `salvar-modelo-organizar` (SM): "Salvar como modelo…" no menu de contexto do
//! quadro, no modo organizar, e o modelo que guarda a lâmina ajustada à mão.

use lamina_core::{PageSide, Project, RelRect};

use super::md::{HHV, album, with_models};
use super::tl_common::spec;
use crate::editor::ContextEntry;
use crate::{Command, ModelsStore};

/// O item `Command` do menu, com o rótulo e se vale.
fn item(menu: &[ContextEntry], command: Command) -> Option<(String, bool)> {
    menu.iter().find_map(|e| match e {
        ContextEntry::Item {
            command: c,
            label,
            enabled,
            ..
        } if *c == command => Some((label.clone(), *enabled)),
        _ => None,
    })
}

fn rects(editor: &crate::Editor) -> Vec<RelRect> {
    let s = editor.session().unwrap();
    s.project().spreads[s.current()]
        .frames
        .iter()
        .map(|f| f.rect)
        .collect()
}

#[test]
fn sm_b1_o_item_no_menu_do_quadro_no_modo_organizar() {
    let (mut editor, _dir) = with_models(album(&[HHV]), 0);
    assert!(editor.run(Command::Organize));
    editor.organize_click(0, false, false);
    let menu = editor.context_menu(Some(0));
    let (label, enabled) = item(&menu, Command::SaveModel).expect("o item está no menu do quadro");
    assert_eq!(label, "Salvar como modelo…");
    assert!(enabled, "a lâmina tem quadros de foto: o item acende");
    assert_eq!(enabled, editor.available(Command::SaveModel));
}

#[test]
fn sm_b2_fora_do_modo_o_menu_do_quadro_nao_tem_o_item() {
    let (mut editor, _dir) = with_models(album(&[HHV]), 0);
    let frame = editor.context_menu(Some(0));
    assert!(
        item(&frame, Command::SaveModel).is_none(),
        "fora do modo organizar, o menu do quadro não tem o item"
    );
    // O menu do fundo continua com ele nos dois modos (EG, regra 13).
    let background = editor.context_menu(None);
    assert!(item(&background, Command::SaveModel).is_some());
    assert!(editor.run(Command::Organize));
    let frame = editor.context_menu(Some(0));
    assert!(item(&frame, Command::SaveModel).is_some());
    let background = editor.context_menu(None);
    assert!(item(&background, Command::SaveModel).is_some());
}

#[test]
fn sm_b3_salvar_do_modo_organizar_guarda_a_lamina_ajustada() {
    let (mut editor, dir) = with_models(album(&[HHV]), 0);
    let before = rects(&editor);
    assert!(editor.run(Command::Organize));
    editor.organize_click(0, false, false);
    // Um ajuste à mão do modo: a seta move o quadro 1 mm (EG, regra 2).
    assert!(editor.run(Command::NudgeRight));
    let adjusted = rects(&editor);
    assert_ne!(adjusted, before, "o quadro andou");
    assert!(editor.organizing(), "salvar não sai do modo");
    // O caminho do modo: o item do menu do quadro, aceso, e o nome do diálogo (MD).
    let menu = editor.context_menu(Some(0));
    let (_, enabled) = item(&menu, Command::SaveModel).expect("o item está no menu do quadro");
    assert!(enabled);
    editor.save_model("Organizado").unwrap();
    assert!(editor.organizing(), "o modo continua depois de salvar");
    let model = editor.models()[0].clone();
    assert_eq!(model.name, "Organizado");
    assert_eq!(
        model.slots.iter().map(|s| s.rect).collect::<Vec<_>>(),
        adjusted,
        "o modelo guarda os quadros ajustados"
    );
    let again = ModelsStore::load(&dir.path().join("modelos.json"));
    assert_eq!(again.all(), [model], "lido de novo do arquivo, é igual");
}

#[test]
fn sm_b4_lamina_sem_quadro_de_foto_deixa_o_item_apagado() {
    // Uma lâmina com um texto e nenhum quadro de foto (MD, seção 6).
    let mut project = Project::new("teste", spec(30.0, 30.0));
    project.add_spread();
    let (mut editor, _dir) = with_models(project, 0);
    // `new_text` sai do modo organizar (correção TX de 2026-10-28): o texto vem antes.
    assert!(editor.new_text(Some(PageSide::Left)));
    assert!(!editor.can_save_model());
    assert!(editor.run(Command::Organize));
    let menu = editor.context_menu(Some(0));
    let (_, enabled) = item(&menu, Command::SaveModel).expect("o item aparece, apagado");
    assert!(!enabled, "sem quadro de foto, o item fica apagado");
}
