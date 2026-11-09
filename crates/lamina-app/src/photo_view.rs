//! A foto à vista pela janela (spec VF, beta 9.1): preenche a prévia da lateral, abre e fecha a
//! ampliada (Espaço), navega com as setas e pede as imagens com prioridade. As decisões (a foto
//! escolhida, abrir, navegar) são do editor; aqui só a tela.

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::state::App;
use crate::{AppWindow, PhotoViewUi};

type Shared = Rc<RefCell<App>>;

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.global::<PhotoViewUi>().on_toggle(move || {
        let mut app = a.borrow_mut();
        if app.editor.toggle_photo_view() {
            // Sem invoke_focus_main ao abrir: o popup pega o teclado no show() e o tiraria dele.
            refresh(&mut app);
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.global::<PhotoViewUi>().on_close(move || {
        let mut app = a.borrow_mut();
        if app.editor.close_photo_view() {
            // O refresh atualiza o global (open=false desmonta o overlay) — sem ele, o editor
            // fechava e a tela continuava "aberta" (o travamento da conferência).
            refresh(&mut app);
            app.refresh_current();
            // Ao fechar, o teclado volta para a janela (o overlay desmontado solta o foco).
            if let Some(w) = app.window.upgrade() {
                w.invoke_focus_main();
            }
        }
    });
    let a = app.clone();
    window.global::<PhotoViewUi>().on_step(move |forward| {
        let mut app = a.borrow_mut();
        if app.editor.step_photo_view(forward) {
            refresh(&mut app);
            app.refresh_current();
            // O refresh pode levar o foco para a grade; com a foto aberta, o teclado volta para a
            // janela (o keys trata Esc/Espaço com o visualizador aberto — rede de segurança).
            if let Some(w) = app.window.upgrade() {
                w.invoke_focus_main();
            }
        }
    });
    let a = app.clone();
    window.global::<PhotoViewUi>().on_clicked_outside(move || {
        let mut app = a.borrow_mut();
        if app.editor.close_photo_view() {
            refresh(&mut app);
            app.refresh_current();
            if let Some(w) = app.window.upgrade() {
                w.invoke_focus_main();
            }
        }
    });
    let a = app.clone();
    window.global::<PhotoViewUi>().on_drag(move |dx, dy| {
        let mut app = a.borrow_mut();
        if app.editor.preview_drag(f64::from(dx), f64::from(dy)) {
            refresh(&mut app);
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.global::<PhotoViewUi>().on_release(move || {
        let mut app = a.borrow_mut();
        if app.editor.preview_release() {
            app.refresh_all();
        }
    });
}

/// Preenche a prévia da lateral e a ampliada com a foto escolhida (spec VF, seção 4).
pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PhotoViewUi>();
    let view = app.editor.photo_view();
    let Some(id) = view.chosen else {
        ui.set_has_preview(false);
        ui.set_open(false);
        return;
    };
    let Some(s) = app.session() else {
        ui.set_has_preview(false);
        ui.set_open(false);
        return;
    };
    let project = s.project().clone();
    let shown = app.shown_photos();
    let img = app.images.photo_view(&project, id);
    let photo = project.photo(id).ok();
    let caption = photo.map(|p| format!("{} · {} × {}", p.path.display(), p.width_px, p.height_px));
    let uses = lamina_editor::photo_list(&project, lamina_editor::Filter::All)
        .into_iter()
        .find(|item| item.id == id)
        .map(|item| {
            if item.uses == 0 {
                String::new()
            } else {
                format!("×{}", item.uses)
            }
        })
        .unwrap_or_default();
    // A posição na aba atual ("14 de 300").
    let big = shown
        .iter()
        .position(|p| *p == id)
        .map(|at| {
            let n = at + 1;
            format!(
                "{} · {} de {}",
                caption.clone().unwrap_or_default(),
                n,
                shown.len()
            )
        })
        .unwrap_or_default();
    ui.set_has_preview(img.is_some() || caption.is_some());
    ui.set_preview_img(img.clone().unwrap_or_default());
    ui.set_caption(caption.clone().unwrap_or_default().into());
    ui.set_uses(uses.into());
    // O recorte visível da prévia (frações da imagem), quando a foto vem de um quadro.
    let clip = app.editor.preview_clip();
    let (cx, cy, cw, ch) = clip.unwrap_or((0.0, 0.0, 1.0, 1.0));
    #[allow(clippy::cast_possible_truncation)]
    let f = |v: f64| v as f32;
    ui.set_has_clip(clip.is_some());
    ui.set_clip_x(f(cx));
    ui.set_clip_y(f(cy));
    ui.set_clip_w(f(cw.max(0.001)));
    ui.set_clip_h(f(ch.max(0.001)));
    // A proporção da prévia (altura ÷ largura): a da foto inteira, ou a do recorte visível quando
    // ela vem de um quadro — a caixa segue o formato do que se vê, sem cortar.
    #[allow(clippy::cast_possible_truncation)]
    let aspect = photo.map(|p| {
        let photo = f64::from(p.height_px) / f64::from(p.width_px).max(1.0);
        let region = clip.map(|(_, _, w, h)| w / h.max(0.001));
        (region.unwrap_or(1.0) * photo) as f32
    });
    ui.set_preview_aspect(aspect.unwrap_or(1.0));
    ui.set_can_frame(view.frame.is_some() && img.is_some());
    ui.set_open(view.open);
    ui.set_img(img.unwrap_or_default());
    ui.set_big_caption(big.into());
}
