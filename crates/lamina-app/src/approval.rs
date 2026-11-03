//! A aba "Para aprovação do cliente" de Exportar (specs AC e AH): traduz os campos para o editor, confere
//! antes de gerar e gera fora da linha da janela, com andamento e resumo. A decisão é do editor.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lamina_editor::forms::short_path;
use lamina_editor::{
    APPROVAL_FILE, ApprovalForm, ApprovalStore, Human, approval_finished, approval_progress,
    approval_replace, config_dir, generate_approval, guarded,
};
use slint::ComponentHandle;

use crate::convert::model;
use crate::files::show_error;
use crate::state::{App, Msg};
use crate::{AppWindow, ApprovalUi};

type Shared = Rc<RefCell<App>>;

/// Quantas letras da pasta cabem na linha da caixa (spec DD, regra 17).
const SHORT_FOLDER: usize = 40;

fn store() -> ApprovalStore {
    ApprovalStore::load(&config_dir().join(APPROVAL_FILE))
}

/// Prepara a aba com a pasta deste álbum e o que o fotógrafo usou por último. Quem abre a janela é
/// o `export::start`.
pub fn prepare(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    let ui = w.global::<ApprovalUi>();
    let store = store();
    let form = ApprovalForm::new(&store.data, session.album());
    ui.set_folder(session.approval_folder().display().to_string().into());
    ui.set_html(form.html);
    ui.set_jpg(form.jpg);
    ui.set_client_name(form.client_name.into());
    ui.set_all_spreads(form.all_spreads);
    ui.set_range(form.range.into());
    ui.set_number(form.number);
    ui.set_mark_on(form.watermark_on);
    ui.set_mark_text(form.watermark_text.into());
    ui.set_whatsapp(form.whatsapp.into());
    ui.set_email(form.email.into());
    ui.set_data_warning(store.warning.unwrap_or_default().into());
    ui.set_error("".into());
    ui.set_step(0);
    app.approval_check = None;
    changed(app);
}

fn form(ui: &ApprovalUi) -> ApprovalForm {
    ApprovalForm {
        jpg: ui.get_jpg(),
        html: ui.get_html(),
        all_spreads: ui.get_all_spreads(),
        range: ui.get_range().into(),
        number: ui.get_number(),
        watermark_on: ui.get_mark_on(),
        watermark_text: ui.get_mark_text().into(),
        client_name: ui.get_client_name().into(),
        whatsapp: ui.get_whatsapp().into(),
        email: ui.get_email().into(),
    }
}

fn folder(ui: &ApprovalUi) -> PathBuf {
    PathBuf::from(ui.get_folder().as_str())
}

/// Confere a cada mudança: cada campo diz o que há de errado, e "Gerar arquivos" fica desabilitado.
pub fn changed(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    let ui = w.global::<ApprovalUi>();
    let form = form(&ui);
    let problems = form.problems(session.album());
    let text = |problem: &Option<String>| problem.clone().unwrap_or_default().into();
    ui.set_outputs_error(text(&problems.outputs));
    ui.set_name_error(text(&problems.name));
    ui.set_range_error(text(&problems.range));
    ui.set_mark_error(text(&problems.watermark));
    ui.set_whatsapp_error(text(&problems.whatsapp));
    ui.set_email_error(text(&problems.email));
    ui.set_whatsapp_shown(form.whatsapp_shown().into());
    ui.set_ready(problems.is_clear());
    ui.set_error("".into());
    ui.set_folder_short(short_path(ui.get_folder().as_str(), SHORT_FOLDER).into());
}

pub fn pick_folder(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let current = folder(&w.global::<ApprovalUi>());
    let start = if current.is_dir() {
        current
    } else {
        current.parent().map(Path::to_path_buf).unwrap_or_default()
    };
    let answer = crate::dialogs::dialog(app, "Pasta da aprovação do cliente")
        .set_directory(start)
        .pick_folder();
    crate::dialogs::then(answer, |app, chosen| {
        if let (Some(chosen), Some(w)) = (chosen, app.window.upgrade()) {
            let ui = w.global::<ApprovalUi>();
            ui.set_folder(chosen.path().display().to_string().into());
            changed(app);
        }
    });
}

