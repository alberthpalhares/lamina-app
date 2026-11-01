//! A seção Foto pela janela (spec AF, seção 3): os controles chamam o editor, que decide; o clique
//! duplo no seletor volta o controle ao neutro (regra 14); a grade fina do recorte some 1 s depois
//! do último giro.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_core::FrameContent;
use lamina_editor::PALETTE;
use lamina_editor::editor::{AdjustChange, AdjustControl};
use lamina_editor::forms::{LookField, mm_text};
use slint::{Color, ComponentHandle, Timer, TimerMode};

use crate::state::App;
use crate::{AppWindow, PhotoUi};

type Shared = Rc<RefCell<App>>;

/// Quanto a grade fina fica depois do último giro (spec AF, seção 4).
const FINE_GRID: Duration = Duration::from_secs(1);

thread_local! {
    static SETTLE: Timer = Timer::default();
}

/// Aplica a mudança e redesenha a lâmina; um giro rearma o fim da grade fina.
fn change(app: &Shared, change: AdjustChange) {
    let turned = matches!(change, AdjustChange::Angle(_) | AdjustChange::Turn(_));
    let mut a = app.borrow_mut();
    if a.editor.adjust(change) {
        // A miniatura da faixa também mostra a foto ajustada.
        a.refresh_strip();
        a.refresh_current();
    }
    drop(a);
    if turned {
        settle_later(app);
    }
}

/// Clique duplo no seletor: o controle volta ao neutro numa edição própria (spec AF, regra 14).
fn reset(app: &Shared, control: AdjustControl) {
    let mut a = app.borrow_mut();
    if a.editor.reset_adjust(control) {
        a.refresh_strip();
        a.refresh_current();
    }
    drop(a);
    if control == AdjustControl::Angle {
        settle_later(app);
    }
}

/// Depois de um giro: a grade fina some 1 s depois, se nada girar de novo.
pub fn settle_later(app: &Shared) {
    let a = app.clone();
    SETTLE.with(|t| {
        t.start(TimerMode::SingleShot, FINE_GRID, move || {
            let mut app = a.borrow_mut();
            app.editor.angle_settled();
            app.refresh_current();
        });
    });
}

/// Um número da tela em inteiro (os controles vão de −100 a 100, ou de −1800 a 1800 décimos).
fn whole(v: f32) -> i32 {
    // Os controles ficam entre −1800 e 1800, então cabe em i32.
    #[allow(clippy::cast_possible_truncation)]
    let n = v.round() as i32;
    n
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PhotoUi>();
    let a = app.clone();
    ui.on_angle_moved(move |v| change(&a, AdjustChange::Angle(whole(v * 10.0))));
    let a = app.clone();
    ui.on_angle_step(move |t| change(&a, AdjustChange::Turn(t)));
    let a = app.clone();
    ui.on_angle_zero(move || change(&a, AdjustChange::Angle(0)));
    let a = app.clone();
    ui.on_brightness_moved(move |v| change(&a, AdjustChange::Brightness(whole(v))));
    let a = app.clone();
    ui.on_contrast_moved(move |v| change(&a, AdjustChange::Contrast(whole(v))));
    let a = app.clone();
    ui.on_temperature_moved(move |v| change(&a, AdjustChange::Temperature(whole(v))));
    let a = app.clone();
    ui.on_tint_moved(move |v| change(&a, AdjustChange::Tint(whole(v))));
    let a = app.clone();
    ui.on_bw_toggled(move |on| change(&a, AdjustChange::Bw(on)));
    let a = app.clone();
    ui.on_restore(move || change(&a, AdjustChange::Restore));
    let a = app.clone();
    ui.on_released(move || a.borrow_mut().editor.end_adjust());
    let a = app.clone();
    ui.on_angle_reset(move || reset(&a, AdjustControl::Angle));
    let a = app.clone();
    ui.on_brightness_reset(move || reset(&a, AdjustControl::Brightness));
    let a = app.clone();
    ui.on_contrast_reset(move || reset(&a, AdjustControl::Contrast));
    let a = app.clone();
    ui.on_temperature_reset(move || reset(&a, AdjustControl::Temperature));
    let a = app.clone();
    ui.on_tint_reset(move || reset(&a, AdjustControl::Tint));
    connect_sliders(&ui, app);
    connect_style(&ui, app);
    let (a, w) = (app.clone(), window.as_weak());
    ui.on_angle_accepted(move || {
        let Some(w) = w.upgrade() else {
            return;
        };
        let text = w.global::<PhotoUi>().get_angle();
        let result = a.borrow_mut().editor.set_angle_text(&text);
        typed(&a, &w, result);
        settle_later(&a);
    });
    let (a, w) = (app.clone(), window.as_weak());
    ui.on_scale_accepted(move || {
        let Some(w) = w.upgrade() else {
            return;
        };
        let text = w.global::<PhotoUi>().get_scale();
        let result = a.borrow_mut().editor.set_scale_text(&text);
        typed(&a, &w, result);
    });
}

