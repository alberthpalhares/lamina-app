//! O modo organizar pela janela (spec EG): os cliques com Ctrl e Shift, o laço, o "Voltar" e o
//! `OrganizeUi` com o título, a seleção e a caixa do grupo; o zoom, a vista, a lupa e o tamanho
//! real (spec ZR). As decisões são do editor.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::editor::{Field, PaletteTool};
use lamina_editor::marks::mark_views;
use lamina_editor::rulers::{Tick, rulers};
use lamina_editor::{Area, Command, PxRect, spread_view};
use slint::ComponentHandle;

use crate::convert::{boxed, model};
use crate::state::App;
use crate::{AppWindow, MarkData, OrganizeUi, TickData};

type Shared = Rc<RefCell<App>>;

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_frame_clicked(move |i, ctrl, shift| {
        let Ok(frame) = usize::try_from(i) else {
            return;
        };
        a.borrow_mut().timed(
            "Selecionar quadro",
            |app| {
                app.editor.organize_click(frame, ctrl, shift);
                true
            },
            App::refresh_current,
        );
    });
    let a = app.clone();
    window.on_lasso_done(move |x0, y0, x1, y1, shift| {
        let mut app = a.borrow_mut();
        let p = |x: f32, y: f32| (f64::from(x), f64::from(y));
        app.editor.lasso(p(x0, y0), p(x1, y1), shift);
        app.refresh_current();
    });
    let ui = window.global::<OrganizeUi>();
    let a = app.clone();
    ui.on_done(move || crate::input::run_command(&a, Command::Organize));
    let a = app.clone();
    ui.on_command(move |name| {
        if let Some(command) = Command::from_name(&name) {
            crate::input::run_command(&a, command);
        }
    });
    let a = app.clone();
    ui.on_field_accepted(move |i| field_accepted(&a, i));
    connect_zoom(&ui, app);
    let a = app.clone();
    ui.on_draw_done(move |tool, x0, y0, x1, y1| {
        let p = |x: f32, y: f32| (f64::from(x), f64::from(y));
        let mut app = a.borrow_mut();
        app.editor.draw_frame(tool == 2, p(x0, y0), p(x1, y1));
        // Quadro e Texto voltam a Selecionar depois de desenhar (spec ZR, regra 2).
        app.editor.set_tool(PaletteTool::Select);
        app.refresh_current();
    });
}

fn ticks(list: Vec<Tick>) -> Vec<TickData> {
    // Frações de tela cabem com folga em f32.
    #[allow(clippy::cast_possible_truncation)]
    let f = |v: f64| v as f32;
    list.into_iter()
        .map(|t| TickData {
            at: f(t.at),
            label: t.label.into(),
            major: t.major,
        })
        .collect()
}

/// A lupa, o Ctrl+roda, a mão e a largura de Ajustar (spec ZR, regras 4 a 7).
fn connect_zoom(ui: &OrganizeUi, app: &Shared) {
    let px = |v: f32| f64::from(v);
    let a = app.clone();
    ui.on_loupe(move |alt, x, y| {
        let mut app = a.borrow_mut();
        if app
            .editor
            .zoom_step(!alt, Some((px(x), px(y))), true)
            .is_some()
        {
            app.refresh_current();
        }
    });
    let a = app.clone();
    ui.on_wheel(move |up, x, y| {
        let mut app = a.borrow_mut();
        if app
            .editor
            .zoom_step(up, Some((px(x), px(y))), false)
            .is_some()
        {
            app.refresh_current();
        }
    });
    let a = app.clone();
    ui.on_pan_done(move || {
        let mut app = a.borrow_mut();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        let ui = w.global::<OrganizeUi>();
        app.editor
            .set_view_pan(px(ui.get_pan_x()), px(ui.get_pan_y()));
    });
    let a = app.clone();
    ui.on_fit_changed(move || {
        let mut app = a.borrow_mut();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        app.editor
            .set_fit_width(px(w.global::<OrganizeUi>().get_fit_width()));
        // O 1:1 é recalculado: redimensionar a janela continua no tamanho real (regra 3).
        if app.editor.organizing() && app.editor.zoom() == lamina_editor::editor::Zoom::Real {
            app.refresh_current();
        }
    });
}

