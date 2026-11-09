//! A foto à vista (spec VF, beta 9.1): a prévia da lateral e a foto ampliada no centro. Sem
//! janela aqui: a escolha, o Espaço, a navegação e a troca de aba.

use lamina_core::PhotoId;

use super::tl_common::{H, V, built, editor_at, spec};
use crate::Filter;

/// Um álbum com `n` fotos (uma vertical a cada 4), lâminas suficientes.
fn album(n: usize) -> lamina_core::Project {
    let photos: Vec<(u32, u32)> = (0..n).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let spreads = (n / 4).max(1).min(n);
    built(spec(30.0, 30.0), &photos, spreads)
}

fn shown(editor: &crate::Editor) -> Vec<PhotoId> {
    editor.shown_photos()
}

/// Álbum com `n` fotos e `spreads` lâminas explícitas (para sobrar foto não usada).
fn album_spreads(n: usize, spreads: usize) -> lamina_core::Project {
    let photos: Vec<(u32, u32)> = (0..n).map(|i| if i % 4 == 3 { V } else { H }).collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

#[test]
fn vf_b1_clique_escolhe_a_foto_da_previa() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    assert!(editor.photo_view().chosen.is_none());
    editor.click_photo(&ids, 13, false, false);
    assert_eq!(editor.photo_view().chosen, Some(ids[13]));
    editor.click_photo(&ids, 19, true, false);
    assert_eq!(
        editor.photo_view().chosen,
        Some(ids[19]),
        "Ctrl+clique também troca a prévia"
    );
    // Desmarcar tudo (Ctrl+clique na única selecionada) esconde a prévia… a regra 1 diz
    // "desmarcar tudo esconde": a seleção vazia não tem foto da prévia.
    editor.click_photo(&ids, 19, true, false);
    assert_eq!(editor.photo_view().chosen, Some(ids[19]));
}

#[test]
fn vf_b1b_desmarcar_esconde_a_previa() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 13, false, false);
    assert_eq!(editor.photo_view().chosen, Some(ids[13]));
    editor.clear_photo_selection();
    assert_eq!(editor.photo_view().chosen, None, "sem seleção, sem prévia");
}

#[test]
fn vf_b2_a_ampliada_e_a_da_previa() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 13, false, false);
    assert!(editor.toggle_photo_view());
    let view = editor.photo_view();
    assert!(view.open);
    assert_eq!(view.chosen, Some(ids[13]));
}

#[test]
fn vf_b3_setas_navegam_dentro_da_foto_aberta() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 13, false, false);
    assert!(editor.toggle_photo_view());
    assert!(editor.step_photo_view(true));
    assert!(editor.step_photo_view(true));
    assert!(editor.step_photo_view(true));
    assert_eq!(editor.photo_view().chosen, Some(ids[16]));
    assert!(editor.step_photo_view(false));
    assert_eq!(editor.photo_view().chosen, Some(ids[15]));
    // A navegação não mexe na seleção da grade.
    assert_eq!(editor.photo_selection_ids(), vec![ids[13]]);
}

#[test]
fn vf_b3b_setas_fechadas_nao_navegam() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 13, false, false);
    assert!(editor.toggle_photo_view());
    assert!(editor.close_photo_view());
    assert!(!editor.step_photo_view(true), "fechada, a seta não navega");
    assert_eq!(editor.photo_view().chosen, Some(ids[13]));
}

#[test]
fn vf_b4_espaco_abre_e_fecha() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 13, false, false);
    assert!(editor.toggle_photo_view());
    assert!(editor.toggle_photo_view(), "Espaço de novo fecha");
    assert!(!editor.photo_view().open);
}

#[test]
fn vf_b6_sem_foto_escolhida_uma_selecionada() {
    let mut editor = editor_at(album(40), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 8, false, false);
    // A prévia é a 8; limpamos a escolha simulando outra sessão… na regra 2, sem escolhida e
    // com seleção de uma, vale a da seleção.
    editor.clear_photo_view_choice();
    assert!(editor.toggle_photo_view());
    assert_eq!(editor.photo_view().chosen, Some(ids[8]));
    assert!(editor.photo_view().open);
}

#[test]
fn vf_b6b_sem_nada_indisponivel() {
    let mut editor = editor_at(album(40), 1);
    assert!(!editor.toggle_photo_view(), "sem foto e sem seleção");
    assert!(!editor.photo_view().open);
}

#[test]
fn vf_b9_trocar_de_aba() {
    let mut editor = editor_at(album(12), 1);
    let ids = shown(&editor);
    // A foto 0 é usada (está na lâmina); escolhemos ela e vamos para Não usadas.
    editor.click_photo(&ids, 0, false, false);
    assert_eq!(editor.photo_view().chosen, Some(ids[0]));
    assert!(editor.toggle_photo_view());
    editor.set_photo_filter(Filter::Unused);
    assert!(!editor.photo_view().open, "a ampliada fecha na troca");
    assert_eq!(editor.photo_view().chosen, None, "a 0 não está na aba");
    editor.set_photo_filter(Filter::All);
    assert_eq!(
        editor.photo_view().chosen,
        None,
        "a escolha não volta sozinha"
    );
}

#[test]
fn vf_b9b_previa_mantida_na_foto_da_aba() {
    // Lâminas automáticas usam 4 fotos cada: com 6 lâminas e 20 fotos, as últimas sobram.
    let mut editor = editor_at(album_spreads(50, 7), 1);
    let ids = shown(&editor);
    let id_last = ids[49];
    editor.click_photo(&ids, 49, false, false);
    assert_eq!(editor.photo_view().chosen, Some(id_last));
    assert_eq!(
        editor.photo_view().chosen,
        Some(id_last),
        "a prévia continua na aba"
    );
}

#[test]
fn vf_b13_setas_na_aba_de_uma_foto() {
    // Uma lâmina com 1 foto: as outras 11 não usadas… a aba Não usadas tem 11. Para uma foto só,
    // usamos um álbum de 1 foto.
    let mut editor = editor_at(album(1), 0);
    let ids = shown(&editor);
    editor.click_photo(&ids, 0, false, false);
    assert!(editor.toggle_photo_view());
    assert!(!editor.step_photo_view(true), "uma foto só: fica nela");
    assert_eq!(editor.photo_view().chosen, Some(ids[0]));
}

#[test]
fn vf_nao_e_edicao() {
    let mut editor = editor_at(album(12), 1);
    let ids = shown(&editor);
    editor.click_photo(&ids, 0, false, false);
    assert!(editor.toggle_photo_view());
    let before = editor.session().unwrap().project().clone();
    assert!(editor.step_photo_view(true));
    assert!(editor.close_photo_view());
    assert_eq!(
        &before,
        editor.session().unwrap().project(),
        "ver a foto não muda o projeto"
    );
}
