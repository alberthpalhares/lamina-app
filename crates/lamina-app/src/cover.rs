//! A capa pela janela (spec CA): a caixa de medidas, o item da faixa, as linhas da lombada e o guia.
//! As contas ficam no `lamina-editor`; aqui só o vai e vem com o `CoverUi`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lamina_core::PhotoId;
use lamina_editor::Command;
use lamina_editor::cover_form::{CoverForm, CoverTab, guide_file, guide_picture};
use lamina_editor::motion::RollPill;
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::convert::to_f32;
use crate::state::App;
use crate::{AppWindow, CoverLook, CoverUi};

type Shared = Rc<RefCell<App>>;

fn tab_index(tab: CoverTab) -> i32 {
    match tab {
        CoverTab::Album => 0,
        CoverTab::Total => 1,
        CoverTab::Guide => 2,
    }
}

fn file_name(form: &CoverForm) -> String {
    form.guide
        .as_ref()
        .and_then(|g| g.path.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Os campos da caixa, como a sessão de agora os vê.
fn show_fields(ui: &CoverUi<'_>, form: &CoverForm) {
    ui.set_tab(tab_index(form.tab));
    ui.set_face_width(form.face_width.as_str().into());
    ui.set_face_height(form.face_height.as_str().into());
    ui.set_spine_auto(form.spine_auto);
    ui.set_sheet(form.sheet_mm.as_str().into());
    ui.set_spine(form.spine.as_str().into());
    ui.set_wrap(form.wrap.as_str().into());
    ui.set_safe(form.safe.as_str().into());
    ui.set_total_width(form.total_width.as_str().into());
    ui.set_total_height(form.total_height.as_str().into());
    ui.set_includes_wrap(form.includes_wrap);
    ui.set_guide_width(form.guide_width.as_str().into());
}

/// O que sai dos campos: o resumo, os avisos, o erro e o desenho.
fn show_derived(ui: &CoverUi<'_>, form: &CoverForm) {
    ui.set_guide_name(file_name(form).into());
    ui.set_needs_guide_width(form.needs_guide_width());
    let error = form.measures().err().unwrap_or_default();
    let missing_guide = form.tab == CoverTab::Guide && form.guide.is_none();
    let error = if missing_guide && error.is_empty() {
        "Escolha o arquivo-guia do laboratório.".to_owned()
    } else {
        error
    };
    ui.set_error(error.into());
    ui.set_summary(form.summary().unwrap_or_default().into());
    ui.set_warnings(form.warnings().join("\n").into());
    if let Some(d) = form.drawing() {
        ui.set_draw_aspect(to_f32(d.aspect));
        ui.set_back_x(to_f32(d.back.x));
        ui.set_back_y(to_f32(d.back.y));
        ui.set_back_w(to_f32(d.back.w));
        ui.set_back_h(to_f32(d.back.h));
        ui.set_spine_x(to_f32(d.spine.x));
        ui.set_spine_w(to_f32(d.spine.w));
        ui.set_front_x(to_f32(d.front.x));
        ui.set_front_w(to_f32(d.front.w));
    }
}

/// Abre a caixa: para criar, com as fotos que nascem na capa; ou para mudar as medidas.
pub fn open(app: &mut App, photos: Vec<lamina_core::PhotoId>) {
    let (Some(w), Some(form)) = (app.window.upgrade(), app.editor.cover_form()) else {
        return;
    };
    let ui = w.global::<CoverUi>();
    ui.set_creating(!app.editor.has_cover());
    show_fields(&ui, &form);
    show_derived(&ui, &form);
    ui.set_open(true);
    app.cover_form = Some((form, photos));
}

fn read_fields(ui: &CoverUi<'_>, form: &mut CoverForm) {
    form.face_width = ui.get_face_width().into();
    form.face_height = ui.get_face_height().into();
    form.spine_auto = ui.get_spine_auto();
    form.sheet_mm = ui.get_sheet().into();
    form.spine = ui.get_spine().into();
    form.wrap = ui.get_wrap().into();
    form.safe = ui.get_safe().into();
    form.total_width = ui.get_total_width().into();
    form.total_height = ui.get_total_height().into();
    form.includes_wrap = ui.get_includes_wrap();
}

fn changed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<CoverUi>();
    let Some((form, _)) = app.cover_form.as_mut() else {
        return;
    };
    read_fields(&ui, form);
    let typed: String = ui.get_guide_width().into();
    if typed != form.guide_width {
        form.set_guide_width(&typed);
        ui.set_total_width(form.total_width.as_str().into());
        ui.set_total_height(form.total_height.as_str().into());
    }
    show_derived(&ui, form);
}

fn pick_tab(app: &mut App, index: i32) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<CoverUi>();
    let Some((form, _)) = app.cover_form.as_mut() else {
        return;
    };
    read_fields(&ui, form);
    form.tab = match index {
        1 => CoverTab::Total,
        2 => CoverTab::Guide,
        _ => CoverTab::Album,
    };
    ui.set_tab(index);
    show_derived(&ui, form);
}

fn choose_guide(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let picked = rfd::FileDialog::new()
        .set_title("Arquivo-guia do laboratório")
        .add_filter("Imagem", &["jpg", "jpeg", "png", "tif", "tiff"])
        .pick_file();
    let Some(path) = picked else {
        return;
    };
    let ui = w.global::<CoverUi>();
    let Some((form, _)) = app.cover_form.as_mut() else {
        return;
    };
    read_fields(&ui, form);
    match guide_file(&path) {
        Ok(file) => {
            form.set_guide(file);
            show_fields(&ui, form);
            show_derived(&ui, form);
        }
        Err(message) => ui.set_error(message.into()),
    }
}

