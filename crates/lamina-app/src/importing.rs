//! Fotos entrando no álbum (spec IM, parte de importar): "Importar fotos…" (Ctrl+I), "Importar
//! uma pasta de fotos…" e arquivos soltos do Explorador, pelo gancho do winit (ADR 0011, decisão
//! 9). A importação roda fora da linha da janela; as contas são do `lamina-editor`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use lamina_editor::importing::{WAIT_IMPORT, import_photos};
use lamina_editor::{Human, guarded};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, Timer};

use crate::state::{App, Msg};
use crate::{AppWindow, ImportUi};

type Shared = Rc<RefCell<App>>;

/// Os arquivos soltos chegam um evento por arquivo; depois de uma pausa curta, vão juntos.
const GATHER: Duration = Duration::from_millis(150);

/// "Importar fotos…" (vários arquivos) ou "Importar uma pasta de fotos…".
pub fn pick(app: &mut App, folder: bool) {
    let dialog = crate::dialogs::dialog(
        app,
        if folder {
            "Pasta com as fotos para importar"
        } else {
            "Fotos para importar"
        },
    );
    let chosen = |app: &mut App, paths: Option<Vec<rfd::FileHandle>>| {
        if let Some(paths) = paths {
            let paths = paths.iter().map(|p| p.path().to_path_buf()).collect();
            start(app, paths, None);
        }
    };
    if folder {
        let answer = dialog.pick_folder();
        crate::dialogs::then(async { answer.await.map(|p| vec![p]) }, chosen);
    } else {
        let photos = &["jpg", "jpeg", "png", "tif", "tiff", "heic", "heif"];
        crate::dialogs::then(dialog.add_filter("Fotos", photos).pick_files(), chosen);
    }
}

/// Importa `paths` no álbum aberto (com `as_spread`, já numa lâmina nova nesse vão; regra 11);
/// sem álbum, abre o Novo projeto com elas (regra 6).
pub fn start(app: &mut App, paths: Vec<PathBuf>, as_spread: Option<usize>) {
    if app.editor.busy() {
        app.notice = WAIT_IMPORT.to_owned();
        return app.refresh_current();
    }
    let Some(session) = app.session() else {
        return crate::new_project::start_with_files(app, paths);
    };
    let file = session.file().map(std::path::Path::to_path_buf);
    if let Some(w) = app.window.upgrade() {
        w.set_busy_text("Importando as fotos…".into());
    }
    app.editor.set_busy(true);
    let (cache, tx) = (app.cache.clone(), app.tx.clone());
    std::thread::spawn(move || {
        let progress = |done: usize, total: usize| {
            // Se a janela já fechou, não há a quem avisar.
            let _ = tx.send(Msg::Importing(done, total));
        };
        let result = guarded(|| import_photos(&paths, &cache, &progress).map_err(|e| e.human()))
            .and_then(|r| r);
        let _ = tx.send(Msg::Imported(result, file, as_spread));
    });
}

/// O andamento e o fim da importação.
pub fn message(app: &mut App, msg: Msg) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match msg {
        Msg::Importing(done, total) => {
            w.set_busy_text(format!("Importando {done} de {total} fotos…").into());
        }
        Msg::Imported(result, file, as_spread) => {
            w.set_busy_text("".into());
            app.editor.set_busy(false);
            // O álbum trocou no meio (não deveria, com o editor ocupado): nada a fazer.
            if app
                .session()
                .map(|s| s.file().map(std::path::Path::to_path_buf))
                != Some(file)
            {
                return;
            }
            let add =
                |app: &mut App, report: &lamina_editor::importing::ImportReport| match as_spread {
                    Some(at) => app.editor.add_photos_as_spread(report, at),
                    None => app.editor.add_photos(report),
                };
            match result.map(|report| add(app, &report)) {
                Ok(Ok(summary)) => {
                    // Só as fotos novas: as imagens que a janela tem ficam (spec TR, regra 8).
                    app.photos_added();
                    app.notice = summary.text();
                    app.refresh_current();
                    if app.verify_import.is_some() {
                        crate::verify::import_done(app, &app.notice);
                    }
                }
                Ok(Err(e)) => failed(app, &e.human()),
                Err(problem) => failed(app, &problem),
            }
        }
        _ => {}
    }
}

/// A importação falhou: a mensagem na janela (no teste de fumaça, a saída com o motivo).
fn failed(app: &App, problem: &lamina_editor::HumanMessage) {
    if app.verify_import.is_some() {
        crate::verify::import_failed(problem);
    }
    crate::files::show_error(app, problem);
}

/// O gancho do winit: arquivos sobre a janela mostram "Solte para importar"; soltos, vão juntos
/// para a importação depois de uma pausa curta. O mesmo gancho lê o monitor (spec ZR, regra 10).
pub fn connect(window: &AppWindow, app: &Shared) {
    let pending: Rc<RefCell<Vec<PathBuf>>> = Rc::default();
    let a = app.clone();
    let weak = window.as_weak();
    window.window().on_winit_window_event(move |window, event| {
        if crate::monitor::may_change(event)
            && let Ok(mut app) = a.try_borrow_mut()
            && crate::monitor::read(&mut app, window)
            && app.editor.organizing()
        {
            app.refresh_current();
        }
        let show = |on: bool| {
            if let Some(w) = weak.upgrade() {
                w.global::<ImportUi>().set_dropping(on);
            }
        };
        // A janela mudando de tamanho: a lâmina não anima até o silêncio (spec FB, regra 6).
        if let winit::event::WindowEvent::Resized(_) = event
            && let Ok(mut app) = a.try_borrow_mut()
        {
            crate::waits::resized(&mut app);
        }
        // De volta à janela: o disco pode ter sido conectado (spec RF, regra 6).
        if let winit::event::WindowEvent::Focused(true) = event
            && let Ok(app) = a.try_borrow()
        {
            crate::relink_dialog::recheck(&app);
        }
        match event {
            winit::event::WindowEvent::HoveredFile(_) => show(true),
            winit::event::WindowEvent::HoveredFileCancelled => show(false),
            winit::event::WindowEvent::DroppedFile(path) => {
                show(false);
                let first = pending.borrow().is_empty();
                pending.borrow_mut().push(path.clone());
                if first {
                    let (a, pending) = (a.clone(), pending.clone());
                    let weak = weak.clone();
                    Timer::single_shot(GATHER, move || {
                        let paths = std::mem::take(&mut *pending.borrow_mut());
                        let mut app = a.borrow_mut();
                        // Soltas sobre a área de lâmina nova, viram a lâmina nova (regra 11).
                        let over = weak.upgrade().is_some_and(|w| w.get_new_spread_hover());
                        let at = (over && app.session().is_some()).then(|| app.editor.album_end());
                        start(&mut app, paths, at);
                    });
                }
            }
            _ => {}
        }
        EventResult::Propagate
    });
}
