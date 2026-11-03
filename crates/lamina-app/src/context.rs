//! O menu de contexto pela janela (specs EG e DC): o clique direito ou a tecla Menu pedem o menu ao
//! editor (que ajusta a seleção), a janela mostra as linhas, e o item escolhido roda o comando do
//! registro. O teclado e o lugar do menu são decisão do editor (`context_nav`).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_core::PageSide;
use lamina_editor::Command;
use lamina_editor::context_nav::{
    KeyMenu, MenuCursor, MenuKey, MenuLine, MenuStep, key_menu, menu_box, next_line,
    photo_menu_lines, row_top, submenu_box,
};
use lamina_editor::editor::ContextEntry;
use lamina_editor::reveal::System;
use slint::{ComponentHandle, Timer, TimerMode};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, ContextRow, ContextUi, MenuBoxData, PhotoMenuUi};

type Shared = Rc<RefCell<App>>;

/// Quanto o mouse precisa ficar parado numa linha para o submenu abrir ou fechar (spec DC, regra 8).
const SETTLE: Duration = Duration::from_millis(200);

/// O que uma linha faz ao ser escolhida.
#[derive(Clone, Copy)]
enum Action {
    Run(Command),
    Remove,
    Reveal,
    Sub,
    Nothing,
}

/// Uma linha do menu: o desenho, o que ela faz e, num submenu, as linhas dele.
#[derive(Clone)]
struct Line {
    row: ContextRow,
    walk: MenuLine,
    action: Action,
    sub: Vec<Line>,
}

/// O menu aberto.
#[derive(Default)]
struct Open {
    lines: Vec<Line>,
    cursor: MenuCursor,
}

impl Open {
    fn walk(&self) -> Vec<MenuLine> {
        self.lines.iter().map(|l| l.walk).collect()
    }

    fn sub_lines(&self) -> &[Line] {
        self.cursor
            .open
            .and_then(|i| self.lines.get(i))
            .map_or(&[], |l| l.sub.as_slice())
    }

    fn sub_walk(&self) -> Vec<MenuLine> {
        self.sub_lines().iter().map(|l| l.walk).collect()
    }
}

fn separator() -> Line {
    Line {
        row: ContextRow {
            kind: 2,
            ..ContextRow::default()
        },
        walk: MenuLine::Separator,
        action: Action::Nothing,
        sub: Vec::new(),
    }
}

fn plain_item(label: &str, action: Action) -> Line {
    Line {
        row: ContextRow {
            label: label.into(),
            enabled: true,
            ..ContextRow::default()
        },
        walk: MenuLine::Item,
        action,
        sub: Vec::new(),
    }
}

fn line(entry: &ContextEntry) -> Line {
    match entry {
        ContextEntry::Separator => separator(),
        ContextEntry::Item {
            command,
            label,
            shortcut,
            enabled,
            checked,
        } => Line {
            row: ContextRow {
                label: label.as_str().into(),
                shortcut: shortcut.as_str().into(),
                kind: 0,
                enabled: *enabled,
                checked: *checked,
            },
            walk: if *enabled {
                MenuLine::Item
            } else {
                MenuLine::Disabled
            },
            action: Action::Run(*command),
            sub: Vec::new(),
        },
        ContextEntry::Sub { title, items } => Line {
            row: ContextRow {
                label: (*title).into(),
                kind: 1,
                enabled: true,
                ..ContextRow::default()
            },
            walk: MenuLine::Sub,
            action: Action::Sub,
            sub: items.iter().map(line).collect(),
        },
    }
}

fn lines(entries: &[ContextEntry]) -> Vec<Line> {
    entries.iter().map(line).collect()
}

/// O menu da foto (spec TF, regra 1): as linhas seguem a seleção e o sistema.
fn photo_lines(app: &App) -> Vec<Line> {
    let [remove, reveal] = photo_menu_lines(app.editor.selection().len(), System::current());
    vec![
        plain_item(&remove, Action::Remove),
        plain_item(&reveal, Action::Reveal),
    ]
}

fn to_i32(i: Option<usize>) -> i32 {
    i.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1)
}

/// Só o que mudou com o teclado ou o mouse: as linhas ficam, e os itens de baixo não se mexem.
fn show_cursor(window: &AppWindow, open: &Open) {
    let ui = window.global::<ContextUi>();
    ui.set_current(to_i32(open.cursor.main));
    ui.set_sub_current(to_i32(open.cursor.sub));
    ui.set_sub_open(to_i32(open.cursor.open));
}

