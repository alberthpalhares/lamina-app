//! O painel de aparência (spec AP): copia a aparência da lâmina para a tela e liga os botões às
//! edições do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_core::PageSide;
use lamina_editor::forms::{LookField, mm_text};
use lamina_editor::new_project::picker_color;
use lamina_editor::{BAD_COLOR, PALETTE, color_hex, parse_color};
use slint::{Color, ComponentHandle};

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, LookUi, Swatch};

type Shared = Rc<RefCell<App>>;

fn color(c: lamina_core::Rgb) -> Color {
    Color::from_rgb_u8(c.r, c.g, c.b)
}

pub fn refresh(app: &App) {
    let (Some(w), Some(look)) = (app.window.upgrade(), app.editor.look()) else {
        return;
    };
    let palette: Vec<Swatch> = PALETTE
        .iter()
        .map(|(name, c)| Swatch {
            c: color(*c),
            name: (*name).into(),
        })
        .collect();
    let ui = w.global::<LookUi>();
    ui.set_palette(model(palette));
    ui.set_gap(mm_text(look.gap.get()).into());
    ui.set_border(mm_text(look.border.width.get()).into());
    ui.set_radius(mm_text(look.radius.get()).into());
    ui.set_split(look.split);
    ui.set_left(color(look.left));
    ui.set_right(color(look.right));
    ui.set_border_color(color(look.border.color));
    ui.set_only_frame(look.only_frame);
    let manual = app.editor.session().is_some_and(|s| {
        s.project()
            .spreads
            .get(s.current())
            .is_some_and(|sp| sp.manual)
    });
    ui.set_manual(manual);
    ui.set_hex(color_hex(look.left).into());
    ui.set_hex_right(color_hex(look.right).into());
}

/// O lado que a tela pediu: 0, 1 ou -1 para a lâmina toda.
fn page_side(side: i32) -> Option<PageSide> {
    match side {
        0 => Some(PageSide::Left),
        1 => Some(PageSide::Right),
        _ => None,
    }
}

/// Depois de uma edição: tudo na tela, e o aviso do editor (espaço reduzido), se houver.
fn after(app: &mut App, notice: Option<String>) {
    if let Some(n) = notice {
        app.notice = n;
    }
    app.refresh_all();
}

/// Um clique em − ou + (espaço, borda ou cantos).
fn step(app: &Shared, field: LookField, delta: i32) {
    let mut app = app.borrow_mut();
    let notice = app.editor.step_look(field, delta);
    after(&mut app, notice);
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<LookUi>();
    let a = app.clone();
    ui.on_gap_step(move |d| step(&a, LookField::Gap, d));
    let a = app.clone();
    ui.on_border_step(move |d| step(&a, LookField::Border, d));
    let a = app.clone();
    ui.on_radius_step(move |d| step(&a, LookField::Radius, d));
    let a = app.clone();
    ui.on_background(move |i, side| {
        let mut app = a.borrow_mut();
        let Some((_, c)) = usize::try_from(i).ok().and_then(|i| PALETTE.get(i)) else {
            return;
        };
        app.editor.set_background(*c, page_side(side));
        after(&mut app, None);
    });
    let a = app.clone();
    ui.on_background_at(move |x, y, side| {
        let mut app = a.borrow_mut();
        let c = picker_color(f64::from(x), f64::from(y));
        app.editor.set_background(c, page_side(side));
        after(&mut app, None);
    });
    let a = app.clone();
    ui.on_background_hex(move |text, side| {
        let mut app = a.borrow_mut();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        match parse_color(&text) {
            Some(c) => {
                w.global::<LookUi>().set_hex_error("".into());
                app.editor.set_background(c, page_side(side));
                after(&mut app, None);
            }
            None => w.global::<LookUi>().set_hex_error(BAD_COLOR.into()),
        }
    });
    let a = app.clone();
    ui.on_split_changed(move |on| {
        let mut app = a.borrow_mut();
        let Some(look) = app.editor.look() else {
            return;
        };
        let side = on.then_some(PageSide::Right);
        app.editor.set_background(look.left, side);
        after(&mut app, None);
    });
    let a = app.clone();
    ui.on_border_pick(move |i| {
        let mut app = a.borrow_mut();
        let Some((_, c)) = usize::try_from(i).ok().and_then(|i| PALETTE.get(i)) else {
            return;
        };
        app.editor.pick_border_color(*c);
        after(&mut app, None);
    });
    let a = app.clone();
    ui.on_apply_all(move || {
        let mut app = a.borrow_mut();
        app.editor.style_all();
        after(&mut app, None);
    });
}
