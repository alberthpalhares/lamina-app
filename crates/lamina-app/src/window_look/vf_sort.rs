//! O seletor de ordenação na janela (spec OF): clicar numa opção do dropdown troca o critério
//! (conferência do dono 2026-11-08: o dropdown abria mas nada mudava).

use slint::ComponentHandle;

use super::tests::session;
use crate::mailbox;

#[test]
fn of_b20_o_clique_na_opcao_troca_o_criterio() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session((1280, 800), dir.path(), Some(&album));
    let ui = screen.app.global::<crate::SortUi>();
    assert_eq!(ui.get_sort(), 0, "começa na data de captura");
    // O callback do global é quem avisa o Rust; chamamos como o clique faria.
    ui.invoke_changed(1);
    mailbox::drain();
    screen.draw();
    assert_eq!(ui.get_sort(), 1, "o critério foi para Nome do arquivo");
    let _ = app;
}

fn example(dir: &std::path::Path) -> std::path::PathBuf {
    let source =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(lamina_editor::EXAMPLE_DIR);
    lamina_editor::prepare_example(&source, &dir.join("exemplo")).unwrap()
}

#[test]
fn of_b21_a_personalizada_reordena_pela_janela() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session((1280, 800), dir.path(), Some(&album));
    // Critério Personalizada.
    screen.app.global::<crate::SortUi>().invoke_changed(2);
    mailbox::drain();
    screen.draw();
    {
        let app = app.borrow();
        let sort = app.editor.photo_sort();
        assert!(
            matches!(sort, lamina_editor::ordering::SortBy::Custom),
            "o critério é {sort:?}"
        );
    }
    let before = app.borrow().shown_photos();
    assert!(before.len() > 3);
    // Como o arrasto faria: mover a foto 3 para a posição 0.
    screen.app.invoke_photo_reorder(3, 0);
    mailbox::drain();
    screen.draw();
    let after = app.borrow().shown_photos();
    assert_ne!(before, after, "a ordem mudou: {:?} → {:?}", before, after);
}
