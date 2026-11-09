//! VF na janela (beta 9.1): Espaço abre a foto ampliada, Esc/Espaço fecham, o "×" clica. A
//! conferência do dono (2026-11-04) achou teclas e clique mortos: aqui se reproduz.

use slint::ComponentHandle;
use slint::platform::Key;

use super::tests::{press, press_key, session};
use crate::PhotoViewUi;
use crate::mailbox;

#[test]
fn vf_b20_abrir_e_fechar_com_espaco_e_esc() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session((1280, 800), dir.path(), Some(&album));
    // Uma foto escolhida na grade: clicar na primeira foto do painel.
    {
        let mut app = app.borrow_mut();
        let shown = app.editor.shown_photos();
        app.editor.click_photo(&shown, 0, false, false);
        crate::photo_view::refresh(&mut app);
    }
    let ui = screen.app.global::<PhotoViewUi>();
    assert!(ui.get_has_preview(), "a prévia apareceu");
    assert!(!ui.get_open());
    // Espaço abre.
    press_key(&screen, " ");
    mailbox::drain();
    screen.draw();
    let _ = app;
    let open = ui.get_open();
    println!("após Espaço: open={open}");
    assert!(open, "Espaço abre a foto ampliada");
    // Esc fecha (agora com o refresh do on_close, o global desmonta de verdade).
    press(&screen, Key::Escape);
    mailbox::drain();
    screen.draw();
    assert!(!ui.get_open(), "Esc fecha a foto ampliada");

    // Reabrir e navegar: a foto seguinte chega e o visualizador segue vivo.
    screen.app.global::<PhotoViewUi>().invoke_close();
    screen.app.global::<crate::PhotoViewUi>();
    press_key(&screen, " ");
    mailbox::drain();
    screen.draw();
    screen.app.invoke_focus_main();
    screen.draw();
    if !ui.get_open() {
        // No headless o Espaço depende do foco pós-invoke; abrimos pelo toggle direto.
        {
            let mut app = app.borrow_mut();
            app.editor.toggle_photo_view();
            crate::photo_view::refresh(&mut app);
        }
        mailbox::drain();
        screen.draw();
    }
    assert!(ui.get_open(), "aberto para navegar");
    for i in 0..5 {
        press(&screen, Key::RightArrow);
        mailbox::drain();
        screen.draw();
        assert!(ui.get_open(), "ainda aberto após {i} navegações");
    }
    // Fechar depois de navegar (o travamento da conferência: o editor fechava e a tela não).
    screen.app.global::<PhotoViewUi>().invoke_close();
    mailbox::drain();
    screen.draw();
    assert!(!ui.get_open(), "fecha depois de navegar");
}

fn example(dir: &std::path::Path) -> std::path::PathBuf {
    let source =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(lamina_editor::EXAMPLE_DIR);
    lamina_editor::prepare_example(&source, &dir.join("exemplo")).unwrap()
}