/// A escala pelo controle e os botões de 90° (spec CD): a lâmina muda na hora.
fn connect_sliders(ui: &PhotoUi, app: &Shared) {
    let a = app.clone();
    ui.on_scale_moved(move |pos| styled(&a, |e| e.set_scale_position(f64::from(pos))));
    let a = app.clone();
    ui.on_scale_reset(move || styled(&a, lamina_editor::Editor::reset_scale));
    let a = app.clone();
    ui.on_quarter(move |way| {
        styled(&a, |e| e.turn_quarter(way));
        settle_later(&a);
    });
}

/// Opacidade, borda, cor da borda e cantos (spec OP): a lâmina muda na hora.
fn connect_style(ui: &PhotoUi, app: &Shared) {
    let a = app.clone();
    ui.on_opacity_moved(move |v| {
        let percent = u8::try_from(whole(v).clamp(0, 100)).unwrap_or(100);
        styled(&a, |e| e.set_photo_opacity(percent));
    });
    let a = app.clone();
    ui.on_opacity_reset(move || styled(&a, lamina_editor::Editor::reset_opacity));
    let a = app.clone();
    ui.on_border_step(move |d| styled(&a, |e| e.step_photo_style(LookField::Border, d)));
    let a = app.clone();
    ui.on_radius_step(move |d| styled(&a, |e| e.step_photo_style(LookField::Radius, d)));
    let a = app.clone();
    ui.on_border_pick(move |i| {
        let Some((_, c)) = usize::try_from(i).ok().and_then(|i| PALETTE.get(i)) else {
            return;
        };
        styled(&a, |e| e.pick_photo_border_color(*c));
    });
}

/// Uma mudança da aparência da foto: a faixa e a lâmina redesenham.
fn styled(app: &Shared, change: impl FnOnce(&mut lamina_editor::Editor) -> bool) {
    let mut a = app.borrow_mut();
    if change(&mut a.editor) {
        a.refresh_strip();
        a.refresh_current();
    }
}

/// Um campo digitado: a mensagem do editor (já humana) ou o redesenho.
fn typed(app: &Shared, w: &AppWindow, result: Result<bool, &'static str>) {
    let ui = w.global::<PhotoUi>();
    match result {
        Err(message) => ui.set_error(message.into()), // texto já humano
        Ok(_) => {
            ui.set_error("".into());
            let mut a = app.borrow_mut();
            a.refresh_strip();
            a.refresh_current();
        }
    }
}

/// Os valores da foto selecionada nos controles.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PhotoUi>();
    let Some(adjust) = app.editor.photo_adjust() else {
        ui.set_has_photo(false);
        return;
    };
    ui.set_has_photo(true);
    ui.set_can_turn(app.editor.can_adjust(AdjustChange::Angle(0)));
    let degrees = adjust.degrees();
    ui.set_angle(format!("{}°", lamina_editor::frame_form::decimal(degrees)).into());
    #[allow(clippy::cast_possible_truncation)]
    let to_f32 = |v: f64| v as f32;
    ui.set_angle_value(to_f32(degrees));
    ui.set_brightness(to_f32(f64::from(adjust.brightness)));
    ui.set_contrast(to_f32(f64::from(adjust.contrast)));
    ui.set_temperature(to_f32(f64::from(adjust.temperature)));
    ui.set_tint(to_f32(f64::from(adjust.tint)));
    ui.set_scale_position(to_f32(app.editor.scale_position().unwrap_or(0.0)));
    ui.set_bw(adjust.bw);
    ui.set_scale(
        app.editor
            .photo_scale()
            .map(lamina_editor::frame_form::decimal)
            .unwrap_or_default()
            .into(),
    );
    ui.set_note(shared_note(app).into());
    if let Some(style) = app.editor.photo_style() {
        ui.set_size(style.size.into());
        ui.set_opacity(f32::from(style.opacity.get()));
        ui.set_border(mm_text(style.border.width.get()).into());
        let c = style.border.color;
        ui.set_border_color(Color::from_rgb_u8(c.r, c.g, c.b));
        ui.set_radius(mm_text(style.radius.get()).into());
    }
    ui.set_style_note(app.editor.photo_style_note().into());
}

/// "Vale para esta foto em todos os quadros" quando a foto está em mais de um quadro do álbum.
fn shared_note(app: &App) -> &'static str {
    let (Some(s), Some(frame)) = (app.editor.session(), app.editor.selected_frame()) else {
        return "";
    };
    let project = s.album();
    let Some(FrameContent::Photo { photo, .. }) = project
        .spreads
        .get(s.current())
        .and_then(|sp| sp.frames.get(frame))
        .map(|f| &f.content)
    else {
        return "";
    };
    let uses = project
        .spreads
        .iter()
        .flat_map(|sp| &sp.frames)
        .filter(|f| matches!(&f.content, FrameContent::Photo { photo: p, .. } if p == photo))
        .count();
    if uses > 1 {
        "Vale para esta foto em todos os quadros."
    } else {
        ""
    }
}
