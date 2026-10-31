//! Recorte pela janela (spec RC): a lâmina atual com o recorte provisório, a prévia, o DPI
//! efetivo do quadro e os gestos. Durante o arrasto só a lâmina é redesenhada, para acompanhar o
//! mouse; a faixa e o painel atualizam quando o gesto é gravado.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_editor::editor::DoubleClick;
use lamina_editor::motion::{Place, Transition};
use lamina_editor::{
    Area, FrameKind, FrameView, frame_dpi_text, guarded, load_original, photo_extent, spread_view,
};
use slint::{Model, Timer, TimerMode};

use crate::AppWindow;
use crate::convert::{boxed, spread_data};
use crate::state::{App, Msg};

type Shared = Rc<RefCell<App>>;

/// Giro da roda, em pixels, que vale um passo de zoom (um entalhe da roda do mouse).
const WHEEL_NOTCH: f64 = 120.0;

/// Pausa da roda do mouse que fecha o gesto de zoom (regra 4).
const WHEEL_PAUSE: Duration = Duration::from_millis(500);

/// A foto do quadro `frame` da lâmina `index`.
fn photo_in(project: &Project, index: usize, frame: usize) -> Option<PhotoId> {
    match &project.spreads.get(index)?.frames.get(frame)?.content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    }
}

/// No modo recorte, pede a foto original (fora da linha da janela); fora dele, solta (regra 9).
fn sync_original(app: &mut App, project: &Project, index: usize) {
    let wanted = app
        .editor
        .crop_mode()
        .and_then(|f| photo_in(project, index, f));
    match wanted {
        None => {
            app.images.clear_original();
            app.loading = None;
        }
        Some(id) if app.images.original_id() != Some(id) && app.loading != Some(id) => {
            app.loading = Some(id);
            let (project, tx) = (project.clone(), app.tx.clone());
            std::thread::spawn(move || match guarded(|| load_original(&project, id)) {
                // Se a janela já fechou, não há a quem avisar.
                Ok(Some(image)) => {
                    // A cópia para os pixels da janela também fica fora da linha dela (MV, regra 1).
                    let pixels = slint::SharedPixelBuffer::clone_from_slice(
                        image.as_raw(),
                        image.width(),
                        image.height(),
                    );
                    let _ = tx.send(Msg::Original(id, pixels));
                }
                Ok(None) => {}
                Err(problem) => {
                    let _ = tx.send(Msg::Failed(problem));
                }
            });
        }
        Some(_) => {}
    }
}

/// A lâmina atual (com o recorte provisório), a prévia e o DPI do quadro selecionado.
pub fn refresh_spread(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(session) = app.editor.session() else {
        return;
    };
    let index = session.current();
    let (missing, warning) = (
        session.missing().to_vec(),
        !session.warnings_for(index).is_empty(),
    );
    let Some(project) = app.editor.view_project().map(Cow::into_owned) else {
        return;
    };
    sync_original(app, &project, index);
    let images = app.images.spread(&project, index);
    let image = |id: PhotoId| images.get(&id).cloned();
    let mut data = spread_data(
        &project,
        index,
        &missing,
        &image,
        (warning, crate::texts::BIG),
    );
    data.snaps = crate::hand::snap_boxes(app, &project);
    crate::adjusted::apply(app, &project, index, &images, &mut data);
    let frames = view_frames(&project, index, &missing);
    // Álbum ainda sem lâminas ("Começar sem fotos"): nada a pôr nas vagas nem a deslizar (spec
    // TR, regra 10).
    if let Some(spread) = project.spreads.get(index) {
        let place = (app.images.epoch(), spread.id);
        let before = (w.get_current(), app.slot_rows.rows());
        place_slots(app, place, &frames, &mut data);
        w.set_current(data);
        slide(app, &w, (place, index), before);
    } else {
        app.slot_rows.clear();
        w.set_current(data);
    }
    crate::organize::refresh(app);
    // Com a capa aberta, nenhuma lâmina fica marcada na faixa.
    let on_cover = app.editor.on_cover();
    w.set_current_index(if on_cover {
        -1
    } else {
        i32::try_from(index).unwrap_or(0)
    });
    let as_i32 = |f: Option<usize>| f.and_then(|f| i32::try_from(f).ok()).unwrap_or(-1);
    w.set_selected_frame(as_i32(app.editor.selected_frame()));
    w.set_crop_frame(as_i32(app.editor.crop_mode()));

    let preview = app.editor.crop_mode().and_then(|f| {
        let frame = frames.get(f)?;
        let FrameKind::Photo { id, .. } = &frame.kind else {
            return None;
        };
        Some((boxed(photo_extent(frame)?), images.get(id)?.clone()))
    });
    w.set_has_preview(preview.is_some());
    if let Some((rect, img)) = preview {
        w.set_preview(rect);
        w.set_preview_img(img);
    }
    let dpi = app
        .editor
        .selected_frame()
        .and_then(|f| frame_dpi_text(&project, index, f));
    w.set_info_frame(
        dpi.as_ref()
            .map(|(t, _)| t.clone())
            .unwrap_or_default()
            .into(),
    );
    w.set_info_frame_warn(dpi.is_some_and(|(_, warn)| warn));
    w.set_hint(app.editor.hint().into());
    let grid = app.editor.crop_grid();
    w.set_crop_grid_thirds(grid.thirds);
    w.set_crop_grid_fine(grid.fine);
    crate::photo_panel::refresh(app);
}

