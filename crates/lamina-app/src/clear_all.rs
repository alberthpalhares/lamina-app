//! Limpar as lâminas pela janela (spec LL): a pergunta com os números do álbum e, no "sim", o
//! comando do editor.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lamina_editor::{Command, LEARNED_QUESTION, LEARNED_QUESTION_DETAIL, NOTHING_LEARNED};
use slint::ComponentHandle;

use crate::state::App;
use crate::{AppWindow, ClearUi};

type Shared = Rc<RefCell<App>>;

thread_local! {
    // O comando que a pergunta aberta vai rodar no "sim".
    static ASKED: Cell<Option<Command>> = const { Cell::new(None) };
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Abre a pergunta de "Tirar as fotos de todas as lâminas…" ou "Apagar todas as lâminas…".
pub fn ask(app: &App, command: Command) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let (photos, spreads) = app.editor.clear_counts();
    let spreads_text = plural(spreads, "lâmina", "lâminas");
    let (title, message, action) = if command == Command::DeleteAllSpreads {
        (
            format!("Apagar as {spreads_text} do álbum?"),
            "As fotos continuam importadas, como não usadas. Ctrl+Z desfaz.",
            "Apagar as lâminas",
        )
    } else {
        (
            format!(
                "Tirar as {} de todas as {spreads_text}?",
                plural(photos, "foto", "fotos")
            ),
            "Os quadros ficam vazios e as fotos voltam para as não usadas. Ctrl+Z desfaz.",
            "Tirar as fotos",
        )
    };
    ASKED.set(Some(command));
    let ui = w.global::<ClearUi>();
    ui.set_title(title.into());
    ui.set_message(message.into());
    ui.set_action(action.into());
    ui.set_show(true);
}

/// "Apagar o aprendizado de layouts…": a mesma caixa; sem nada aprendido, o aviso no lugar dela (spec
/// AL, regra 18).
pub fn ask_learned(app: &mut App) {
    if app.editor.nothing_learned() {
        NOTHING_LEARNED.clone_into(&mut app.notice);
        app.refresh_current();
        return;
    }
    let Some(w) = app.window.upgrade() else {
        return;
    };
    ASKED.set(Some(Command::DeleteLearned));
    let ui = w.global::<ClearUi>();
    ui.set_title(LEARNED_QUESTION.into());
    ui.set_message(LEARNED_QUESTION_DETAIL.into());
    ui.set_action("Apagar".into());
    ui.set_show(true);
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.global::<ClearUi>().on_confirmed(move || {
        let Some(command) = ASKED.take() else {
            return;
        };
        let mut app = a.borrow_mut();
        if app.editor.run(command) {
            app.refresh_all();
        }
    });
}