fn confirm(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<CoverUi>();
    let Some((form, photos)) = app.cover_form.as_mut() else {
        return;
    };
    read_fields(&ui, form);
    let measures = match form.measures() {
        Ok(m) => m,
        Err(message) => {
            ui.set_error(message.into());
            return;
        }
    };
    let guide = form.guide_to_save();
    let photos = std::mem::take(photos);
    let done = if app.editor.has_cover() {
        app.editor.set_cover_measures(measures, guide)
    } else {
        app.editor.create_cover(measures, guide, &photos)
    };
    if done {
        ui.set_open(false);
        app.cover_form = None;
        app.refresh_all();
    } else {
        app.refresh_current();
    }
}

/// O guia reduzido, lido uma vez por caminho; sem o arquivo, a capa funciona sem ele (seção 6).
fn guide_image(app: &mut App, path: &std::path::Path) -> Option<Image> {
    if let Some((seen, image)) = &app.cover_guide
        && seen == path
    {
        return image.clone();
    }
    let image = guide_picture(path).map(|picture| {
        let pixels = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            picture.as_raw(),
            picture.width(),
            picture.height(),
        );
        Image::from_rgba8(pixels)
    });
    if image.is_none() {
        app.notice =
            "O guia da capa não foi encontrado. Escolha o arquivo de novo em Medidas da capa."
                .to_owned();
        if let Some(w) = app.window.upgrade() {
            w.set_notice(app.notice.clone().into());
        }
    }
    app.cover_guide = Some((path.to_path_buf(), image.clone()));
    image
}

/// O estado da capa na tela: a capa aberta, as linhas, o guia e o atalho do rolo.
pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<CoverUi>();
    ui.set_on(app.editor.on_cover());
    ui.set_has_cover(app.editor.has_cover());
    ui.set_roll_pill(
        match app.editor.roll_pill() {
            RollPill::None => "",
            RollPill::GoToCover => "← Capa",
            RollPill::CreateCover => "Criar a capa",
        }
        .into(),
    );
    let look = w.global::<CoverLook>();
    look.set_on(app.editor.on_cover());
    let Some(overlay) = app.editor.cover_overlay() else {
        look.set_guide_on(false);
        return;
    };
    look.set_spine_l(to_f32(overlay.spine_left));
    look.set_spine_r(to_f32(overlay.spine_right));
    look.set_trim_top(to_f32(overlay.trim_top));
    let picture = overlay
        .guide
        .as_ref()
        .and_then(|g| guide_image(app, &g.path).map(|image| (image, g)));
    match picture {
        Some((image, guide)) => {
            look.set_guide_image(image);
            look.set_guide_x(to_f32(guide.at.0));
            look.set_guide_y(to_f32(guide.at.1));
            look.set_guide_w(to_f32(guide.at.2));
            look.set_guide_h(to_f32(guide.at.3));
            look.set_guide_on(true);
        }
        None => look.set_guide_on(false),
    }
}

/// A miniatura da capa na faixa (ou a proporção do espaço, sem capa).
pub fn refresh_tile(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(album) = app.session().map(lamina_editor::Session::album_shared) else {
        return;
    };
    let aspect = lamina_editor::cover_form::cover_aspect(&album);
    w.global::<CoverUi>().set_tile_aspect(to_f32(aspect));
    if album.cover.is_some() {
        let images = app.thumbs(&album);
        draw_tile(app, &images);
    }
}

/// Chegou a miniatura de uma foto da capa: o espaço na faixa mostra, sem pedir nada de novo.
pub fn thumb_arrived(app: &App) {
    draw_tile(app, app.images.thumbs_map());
}

fn draw_tile(app: &App, images: &HashMap<PhotoId, Image>) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    if let Some(canvas) = session.album().cover_canvas().ok().flatten() {
        let warned = session.cover_warned();
        let data = crate::state::strip_data(&canvas, 0, session.missing(), images, warned);
        w.global::<CoverUi>().set_tile(data);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<CoverUi>();
    let a = app.clone();
    ui.on_changed(move || changed(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_tab_picked(move |i| pick_tab(&mut a.borrow_mut(), i));
    let a = app.clone();
    ui.on_choose_guide(move || choose_guide(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_confirm(move || confirm(&mut a.borrow_mut()));
    let (a, weak) = (app.clone(), window.as_weak());
    ui.on_cancel(move || {
        a.borrow_mut().cover_form = None;
        if let Some(w) = weak.upgrade() {
            w.global::<CoverUi>().set_open(false);
        }
    });
    let a = app.clone();
    ui.on_tile_clicked(move || {
        let mut app = a.borrow_mut();
        if app.editor.run(Command::GoToCover) {
            app.refresh_all();
        }
    });
    let a = app.clone();
    ui.on_tile_double_clicked(move || {
        let mut app = a.borrow_mut();
        if !app.editor.has_cover() {
            open(&mut app, Vec::new());
        }
    });
    let a = app.clone();
    ui.on_tile_dropped(move |i| {
        let mut app = a.borrow_mut();
        let Some(photo) = app.photo_at(i) else {
            return;
        };
        let photos = app.editor.drag_photos(photo);
        if app.editor.has_cover() {
            // No item da capa, as fotos vão para a frente, sem mexer no verso.
            if app.editor.go_to_cover() {
                app.editor.drop_on_cover(0.9, &photos);
                app.refresh_all();
            }
        } else {
            open(&mut app, photos);
        }
    });
    let a = app.clone();
    ui.on_pill_clicked(move || {
        let mut app = a.borrow_mut();
        if app.editor.has_cover() {
            if app.editor.go_to_cover() {
                app.refresh_all();
            }
        } else {
            open(&mut app, Vec::new());
        }
    });
}