/// Os quadros da lâmina `index`, em frações da largura da lâmina (como a tela desenha).
fn view_frames(project: &Project, index: usize, missing: &[PhotoId]) -> Vec<FrameView> {
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    spread_view(project, index, area, missing).map_or_else(Vec::new, |v| v.frames)
}

/// Leva os quadros às vagas da lâmina grande (spec MV, regra 4) e marca o quadro selecionado e
/// o do recorte, que a tela não acha mais pela posição na lista.
fn place_slots(app: &mut App, place: Place, frames: &[FrameView], data: &mut crate::SpreadData) {
    let mut rows: Vec<crate::FrameData> = data.frames.iter().collect();
    for (row, frame) in rows.iter_mut().zip(frames) {
        if let FrameKind::Photo { id, .. } = &frame.kind {
            // A foto ajustada tem a imagem e o número dela (spec AF, regra 7).
            row.photo = crate::adjusted::version(frame.id).unwrap_or(app.images.version(*id));
        }
    }
    let gesture = app.editor.in_gesture();
    if app.slot_rows.sync(frames, &rows, place, gesture) {
        // As vagas que saíram esmaecem e depois saem da lista (MV-B12).
        let fade = u64::from(app.editor.durations().fade);
        crate::mailbox::later(Duration::from_millis(fade + 40), |app| {
            app.slot_rows.sweep();
        });
    }
    data.frames = app.slot_rows.model();
    let rect = |f: Option<usize>| {
        f.and_then(|f| frames.get(f))
            .map(|f| boxed(f.rect))
            .unwrap_or_default()
    };
    // No modo organizar com vários quadros, as alças são as da caixa do grupo (spec EG).
    let group: Vec<usize> = app.editor.selected_frames();
    data.selected_box = if app.editor.organizing() && group.len() > 1 {
        let rects: Vec<_> = group
            .iter()
            .filter_map(|f| frames.get(*f))
            .map(|f| f.rect)
            .collect();
        boxed(crate::organize::union_all(&rects))
    } else {
        rect(app.editor.selected_frame())
    };
    data.cropping_box = rect(app.editor.crop_mode());
}

/// Outra lâmina: a que sai fica por cima esmaecendo e a nova entra deslizando, na direção da
/// navegação (spec MV, seção 4); navegar seguido, outro álbum e "Reduzir movimento" não deslizam.
fn slide(
    app: &mut App,
    w: &AppWindow,
    (place, index): (Place, usize),
    (mut old, rows): (crate::SpreadData, Vec<crate::FrameData>),
) {
    let (at, reduce) = (app.clock.now(), app.editor.reduce_motion());
    let transition = app.navigator.arrive(place, index, at, reduce);
    let from = app.roll.shown.replace(index);
    old.frames = crate::convert::model(rows);
    // Com o rolo, perto do centro ele desliza (spec RL, regra 12).
    if crate::roll_connect::navigate(app, w, (index, transition), (&old, from)) {
        return;
    }
    let dir = match transition {
        Transition::Forward => 1,
        Transition::Backward => -1,
        Transition::Stay | Transition::Cut => return,
    };
    w.set_leaving(old);
    w.set_has_leaving(true);
    app.images.start_slide();
    w.invoke_start_nav(dir);
}

/// A foto original chegou: entra no quadro se ele ainda está em modo recorte.
pub fn original_ready(app: &mut App, id: PhotoId, pixels: crate::images::Pixels) {
    if app.loading == Some(id) {
        app.images.set_original(id, pixels);
        app.loading = None;
        refresh_spread(app);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_nav_ended(move || {
        let mut app = a.borrow_mut();
        if let Some(w) = app.window.upgrade() {
            w.set_has_leaving(false);
        }
        // As fotos nítidas que chegaram no meio do deslize entram agora (MV-B26).
        if app.images.slide_ended() {
            refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_frame_double(move |i| {
        let mut app = a.borrow_mut();
        // Fora dos quadros (-1) entra no modo organizar (spec EG); numa foto abre o recorte; num
        // texto, o cursor vai para o campo do texto (spec TX).
        match app.editor.double_click(usize::try_from(i).ok()) {
            DoubleClick::Text | DoubleClick::Organize | DoubleClick::Crop => app.refresh_current(),
            // Numa foto ausente: "Localizar esta foto…" (spec RF, regra 8).
            DoubleClick::Locate(photo) => {
                app.refresh_current();
                crate::relink_dialog::locate_photo(&mut app, photo);
            }
            DoubleClick::Nothing => {}
        }
    });
    let a = app.clone();
    window.on_crop_drag(move |dx, dy| {
        let mut app = a.borrow_mut();
        if app.editor.crop_drag(f64::from(dx), f64::from(dy)) {
            refresh_spread(&mut app);
        }
    });
    let a = app.clone();
    window.on_crop_end(move || {
        let mut app = a.borrow_mut();
        if app.editor.commit_crop() {
            app.refresh_all();
        }
    });
    let a = app.clone();
    window.on_crop_wheel(move |delta| {
        let mut app = a.borrow_mut();
        let steps = f64::from(delta) / WHEEL_NOTCH;
        let factor = lamina_editor::editor::ZOOM_STEP.powf(steps);
        if !app.editor.crop_zoom(factor) {
            return;
        }
        refresh_spread(&mut app);
        let b = a.clone();
        app.crop_timer
            .start(TimerMode::SingleShot, WHEEL_PAUSE, move || {
                let mut app = b.borrow_mut();
                if app.editor.commit_crop() {
                    app.refresh_all();
                }
            });
    });
}

/// Um temporizador parado, para o `App` guardar o da roda do mouse.
pub fn timer() -> Timer {
    Timer::default()
}