/// O zoom, a vista, a ferramenta e as réguas da lâmina em destaque, como o editor guarda; a
/// duração vem da última mudança (a saída do modo também anima).
fn view(app: &App, ui: &OrganizeUi) {
    #[allow(clippy::cast_possible_truncation)]
    let f = |v: f64| v as f32;
    ui.set_view_ms(i32::try_from(app.editor.view_ms()).unwrap_or(0));
    ui.set_zoom(f(app.editor.view_zoom()));
    let (x, y) = app.editor.view_pan();
    ui.set_pan_x(f(x));
    ui.set_pan_y(f(y));
    ui.set_tool(app.editor.tool().index());
    if !app.editor.organizing() {
        ui.set_hand(false);
        return;
    }
    if let Some(project) = app.editor.view_project() {
        let r = rulers(&project, app.editor.strips().unit);
        ui.set_ticks_x(model(ticks(r.xs)));
        ui.set_ticks_y(model(ticks(r.ys)));
    }
}

/// Um campo da seleção digitado (Enter): o editor decide; texto inválido mostra a mensagem.
fn field_accepted(app: &Shared, index: i32) {
    let mut app = app.borrow_mut();
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<OrganizeUi>();
    let (field, text) = match index {
        0 => (Field::X, ui.get_gx()),
        1 => (Field::Y, ui.get_gy()),
        2 => (Field::Width, ui.get_gw()),
        _ => (Field::Height, ui.get_gh()),
    };
    match app.editor.set_group_field(field, &text, ui.get_keep()) {
        // Mensagem fixa do editor, já em linguagem de fotógrafo.
        Err(message) => ui.set_field_error(message.into()), // texto já humano
        Ok(_) => {
            ui.set_field_error("".into());
            app.refresh_current();
        }
    }
}

/// O título, a seleção e a caixa do grupo do modo organizar.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<OrganizeUi>();
    ui.set_on(app.editor.organizing());
    ui.set_title(app.editor.organize_title().into());
    ui.set_count(app.editor.selection_text().into());
    let rects = selected_rects(app);
    let group = rects.iter().copied().reduce(union);
    ui.set_has_group(rects.len() > 1);
    ui.set_group(group.map(boxed).unwrap_or_default());
    ui.set_boxes(model(rects.into_iter().map(boxed).collect()));
    ui.set_marks(model(marks(app)));
    let [x, y, w, h] = app.editor.group_fields();
    ui.set_gx(x.into());
    ui.set_gy(y.into());
    ui.set_gw(w.into());
    ui.set_gh(h.into());
    ui.set_unit(app.editor.strips().unit.label().into());
    ui.set_hint(app.editor.distribute_hint().into());
    view(app, &ui);
    crate::palette::refresh(app);
}

/// As marcas do ímã do gesto, na unidade da tela.
fn marks(app: &App) -> Vec<MarkData> {
    let Some(project) = app.editor.view_project() else {
        return Vec::new();
    };
    // Frações de tela cabem com folga em f32.
    #[allow(clippy::cast_possible_truncation)]
    let f = |v: f64| v as f32;
    mark_views(
        &project,
        app.editor.magnet_marks(),
        app.editor.strips().unit,
    )
    .into_iter()
    .map(|m| MarkData {
        line: boxed(m.line),
        text: m.text.into(),
        tx: f(m.at.0),
        ty: f(m.at.1),
    })
    .collect()
}

/// Os retângulos dos quadros selecionados, em frações da largura da lâmina.
fn selected_rects(app: &App) -> Vec<PxRect> {
    let (Some(s), Some(project)) = (app.session(), app.editor.view_project()) else {
        return Vec::new();
    };
    if !app.editor.organizing() {
        return Vec::new();
    }
    // Com o gesto do grupo aplicado: as marcas acompanham o mouse.
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    let Some(view) = spread_view(&project, s.current(), area, &[]) else {
        return Vec::new();
    };
    app.editor
        .selected_frames()
        .into_iter()
        .filter_map(|f| view.frames.get(f).map(|v| v.rect))
        .collect()
}

/// A caixa que envolve os retângulos (vazia sem nenhum).
pub fn union_all(rects: &[PxRect]) -> PxRect {
    rects.iter().copied().reduce(union).unwrap_or_default()
}

fn union(a: PxRect, b: PxRect) -> PxRect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    PxRect {
        x,
        y,
        width: (a.x + a.width).max(b.x + b.width) - x,
        height: (a.y + a.height).max(b.y + b.height) - y,
    }
}
