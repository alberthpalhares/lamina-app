//! Cenários de painel da RM (beta 9.1): ocultar e restaurar de fábrica, salvar como cópia, as
//! chaves separadas e a biblioteca na montagem pela janela.

use super::tl_common::{H, V, built, editor_at, spec};

fn album_hhv() -> lamina_core::Project {
    built(spec(30.0, 30.0), &[H, H, V], 1)
}

#[test]
fn rm_b6_ocultar_e_restaurar_de_fabrica() {
    let mut editor = editor_at(album_hhv(), 0);
    // Um modelo de fábrica da mistura 2H 1V aparece na faixa.
    let titles = super::rm_common::titles(&mut editor);
    assert!(
        titles.iter().any(|t| t.starts_with("Modelo de fábrica")),
        "há de fábrica na faixa: {titles:?}"
    );
    assert!(!super::rm_common::first_factory_name(&editor).is_empty());
    // Ocultar o primeiro de fábrica some da faixa; restaurar traz de volta.
    let name = super::rm_common::first_factory_name(&editor);
    let full = format!("Modelo de fábrica · {name}");
    assert!(editor.hide_factory_model(&name));
    let titles = super::rm_common::titles(&mut editor);
    assert!(
        !titles
            .iter()
            .any(|t| *t == full || *t == format!("{full} · espelhado")),
        "oculto some da faixa (ficaram: {titles:?})"
    );
    assert!(editor.can_restore_factory());
    assert!(editor.restore_factory_models());
    let titles = super::rm_common::titles(&mut editor);
    assert!(titles.contains(&full), "restaurado volta");
    assert!(!editor.can_restore_factory());
}

#[test]
fn rm_b8_restaurar_nao_mexe_nos_meus_modelos() {
    let mut editor = editor_at(album_hhv(), 0);
    // Um modelo meu favoritado…
    editor.save_model("Abertura").unwrap();
    let key = lamina_layout::model_key("Abertura", false, false);
    editor.preferences_mark(&key, true);
    // Um de fábrica oculto.
    let name = super::rm_common::first_factory_name(&editor);
    assert!(editor.hide_factory_model(&name));
    assert!(editor.restore_factory_models());
    // "Abertura" continua favorito e salvo.
    assert!(editor.model_exists("Abertura"));
    assert!(editor.preferences().favorites.contains(&key));
}

#[test]
fn rm_b7_renomear_de_fabrica_vira_copia() {
    let mut editor = editor_at(album_hhv(), 0);
    let name = super::rm_common::first_factory_name(&editor);
    editor.save_factory_copy(&name, "Minha cópia").unwrap();
    assert!(editor.model_exists("Minha cópia"));
    // O original continua na fábrica (a cópia não o substitui na faixa: os dois aparecem).
    let titles = super::rm_common::titles(&mut editor);
    assert!(titles.iter().any(|t| t.contains("Minha cópia")));
    assert!(titles.iter().any(|t| t.contains(&name)));
}

#[test]
fn rm_b15_chaves_separadas() {
    let mut editor = editor_at(album_hhv(), 0);
    let name = super::rm_common::first_factory_name(&editor);
    // Cópia com o mesmo nome do original de fábrica…
    // (a conferência de nome repetido contra a fábrica é do diálogo; aqui, chaves separadas)
    editor.save_factory_copy(&name, "Cópia A").unwrap();
    // Favoritar a cópia não favorita o de fábrica.
    let copy_key = lamina_layout::model_key("Cópia A", false, false);
    let factory_key = lamina_layout::model_key(&name, false, true);
    editor.preferences_mark(&copy_key, true);
    assert!(editor.preferences().favorites.contains(&copy_key));
    assert!(!editor.preferences().favorites.contains(&factory_key));
}

#[test]
fn rm_b17_a_montagem_da_janela_usa_as_listas_novas() {
    // A montagem com window (janela) e sem (linha de comando) diferem; sem window, o resultado é
    // o mesmo de antes da spec (os testes CB cobrem isso). Aqui: com window, muda.
    let photos: Vec<(u32, u32)> = (0..8).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let project = built(spec(30.0, 30.0), &photos, 2);
    let plain = lamina_layout::auto_build(
        &project,
        2,
        &lamina_layout::BuildOptions {
            ..Default::default()
        },
    )
    .unwrap();
    let window = lamina_layout::auto_build(
        &project,
        2,
        &lamina_layout::BuildOptions {
            window: true,
            ..Default::default()
        },
    )
    .unwrap();
    // Pode coincidir; o que não pode é a linha de comando mudar (os quadros de `plain` são os de
    // sempre — garantido pelos testes CB; aqui só conferimos que a janela não quebra).
    assert_eq!(window.project.spreads.len(), plain.project.spreads.len());
}

#[test]
fn rm_b9b_montagem_rapida_com_fabrica() {
    // O BuildOptions da janela com a fábrica toda não explode: as lâminas saem.
    let photos: Vec<(u32, u32)> = (0..8).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let project = built(spec(30.0, 30.0), &photos, 2);
    let built = lamina_layout::auto_build(
        &project,
        2,
        &lamina_layout::BuildOptions {
            window: true,
            models: lamina_layout::factory::factory_models_all(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!built.project.spreads.is_empty());
}
