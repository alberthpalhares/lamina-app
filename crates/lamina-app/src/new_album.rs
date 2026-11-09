//! O álbum novo com fotos (spec JP, regra 4; spec NP): monta em segundo plano, com andamento, e
//! grava no arquivo escolhido na etapa 3 assim que fica pronto. As etapas ficam em
//! `new_project.rs`.

use lamina_editor::{Human, NewAlbum, Step, create_album, guarded};

use crate::files::show_error;
use crate::state::{App, Msg};

/// "Criar" com fotos: importa e monta fora da linha da janela.
pub fn build(app: &mut App, new: NewAlbum) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    w.set_busy_text("Importando as fotos…".into());
    app.editor.set_busy(true);
    let (cache, tx) = (app.cache.clone(), app.tx.clone());
    std::thread::spawn(move || {
        let progress = |step: Step| {
            // Se a janela já fechou, não há a quem avisar.
            let _ = tx.send(Msg::Step(step));
        };
        // Uma falha inesperada também libera a janela (spec EH, regra 6).
        let result = guarded(|| {
            create_album(&new, &cache, &progress)
                .map(|(built, failures)| (built.project, failures.len()))
                .map_err(|e| e.human())
        })
        .and_then(|r| r);
        let _ = tx.send(Msg::Album(result));
    });
}

/// Mensagens do trabalho em segundo plano.
pub fn message(app: &mut App, msg: Msg) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match msg {
        Msg::Step(Step::Importing { done, total }) => {
            w.set_busy_text(format!("Importando {done} de {total} fotos…").into());
        }
        Msg::Step(Step::Building) => w.set_busy_text("Montando o álbum…".into()),
        Msg::Album(result) => {
            w.set_busy_text("".into());
            app.editor.set_busy(false);
            w.invoke_focus_main();
            match result {
                Ok((project, failures)) => {
                    // Criar já grava o arquivo (spec NP, regra 8); ele entra nos recentes.
                    let saved = match app.new_target.take() {
                        Some(file) => app.editor.adopt_saved(project, &file).map(|_| ()),
                        None => {
                            app.editor.adopt(project);
                            Ok(())
                        }
                    };
                    app.project_changed();
                    crate::recents::refresh(app);
                    if let Err(e) = saved {
                        show_error(app, &e.human());
                    }
                    if failures > 0 {
                        app.notice = format!("{failures} foto(s) não puderam ser importadas.");
                        app.refresh_current();
                    }
                }
                Err(e) => show_error(app, &e),
            }
        }
        Msg::ImagesReady => app.images_ready(),
        Msg::SystemMotion(reduces) => crate::window_look::motion_read(app, reduces),
        Msg::Screen { epoch, id, pixels } => {
            app.images.screen_arrived(epoch, id, pixels);
            // A foto à vista pode ter chegado (spec VF): a prévia e a ampliada trocam na hora.
            if app.editor.photo_view().chosen == Some(id) {
                crate::photo_view::refresh(app);
            }
        }
        Msg::Thumb { epoch, id, pixels } => app.images.thumb_arrived(epoch, id, pixels),
        Msg::SpreadImage(painted) => crate::roll::arrived(app, *painted),
        Msg::Original(id, pixels) => crate::crop::original_ready(app, id, pixels),
        Msg::Exporting(..) | Msg::Exported(..) => crate::export::message(app, msg),
        Msg::Approving(..) | Msg::Approved(..) => crate::approval::message(app, msg),
        Msg::Searching(..) | Msg::Relinked { .. } => crate::relink::message(app, msg),
        Msg::Rechecked { file, still } => crate::relink_dialog::rechecked(app, file, &still),
        Msg::Located { file, result } => crate::relink_dialog::located(app, file, result),
        Msg::Discovered { file, found } => crate::relink_dialog::discovered(app, file, found),
        Msg::Update(outcome) => crate::updates::message(app, outcome),
        Msg::Failed(problem) => failed(app, &problem),
        Msg::RecentsChecked(_) | Msg::ThumbReady => crate::recents::message(app, msg),
        Msg::Importing(..) | Msg::Imported(..) => crate::importing::message(app, msg),
        Msg::Built(result) => crate::build_album::finished(app, result),
    }
}

/// Uma tarefa em segundo plano quebrou: a janela volta a responder e mostra a mensagem (spec EH,
/// regra 6).
fn failed(app: &mut App, problem: &lamina_editor::HumanMessage) {
    if let Some(w) = app.window.upgrade() {
        w.set_busy_text("".into());
    }
    app.editor.set_busy(false);
    app.loading = None;
    show_error(app, problem);
}