/// "Gerar arquivos": foto não encontrada para tudo; com avisos, pergunta antes (AC, regras 15 e 16).
pub fn generate(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    let ui = w.global::<ApprovalUi>();
    match session.check_approval(&folder(&ui), &form(&ui).options(false)) {
        Ok(check) => {
            let warned = !check.warnings.is_empty() || check.cover.is_some();
            let lines = check
                .warnings
                .iter()
                .map(|x| x.to_string().into())
                .collect();
            ui.set_warnings(model(lines));
            ui.set_cover_notice(check.cover.clone().unwrap_or_default().into());
            app.approval_check = Some(check);
            if warned {
                ui.set_step(1);
            } else {
                proceed(app);
            }
        }
        Err(e) => ui.set_error(e.human().line().into()),
    }
}

/// Depois dos avisos: pergunta antes de substituir o que já existe (AC, regra 14).
pub fn proceed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ApprovalUi>();
    let existing = app.approval_check.as_ref().map_or(0, |c| c.existing.len());
    if existing == 0 {
        run(app, false);
        return;
    }
    let (title, text) = approval_replace(existing, &folder(&ui), &form(&ui));
    ui.set_replace_title(title.into());
    ui.set_replace_text(text.into());
    ui.set_step(2);
}

pub fn back(app: &mut App) {
    if let Some(w) = app.window.upgrade() {
        w.global::<ApprovalUi>().set_step(0);
    }
}

/// Gera fora da linha da janela, que fica parada com o andamento até terminar (AC, regra 18).
pub fn run(app: &mut App, overwrite: bool) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.editor.session_mut()) else {
        return;
    };
    let ui = w.global::<ApprovalUi>();
    let (folder, form) = (folder(&ui), form(&ui));
    session.set_approval_folder(&folder);
    let project = session.album_shared();
    w.set_show_export(false);
    w.set_busy_text("Gerando…".into());
    app.editor.set_busy(true);
    let (tx, cache) = (app.tx.clone(), app.cache.clone());
    std::thread::spawn(move || {
        let progress = |done: usize, total: usize| {
            // Se a janela já fechou, não há a quem avisar.
            let _ = tx.send(Msg::Approving(done, total));
        };
        // Uma falha inesperada também libera a janela (spec EH, regra 6).
        let result = guarded(|| {
            generate_approval(
                &project,
                &folder,
                &form.options(overwrite),
                &cache,
                &progress,
            )
            .map(|report| approval_finished(store(), form.data(), &report))
            .map_err(|e| e.human())
        })
        .and_then(|r| r);
        let _ = tx.send(Msg::Approved(result, folder));
    });
}

/// Andamento e fim da geração. O fim usa a mesma mensagem da exportação, com "Abrir a pasta".
pub fn message(app: &mut App, msg: Msg) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    match msg {
        Msg::Approving(done, total) => w.set_busy_text(approval_progress(done, total).into()),
        Msg::Approved(result, folder) => {
            w.set_busy_text("".into());
            app.editor.set_busy(false);
            match result {
                Ok(summary) => {
                    w.set_message_title("Arquivos prontos".into());
                    w.set_message_text(summary.into());
                    w.set_message_details("".into());
                    w.set_message_folder(folder.display().to_string().into());
                    w.set_show_message(true);
                }
                Err(e) => show_error(app, &e),
            }
        }
        _ => {}
    }
}

/// Liga a aba ao estado do programa.
pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<ApprovalUi>();
    let a = app.clone();
    ui.on_pick_folder(move || pick_folder(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_changed(move || changed(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_generate(move || generate(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_proceed(move || proceed(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_replace(move || run(&mut a.borrow_mut(), true));
    let a = app.clone();
    ui.on_back(move || back(&mut a.borrow_mut()));
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;
