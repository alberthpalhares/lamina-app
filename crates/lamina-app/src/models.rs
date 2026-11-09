//! Modelos pela janela (specs MD e RM, beta 9.1): o nome ao salvar a lâmina como modelo e o painel
//! de gerenciamento (meus e de fábrica, agrupados por quantidade de fotos, com busca, favoritar,
//! ocultar de fábrica, salvar como cópia e restaurar). As decisões são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::Human;
use lamina_editor::naming::{Named, model_saved};
use slint::ComponentHandle;

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, ModelBox, ModelRow, ModelUi};

type Shared = Rc<RefCell<App>>;

/// Lâmina → Salvar como modelo…
pub fn open_save(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    ui.set_name("".into());
    ui.set_name_message("".into());
    ui.set_show_name(true);
    app.name_guard.reset();
}

fn name_save(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    let name = ui.get_name().trim().to_owned();
    let exists = app.editor.model_exists(&name);
    if let Err(text) = app.name_guard.check(Named::Model, &name, exists) {
        ui.set_name_message(text.into());
        return;
    }
    match app.editor.save_model(&name) {
        Ok(()) => {
            ui.set_show_name(false);
            app.notice = model_saved(&name);
            app.refresh_current();
            w.invoke_focus_main();
        }
        Err(e) => ui.set_name_message(e.human().line().into()),
    }
}

/// As linhas do painel de gerenciamento, filtradas pela busca (spec RM, seção 3).
fn rows(app: &App, search: &str) -> Vec<ModelRow> {
    let editor = &app.editor;
    let needle = search.trim().to_lowercase();
    // Os retângulos dos quadros viram a miniatura (frações do corte).
    // As frações do corte cabem em f32 com folga (miniatura de 56 px).
    #[allow(clippy::cast_possible_truncation)]
    let boxes_of = |slots: &[lamina_layout::ModelSlot]| -> Vec<ModelBox> {
        slots
            .iter()
            .map(|s| ModelBox {
                x: s.rect.x() as f32,
                y: s.rect.y() as f32,
                w: s.rect.width() as f32,
                h: s.rect.height() as f32,
            })
            .collect()
    };
    let mut mine: Vec<(usize, String, Vec<ModelBox>)> = editor
        .models()
        .iter()
        .map(|m| (m.slots.len(), m.name.clone(), boxes_of(&m.slots)))
        .collect();
    mine.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut factory: Vec<(usize, String, bool, Vec<ModelBox>)> =
        lamina_layout::factory::factory_models_all()
            .into_iter()
            .map(|m| {
                let hidden = editor
                    .preferences()
                    .hidden
                    .contains(&lamina_layout::model_key(&m.name, false, true));
                (m.slots.len(), m.name, hidden, boxes_of(&m.slots))
            })
            .filter(|(_, _, hidden, _)| !hidden)
            .collect();
    factory.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<ModelRow> = Vec::new();
    let mut last_group = 0;
    let mut push =
        |group: usize, name: &str, factory: bool, favorite: bool, boxes: Vec<ModelBox>| {
            let group_title = if group != last_group {
                format!("{group} {}", if group == 1 { "foto" } else { "fotos" })
            } else {
                String::new()
            };
            last_group = group;
            out.push(ModelRow {
                heading: group_title.into(),
                name: name.into(),
                factory,
                favorite,
                boxes: model(boxes),
            });
        };
    for (group, name, boxes) in mine {
        if needle.is_empty() || name.to_lowercase().contains(&needle) || needle == group.to_string()
        {
            let key = lamina_layout::model_key(&name, false, false);
            push(
                group,
                &name,
                false,
                editor.preferences().favorites.contains(&key),
                boxes,
            );
        }
    }
    for (group, name, _, boxes) in factory {
        if needle.is_empty() || name.to_lowercase().contains(&needle) || needle == group.to_string()
        {
            let key = lamina_layout::model_key(&name, false, true);
            push(
                group,
                &name,
                true,
                editor.preferences().favorites.contains(&key),
                boxes,
            );
        }
    }
    out
}

fn refresh_manage(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    let search = ui.get_search().to_string();
    let list = rows(app, &search);
    ui.set_rows(model(list));
    ui.set_can_restore(app.editor.can_restore_factory());
}

/// Lâmina → Gerenciar modelos… (spec RM).
pub fn open_manage(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    ui.set_manage_message("".into());
    ui.set_manage_name("".into());
    ui.set_search("".into());
    refresh_manage(app);
    ui.set_show_manage(true);
}

fn search_changed(app: &mut App, text: &str) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    ui.set_search(text.into());
    refresh_manage(app);
}

fn row_favorite(app: &mut App, name: &str, factory: bool) {
    let key = lamina_layout::model_key(name, false, factory);
    let prefs = app.editor.preferences().clone();
    let now_favorite = prefs.favorites.contains(&key);
    app.editor.preferences_mark(&key, !now_favorite);
    refresh_manage(app);
    app.refresh_current();
}

fn row_remove(app: &mut App, name: &str, factory: bool) {
    if factory {
        app.editor.hide_factory_model(name);
    } else {
        let _ = app.editor.remove_model(name);
    }
    refresh_manage(app);
    app.refresh_current();
}

fn row_copy(app: &mut App, source: &str) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    ui.set_copy_source(source.into());
    ui.set_copy_name("".into());
    ui.set_copy_message("".into());
    ui.set_show_copy(true);
}

fn copy_save(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<ModelUi>();
    let name = ui.get_copy_name().trim().to_owned();
    let source = ui.get_copy_source().to_string();
    let exists = app.editor.factory_copy_exists(&name);
    if let Err(text) = app.name_guard.check(Named::Model, &name, exists) {
        ui.set_copy_message(text.into());
        return;
    }
    match app.editor.save_factory_copy(&source, &name) {
        Ok(()) => {
            ui.set_show_copy(false);
            app.notice = model_saved(&name);
            refresh_manage(app);
            app.refresh_current();
            w.invoke_focus_main();
        }
        Err(e) => ui.set_copy_message(e.human().line().into()),
    }
}

fn restore(app: &mut App) {
    if app.editor.restore_factory_models() {
        refresh_manage(app);
        app.refresh_current();
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<ModelUi>();
    let a = app.clone();
    ui.on_name_save(move || name_save(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_search_changed(move |text| search_changed(&mut a.borrow_mut(), &text));
    let a = app.clone();
    ui.on_row_favorite(move |name, factory| row_favorite(&mut a.borrow_mut(), &name, factory));
    let a = app.clone();
    ui.on_row_remove(move |name, factory| row_remove(&mut a.borrow_mut(), &name, factory));
    let a = app.clone();
    ui.on_row_copy(move |source| row_copy(&mut a.borrow_mut(), &source));
    let a = app.clone();
    ui.on_copy_save(move || copy_save(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_restore(move || restore(&mut a.borrow_mut()));
    let w = window.as_weak();
    ui.on_closed(move || {
        if let Some(w) = w.upgrade() {
            w.invoke_focus_main();
        }
    });
}