/// Abre o menu com `new` linhas, no ponto da janela.
fn show(window: &AppWindow, open: &mut Open, new: Vec<Line>, at: (f32, f32), by_keyboard: bool) {
    open.lines = new;
    open.cursor = if by_keyboard {
        MenuCursor::by_keyboard(&open.walk())
    } else {
        MenuCursor::default()
    };
    let ui = window.global::<ContextUi>();
    ui.set_rows(model(open.lines.iter().map(|l| l.row.clone()).collect()));
    ui.set_sub_rows(model(Vec::new()));
    show_cursor(window, open);
    ui.set_x(at.0);
    ui.set_y(at.1);
    ui.set_open(true);
}

fn open_sub(window: &AppWindow, open: &mut Open, row: usize, by_keyboard: bool) {
    open.cursor.main = Some(row);
    open.cursor.open = Some(row);
    let ui = window.global::<ContextUi>();
    ui.set_sub_rows(model(
        open.sub_lines().iter().map(|l| l.row.clone()).collect(),
    ));
    ui.set_sub_top(row_top(&open.walk(), row));
    open.cursor.sub = if by_keyboard {
        next_line(&open.sub_walk(), None, true)
    } else {
        None
    };
    show_cursor(window, open);
}

fn close_sub(window: &AppWindow, open: &mut Open) {
    open.cursor.open = None;
    open.cursor.sub = None;
    show_cursor(window, open);
}

/// Fecha o menu se estiver aberto: um comando vindo do menu principal muda o que vale (seção 6).
pub fn close(window: &AppWindow) {
    window.global::<ContextUi>().set_open(false);
}

/// Roda o que a linha faz, com o menu já fechado.
fn run(app: &Shared, window: &AppWindow, action: Action) {
    close(window);
    match action {
        Action::Run(command) => crate::input::run_command(app, command),
        Action::Remove => window.global::<PhotoMenuUi>().invoke_remove(),
        Action::Reveal => window.global::<PhotoMenuUi>().invoke_reveal(),
        Action::Sub | Action::Nothing => {}
    }
}

fn press(app: &Shared, window: &AppWindow, state: &RefCell<Open>, key: MenuKey) {
    let action = {
        let mut open = state.borrow_mut();
        let (main, sub) = (open.walk(), open.sub_walk());
        match open.cursor.press(key, &main, &sub) {
            MenuStep::Stay => {
                show_cursor(window, &open);
                None
            }
            MenuStep::OpenSub(row) => {
                open_sub(window, &mut open, row, true);
                None
            }
            MenuStep::CloseSub => {
                close_sub(window, &mut open);
                None
            }
            MenuStep::Run(i) => open.lines.get(i).map(|l| l.action),
            MenuStep::RunSub(i) => open.sub_lines().get(i).map(|l| l.action),
            MenuStep::Close => {
                close(window);
                None
            }
        }
    };
    if let Some(action) = action {
        run(app, window, action);
    }
}

fn key_of(name: &str) -> Option<MenuKey> {
    match name {
        "up" => Some(MenuKey::Up),
        "down" => Some(MenuKey::Down),
        "right" => Some(MenuKey::Right),
        "left" => Some(MenuKey::Left),
        "enter" => Some(MenuKey::Enter),
        "esc" => Some(MenuKey::Esc),
        _ => None,
    }
}

fn to_box(placed: lamina_editor::context_nav::MenuBox) -> MenuBoxData {
    MenuBoxData {
        x: placed.x,
        y: placed.y,
        height: placed.height,
    }
}

/// A tecla Menu ou Shift+F10 (spec DC, seção 3): o menu do que está selecionado, ao lado dele.
fn from_keyboard(app: &Shared, state: &RefCell<Open>) {
    let (target, entries) = {
        let mut app = app.borrow_mut();
        let target = key_menu(
            app.session().is_some(),
            app.grid_focus && !app.editor.organizing() && !app.editor.selection().is_empty(),
            app.editor.selected_frame(),
        );
        let entries = match target {
            Some(KeyMenu::Frame(frame)) => lines(&app.editor.context_menu(Some(frame))),
            Some(KeyMenu::Background) => lines(&app.editor.context_menu_on_page(PageSide::Left)),
            Some(KeyMenu::Photo) => photo_lines(&app),
            None => return,
        };
        app.refresh_current();
        (target, entries)
    };
    let Some(w) = app.borrow().window.upgrade() else {
        return;
    };
    let at = match target {
        Some(KeyMenu::Frame(frame)) => w.invoke_menu_anchor(i32::try_from(frame).unwrap_or(-1)),
        Some(KeyMenu::Photo) => w.invoke_photos_anchor(),
        _ => w.invoke_menu_anchor(-1),
    };
    show(&w, &mut state.borrow_mut(), entries, (at.x, at.y), true);
}

