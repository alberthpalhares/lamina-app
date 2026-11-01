//! A aba Quadro pela janela (spec TX): copia as medidas e o texto do quadro selecionado para a
//! tela e liga os campos às edições do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_core::{TextAlign, TextRotation, TextVAlign};
use lamina_editor::editor::TextFocus;
use lamina_editor::frame_form::{parse_frame, parse_size, parse_spacing};
use lamina_editor::{PALETTE, TextChange};
use slint::{Color, ComponentHandle, SharedString};

use crate::state::App;
use crate::{AppWindow, FrameUi};

type Shared = Rc<RefCell<App>>;

/// A aba Quadro do quadro selecionado (ou vazia). Quando o editor pede (clique, T, duplo clique,
/// "Editar texto"), a aba abre e, se for o caso, o cursor vai para o texto.
pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    fill(app, &w);
    match app.editor.take_text_focus() {
        Some(TextFocus::Tab) => w.set_panel_tab(FRAME_TAB),
        Some(TextFocus::Cursor) => {
            w.set_panel_tab(FRAME_TAB);
            let ui = w.global::<FrameUi>();
            ui.set_focus_count(ui.get_focus_count() + 1);
        }
        None => {}
    }
}

/// A aba Quadro no painel da direita.
const FRAME_TAB: i32 = 4;

fn fill(app: &App, w: &AppWindow) {
    let ui = w.global::<FrameUi>();
    let Some(form) = app.editor.frame_form() else {
        ui.set_has_frame(false);
        ui.set_is_text(false);
        return;
    };
    ui.set_has_frame(true);
    ui.set_x(form.x.into());
    ui.set_y(form.y.into());
    ui.set_w(form.width.into());
    ui.set_h(form.height.into());
    ui.set_unit(form.unit.into());
    ui.set_rect_error("".into());
    let Some(t) = form.text else {
        ui.set_is_text(false);
        return;
    };
    ui.set_is_text(true);
    // Reescrever o mesmo texto tiraria o cursor do lugar enquanto o fotógrafo digita.
    if ui.get_text() != t.text.as_str() {
        ui.set_text(t.text.into());
    }
    ui.set_font_missing(t.font_label != t.font);
    ui.set_font_label(t.font_label.into());
    ui.set_size(t.size.into());
    ui.set_size_error("".into());
    ui.set_bold(t.bold);
    ui.set_italic(t.italic);
    ui.set_can_bold(t.can_bold);
    ui.set_can_italic(t.can_italic);
    ui.set_color(Color::from_rgb_u8(t.color.r, t.color.g, t.color.b));
    ui.set_align(i32::try_from(t.align).unwrap_or(1));
    ui.set_valign(i32::try_from(t.valign).unwrap_or(1));
    ui.set_rotation(i32::try_from(t.rotation).unwrap_or(0));
    ui.set_spacing(t.spacing.into());
    ui.set_spacing_error("".into());
}

pub(crate) fn change(app: &Shared, change: TextChange) {
    let mut app = app.borrow_mut();
    if app.editor.change_text(change) {
        app.refresh_strip();
        app.refresh_current();
    }
}

fn align(i: i32) -> TextAlign {
    match i {
        0 => TextAlign::Left,
        2 => TextAlign::Right,
        _ => TextAlign::Center,
    }
}

fn valign(i: i32) -> TextVAlign {
    match i {
        0 => TextVAlign::Top,
        2 => TextVAlign::Bottom,
        _ => TextVAlign::Middle,
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<FrameUi>();
    let a = app.clone();
    let w = window.as_weak();
    ui.on_rect_accepted(move || {
        let Some(w) = w.upgrade() else {
            return;
        };
        let ui = w.global::<FrameUi>();
        let mut app = a.borrow_mut();
        let Some(project) = app.editor.session().map(|s| s.project_shared()) else {
            return;
        };
        let fields = [ui.get_x(), ui.get_y(), ui.get_w(), ui.get_h()];
        let unit = app.editor.strips().unit;
        match parse_frame(&project, fields.each_ref().map(SharedString::as_str), unit) {
            Ok(rect) => {
                if app.editor.set_frame_mm(rect) {
                    app.refresh_all();
                }
            }
            Err(e) => ui.set_rect_error(e.into()),
        }
    });
    let a = app.clone();
    ui.on_text_edited(move |t| change(&a, TextChange::Text(t.into())));
    let a = app.clone();
    let w = window.as_weak();
    ui.on_size_accepted(move |t| match parse_size(&t) {
        Ok(v) => change(&a, TextChange::Size(v)),
        Err(e) => {
            if let Some(w) = w.upgrade() {
                w.global::<FrameUi>().set_size_error(e.into());
            }
        }
    });
    let a = app.clone();
    ui.on_bold_toggled(move |on| change(&a, TextChange::Bold(on)));
    let a = app.clone();
    ui.on_italic_toggled(move |on| change(&a, TextChange::Italic(on)));
    let a = app.clone();
    ui.on_color_picked(move |i| {
        if let Some((_, c)) = usize::try_from(i).ok().and_then(|i| PALETTE.get(i)) {
            change(&a, TextChange::Color(*c));
        }
    });
    let a = app.clone();
    ui.on_align_picked(move |i| change(&a, TextChange::Align(align(i))));
    let a = app.clone();
    ui.on_rotation_picked(move |i| {
        let rotation = match i {
            1 => TextRotation::Up,
            2 => TextRotation::Down,
            _ => TextRotation::Zero,
        };
        change(&a, TextChange::Rotation(rotation));
    });
    let a = app.clone();
    ui.on_valign_picked(move |i| change(&a, TextChange::VAlign(valign(i))));
    let a = app.clone();
    let w = window.as_weak();
    ui.on_spacing_accepted(move |t| match parse_spacing(&t) {
        Ok(v) => change(&a, TextChange::LineSpacing(v)),
        Err(e) => {
            if let Some(w) = w.upgrade() {
                w.global::<FrameUi>().set_spacing_error(e.into());
            }
        }
    });
}
