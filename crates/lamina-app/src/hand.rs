//! Quadros à mão pela janela (spec TX): as alças e o arrasto do texto viram o gesto do editor,
//! em mm, e as guias que pegaram no ímã voltam como linhas. As decisões são do editor.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lamina_core::Project;
use lamina_editor::Grip;
use slint::ModelRc;

use crate::convert::model;
use crate::state::App;
use crate::{AppWindow, BoxData};

type Shared = Rc<RefCell<App>>;

/// Distância do ímã na tela, em pixels (regra 7).
const MAGNET_PX: f64 = 8.0;

thread_local! {
    // Onde o gesto começou (pixels da janela) e a largura da lâmina em pixels.
    static START: Cell<Option<(f32, f32, f32)>> = const { Cell::new(None) };
}

/// A alça pelo número da tela: 0 = mover; 1 a 8 a partir do canto de cima à esquerda, no
/// sentido do relógio.
fn grip_of(n: i32) -> Grip {
    let resize = |left, right, top, bottom| Grip::Resize {
        left,
        right,
        top,
        bottom,
    };
    match n {
        1 => resize(true, false, true, false),
        2 => resize(false, false, true, false),
        3 => resize(false, true, true, false),
        4 => resize(false, true, false, false),
        5 => resize(false, true, false, true),
        6 => resize(false, false, false, true),
        7 => resize(true, false, false, true),
        8 => resize(true, false, false, false),
        _ => Grip::Move,
    }
}

/// Milímetros por pixel numa lâmina desenhada com `width_px` pixels (com zoom, a largura
/// desenhada cresce e o mesmo arrasto anda menos; spec EG, regra 14).
fn mm_per_px(app: &App, width_px: f32) -> Option<f64> {
    lamina_editor::group::mm_per_px(app.editor.session()?.project(), f64::from(width_px))
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_frame_grab(move |frame, grip, x, y, width| {
        let mut app = a.borrow_mut();
        // No modo organizar, ou na edição comum com seleção múltipla (spec GJ), as alças são as
        // da caixa do grupo (spec EG, regra 5).
        let group = app.editor.selected_frames().len() > 1;
        let grabbed = usize::try_from(frame).is_ok_and(|f| {
            if group {
                app.editor.group_grab(f, grip_of(grip))
            } else {
                app.editor.frame_grab(f, grip_of(grip))
            }
        });
        if grabbed {
            START.set(Some((x, y, width)));
            // Só a lâmina: o painel muda no fim do gesto, sem mexer no tamanho da tela agora.
            crate::crop::refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_frame_drag(move |x, y, alt, shift| {
        let mut app = a.borrow_mut();
        let Some((x0, y0, width)) = START.get() else {
            return;
        };
        let Some(k) = mm_per_px(&app, width) else {
            return;
        };
        let (dx, dy) = (f64::from(x - x0) * k, f64::from(y - y0) * k);
        let moved = if app.editor.group_moving() {
            app.editor.group_drag((dx, dy), !alt, MAGNET_PX * k, shift)
        } else {
            app.editor.frame_drag(dx, dy, !alt, MAGNET_PX * k)
        };
        if moved {
            crate::crop::refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_organize_press(move |frame, ctrl, shift, x, y, width| {
        let mut app = a.borrow_mut();
        let Ok(frame) = usize::try_from(frame) else {
            return;
        };
        // Ctrl ou Shift: só soma ou tira da seleção; sem eles, segura o grupo.
        if ctrl || shift {
            app.editor.organize_click(frame, ctrl, shift);
            app.refresh_current();
        } else if app.editor.group_grab(frame, lamina_editor::Grip::Move) {
            START.set(Some((x, y, width)));
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.on_frame_release(move || {
        let mut app = a.borrow_mut();
        START.set(None);
        let released = if app.editor.group_moving() {
            app.editor.group_release()
        } else {
            app.editor.frame_release()
        };
        if released {
            app.refresh_all();
        } else {
            app.refresh_current();
        }
    });
}

/// As linhas do ímã em frações da largura da lâmina, para a tela.
pub fn snap_boxes(app: &App, project: &Project) -> ModelRc<BoxData> {
    let canvas = project.spec.canvas_rect();
    let w = canvas.width.get();
    let h = canvas.height.get() / w;
    // Frações de tela cabem com folga em f32.
    #[allow(clippy::cast_possible_truncation)]
    let f = |v: f64| v as f32;
    let lines = app.editor.snap_lines().iter().map(|l| {
        if l.vertical {
            let x = (l.at - canvas.x.get()) / w;
            BoxData {
                x: f(x),
                y: 0.0,
                w: 0.0,
                h: f(h),
            }
        } else {
            let y = (l.at - canvas.y.get()) / w;
            BoxData {
                x: 0.0,
                y: f(y),
                w: 1.0,
                h: 0.0,
            }
        }
    });
    model(lines.collect())
}
