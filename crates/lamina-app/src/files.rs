//! Abrir, salvar e a pergunta antes de sair (spec JP, regras 8 e 9), com os diálogos de arquivo
//! do sistema (`rfd`).

use std::path::{Path, PathBuf};

use lamina_editor::{Human, HumanMessage, Leave};
use slint::ComponentHandle;

use crate::state::{App, Pending};

const FILTER_NAME: &str = "Álbum do Lamina";
const EXT: &str = "lamina";

/// O diálogo de erro com a mensagem humana: a frase, o que fazer e o detalhe em "Mostrar
/// detalhes" (spec EH). Toda mensagem mostrada vai para o registro (spec SR, regra 5).
pub fn show_error(app: &App, message: &HumanMessage) {
    app.editor.record(message);
    if let Some(w) = app.window.upgrade() {
        w.set_message_title(message.title.as_str().into());
        w.set_message_text(message.what_to_do.as_str().into());
        w.set_message_details(message.details.as_str().into());
        w.set_message_folder("".into());
        w.set_show_message(true);
    }
}

/// O arquivo escolhido no "Salvar como", sempre com a extensão `.lamina`.
fn with_ext(path: &Path) -> PathBuf {
    if path.extension().is_some_and(|e| e == EXT) {
        path.to_path_buf()
    } else {
        path.with_extension(EXT)
    }
}

/// Salva (ou "Salvar como", se `as_new` ou se ainda não tem arquivo) e roda `after` com `true` se
/// salvou. O "Salvar como" espera a resposta do diálogo.
pub fn save(app: &mut App, as_new: bool, after: impl FnOnce(&mut App, bool) + 'static) {
    let Some(session) = app.editor.session() else {
        return after(app, false);
    };
    if !as_new && session.file().is_some() {
        let saved = save_to(app, None);
        return after(app, saved);
    }
    let name = session.title().to_owned();
    let answer = crate::dialogs::dialog(app, "Salvar como")
        .add_filter(FILTER_NAME, &[EXT])
        .set_file_name(format!("{name}.{EXT}"))
        .save_file();
    crate::dialogs::then(answer, move |app, path| {
        let saved = path.is_some_and(|p| save_to(app, Some(&with_ext(p.path()))));
        after(app, saved);
    });
}

/// Grava no arquivo do álbum ou em `target`. `true` = salvou.
fn save_to(app: &mut App, target: Option<&Path>) -> bool {
    // Salvar põe o álbum no topo dos recentes e gera a miniatura (spec TI, regras 1 e 5).
    let result = app.editor.save_to(target);
    app.refresh_current();
    match result {
        Ok(_) => {
            crate::recents::thumbnail(app);
            crate::recents::refresh(app);
            // Salvo: a cópia de segurança vai embora (spec CS, regra 2).
            crate::backup::tick(app);
            true
        }
        Err(e) => {
            show_error(app, &e.human());
            false
        }
    }
}

/// Fechar, abrir ou criar: pergunta antes se há alterações não salvas.
pub fn request(app: &mut App, action: Pending) {
    if app.editor.needs_confirmation() {
        app.pending = Some(action);
        if let Some(w) = app.window.upgrade() {
            w.set_show_confirm(true);
        }
    } else {
        proceed(app, action);
    }
}

/// Resposta à pergunta: 0 = Salvar, 1 = Não salvar, 2 = Cancelar.
pub fn answer(app: &mut App, choice: i32) {
    if let Some(w) = app.window.upgrade() {
        w.set_show_confirm(false);
        w.invoke_focus_main();
    }
    let Some(action) = app.pending.take() else {
        return;
    };
    let choice = match choice {
        0 => Leave::Save,
        1 => Leave::DontSave,
        _ => Leave::Cancel,
    };
    if choice == Leave::Save {
        save(app, false, move |app, saved| {
            after_answer(app, action, saved)
        });
    } else {
        let go = app.editor.leave(choice, None).unwrap_or(false);
        after_answer(app, action, go);
    }
}

/// Segue com a ação depois da pergunta, ou volta atrás se não deu para sair.
fn after_answer(app: &mut App, action: Pending, go: bool) {
    if go {
        proceed(app, action);
    } else if action == Pending::Update {
        crate::updates::restart_cancelled(app);
    }
}

fn proceed(app: &mut App, action: Pending) {
    match action {
        Pending::Close => {
            if let Some(w) = app.window.upgrade() {
                // Já fechando: um erro aqui não muda nada.
                let _ = w.hide();
            }
        }
        Pending::Open => open(app),
        Pending::NewAlbum => crate::new_project::start(app),
        Pending::Update => {
            // O fotógrafo já respondeu "Salvar as alterações?" (spec CS, regra 3).
            app.editor.finish_backups();
            // O programa fecha sem voltar por `main` (spec AL, regra 19).
            app.editor.flush_learned();
            crate::updates::restart(app);
        }
        Pending::Example => crate::welcome::open_example(app),
        Pending::OpenRecent(i) => crate::recents::open(app, i),
        Pending::OpenReceived => crate::opened::proceed(app),
        Pending::CloseAlbum => {
            if app.editor.close_album() {
                app.project_changed();
                crate::recents::refresh(app);
                crate::recents::check(app);
            }
        }
    }
}

fn open(app: &mut App) {
    let answer = crate::dialogs::dialog(app, "Abrir álbum")
        .add_filter(FILTER_NAME, &[EXT])
        .pick_file();
    crate::dialogs::then(answer, |app, file| {
        if let Some(file) = file {
            open_file(app, file.path());
        }
    });
}

/// Abre `file`; se não der, mostra o erro e o projeto anterior continua aberto.
pub fn open_file(app: &mut App, file: &Path) {
    match app.editor.open(file) {
        Ok(()) => {
            app.project_changed();
            crate::relink::near(app);
            // Um álbum que nunca foi salvo por esta versão também ganha a miniatura (TI-B18).
            crate::recents::thumbnail(app);
            crate::recents::refresh(app);
        }
        Err(e) => show_error(app, &e.human()),
    }
}
