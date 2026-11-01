//! O menu de contexto pela janela (spec EG, regra 13): o clique direito pede o menu ao editor
//! (que ajusta a seleção), a janela mostra as linhas e o item escolhido roda o comando do
//! registro. Os submenus abrem no lugar.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_core::PageSide;
use lamina_editor::Command;
use lamina_editor::editor::ContextEntry;
use slint::ComponentHandle;

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, ContextRow, ContextUi};

type Shared = Rc<RefCell<App>>;

/// O menu aberto e o submenu expandido.
#[derive(Default)]
struct Open {
    entries: Vec<ContextEntry>,
    sub: Option<&'static str>,
}

/// O que uma linha faz ao ser clicada.
enum Action {
    Run(Command),
    Toggle(&'static str),
    Nothing,
}

fn row(label: &str, kind: i32, enabled: bool, indent: bool) -> ContextRow {
    ContextRow {
        label: label.into(),
        kind,
        enabled,
        open: false,
        indent,
        checked: false,
    }
}

fn push_item(rows: &mut Vec<(ContextRow, Action)>, entry: &ContextEntry, indent: bool) {
    if let ContextEntry::Item {
        command,
        label,
        enabled,
        checked,
    } = entry
    {
        let mut line = row(label, 0, *enabled, indent);
        line.checked = *checked;
        rows.push((line, Action::Run(*command)));
    }
}

/// As linhas do menu, com o submenu `sub` aberto.
fn rows(open: &Open) -> Vec<(ContextRow, Action)> {
    let mut rows = Vec::new();
    for e in &open.entries {
        match e {
            ContextEntry::Item { .. } => push_item(&mut rows, e, false),
            ContextEntry::Separator => rows.push((row("", 2, false, false), Action::Nothing)),
            ContextEntry::Sub { title, items } => {
                let expanded = open.sub == Some(*title);
                let mut head = row(title, 1, true, false);
                head.open = expanded;
                rows.push((head, Action::Toggle(title)));
                if expanded {
                    items.iter().for_each(|i| push_item(&mut rows, i, true));
                }
            }
        }
    }
    rows
}

fn show(window: &AppWindow, open: &Open) {
    let ui = window.global::<ContextUi>();
    ui.set_rows(model(rows(open).into_iter().map(|(r, _)| r).collect()));
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let state = Rc::new(RefCell::new(Open::default()));
    let (a, st) = (app.clone(), state.clone());
    window.on_context_requested(move |i, x, y, fx| {
        let mut app = a.borrow_mut();
        let entries = match usize::try_from(i) {
            Ok(frame) => app.editor.context_menu(Some(frame)),
            // No fundo, o menu vale para a página clicada (spec PB): a dobra fica no meio do canvas.
            Err(_) if fx < 0.5 => app.editor.context_menu_on_page(PageSide::Left),
            Err(_) => app.editor.context_menu_on_page(PageSide::Right),
        };
        app.refresh_current();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        let mut open = st.borrow_mut();
        *open = Open { entries, sub: None };
        show(&w, &open);
        let ui = w.global::<ContextUi>();
        ui.set_x(x);
        ui.set_y(y);
        ui.set_open(true);
    });
    let ui = window.global::<ContextUi>();
    let w = window.as_weak();
    ui.on_close(move || {
        if let Some(w) = w.upgrade() {
            w.global::<ContextUi>().set_open(false);
        }
    });
    let (a, st, w) = (app.clone(), state, window.as_weak());
    ui.on_pick(move |i| {
        let Some(w) = w.upgrade() else {
            return;
        };
        let action = {
            let open = st.borrow();
            usize::try_from(i)
                .ok()
                .and_then(|i| rows(&open).into_iter().nth(i))
                .map_or(Action::Nothing, |(_, a)| a)
        };
        match action {
            Action::Run(command) => {
                w.global::<ContextUi>().set_open(false);
                crate::input::run_command(&a, command);
            }
            Action::Toggle(title) => {
                let mut open = st.borrow_mut();
                open.sub = if open.sub == Some(title) {
                    None
                } else {
                    Some(title)
                };
                show(&w, &open);
            }
            Action::Nothing => {}
        }
    });
}