fn connect_opening(window: &AppWindow, app: &Shared, state: &Rc<RefCell<Open>>) {
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
        if let Some(w) = app.window.upgrade() {
            show(&w, &mut st.borrow_mut(), lines(&entries), (x, y), false);
        }
    });
    let (a, st) = (app.clone(), state.clone());
    window.global::<PhotoMenuUi>().on_menu_at(move |x, y| {
        let app = a.borrow();
        if let Some(w) = app.window.upgrade() {
            show(&w, &mut st.borrow_mut(), photo_lines(&app), (x, y), false);
        }
    });
    let (a, st) = (app.clone(), state.clone());
    window.on_context_key(move || from_keyboard(&a, &st));
}

/// O mouse parou numa linha: abre o submenu dela ou fecha o que estiver aberto (spec DC, regra 8).
fn settled(window: &AppWindow, open: &mut Open, row: usize) {
    let is_sub = open.lines.get(row).is_some_and(|l| l.walk == MenuLine::Sub);
    if is_sub && open.cursor.open != Some(row) {
        open_sub(window, open, row, false);
    } else if !is_sub && open.cursor.open.is_some() {
        close_sub(window, open);
    }
}

fn connect_mouse(window: &AppWindow, state: &Rc<RefCell<Open>>) {
    let ui = window.global::<ContextUi>();
    let wait = Rc::new(Timer::default());
    let (st, w, delay) = (state.clone(), window.as_weak(), wait.clone());
    ui.on_hover(move |i| {
        let Some(window) = w.upgrade() else {
            return;
        };
        let mut open = st.borrow_mut();
        let row = usize::try_from(i).ok();
        let walkable = row
            .and_then(|r| open.lines.get(r))
            .is_some_and(|l| !matches!(l.walk, MenuLine::Disabled | MenuLine::Separator));
        // Saindo da linha com um submenu aberto, ela continua marcada.
        open.cursor.main = if walkable { row } else { open.cursor.open };
        show_cursor(&window, &open);
        // Saindo da linha, a espera continua: o mouse a caminho do submenu pode passar por um vão.
        let Some(row) = row else {
            return;
        };
        let (st, w) = (st.clone(), w.clone());
        delay.start(TimerMode::SingleShot, SETTLE, move || {
            if let Some(window) = w.upgrade() {
                settled(&window, &mut st.borrow_mut(), row);
            }
        });
    });
    let (st, w) = (state.clone(), window.as_weak());
    ui.on_hover_sub(move |i| {
        wait.stop();
        let (mut open, Some(w)) = (st.borrow_mut(), w.upgrade()) else {
            return;
        };
        let row = usize::try_from(i).ok();
        let walkable = row
            .and_then(|r| open.sub_lines().get(r))
            .is_some_and(|l| l.walk == MenuLine::Item);
        open.cursor.main = open.cursor.open;
        open.cursor.sub = if walkable { row } else { None };
        show_cursor(&w, &open);
    });
}

fn connect_place(window: &AppWindow) {
    let ui = window.global::<ContextUi>();
    ui.on_place(|x, y, w, h, room_w, room_h| to_box(menu_box((x, y), (w, h), (room_w, room_h))));
    ui.on_place_sub(|main_x, main_w, row_y, w, h, room_w, room_h| {
        to_box(submenu_box(
            (main_x, main_w),
            row_y,
            (w, h),
            (room_w, room_h),
        ))
    });
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let state = Rc::new(RefCell::new(Open::default()));
    connect_opening(window, app, &state);
    connect_mouse(window, &state);
    connect_place(window);
    let ui = window.global::<ContextUi>();
    let w = window.as_weak();
    ui.on_close(move || {
        if let Some(w) = w.upgrade() {
            close(&w);
        }
    });
    let (a, st, w) = (app.clone(), state.clone(), window.as_weak());
    ui.on_key(move |name| {
        if let (Some(key), Some(w)) = (key_of(&name), w.upgrade()) {
            press(&a, &w, &st, key);
        }
    });
    let (a, st, w) = (app.clone(), state.clone(), window.as_weak());
    ui.on_pick(move |i| {
        let Some(w) = w.upgrade() else {
            return;
        };
        let Some(row) = usize::try_from(i).ok() else {
            return;
        };
        let action = {
            let mut open = st.borrow_mut();
            match open.lines.get(row).map(|l| l.action) {
                // O clique abre na hora; clicar de novo fecha.
                Some(Action::Sub) => {
                    if open.cursor.open == Some(row) {
                        close_sub(&w, &mut open);
                    } else {
                        open_sub(&w, &mut open, row, false);
                    }
                    None
                }
                other => other,
            }
        };
        if let Some(action) = action {
            run(&a, &w, action);
        }
    });
    let (a, st, w) = (app.clone(), state, window.as_weak());
    ui.on_pick_sub(move |i| {
        let Some(w) = w.upgrade() else {
            return;
        };
        let action = usize::try_from(i)
            .ok()
            .and_then(|row| st.borrow().sub_lines().get(row).map(|l| l.action));
        if let Some(action) = action {
            run(&a, &w, action);
        }
    });
}
