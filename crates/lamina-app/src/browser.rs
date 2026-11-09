//! O navegador de fotos (spec PN): o clique e a seleção com Ctrl e Shift, o duplo clique que acha
//! a foto no álbum e as abas Todas, Usadas e Não usadas. As decisões são do editor; soltar as
//! fotos fica em `drops.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::Filter;
use slint::ComponentHandle;

use crate::AppWindow;
use crate::state::App;

type Shared = Rc<RefCell<App>>;

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_photo_pressed(move |i, ctrl, shift| {
        let edit = |app: &mut App| {
            let shown = app.shown_photos();
            if let Ok(index) = usize::try_from(i) {
                app.editor.click_photo(&shown, index, ctrl, shift);
            }
            app.grid_focus = true;
            crate::photo_menu::refresh(app);
            true
        };
        let show = |app: &mut App| {
            let id = app.photo_at(i);
            app.select_photo(id);
            crate::photo_view::refresh(app);
            app.refresh_current();
        };
        a.borrow_mut().timed("Clicar numa foto", edit, show);
    });
    let a = app.clone();
    window.on_photo_clicked(move |index| {
        let mut app = a.borrow_mut();
        let id = app.photo_at(index);
        app.select_photo(id);
    });
    let a = app.clone();
    window.on_photo_double_clicked(move |index| {
        let mut app = a.borrow_mut();
        let Some(id) = app.photo_at(index) else {
            return;
        };
        // Uma foto ausente: "Localizar esta foto…" (spec RF, regra 8).
        if app.session().is_some_and(|s| s.missing().contains(&id)) {
            crate::relink_dialog::locate_photo(&mut app, id);
        } else if app.editor.go_to_photo(id) {
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.on_filter_changed(move |filter| {
        let filter = match filter {
            1 => Filter::Used,
            2 => Filter::Unused,
            _ => Filter::All,
        };
        a.borrow_mut().set_filter(filter);
    });
    let a = app.clone();
    window.on_photo_reorder(move |from, to| {
        let mut app = a.borrow_mut();
        // Só reordena com o critério Personalizada (spec OF, regra 4).
        if app.editor.photo_sort() != lamina_editor::ordering::SortBy::Custom {
            return;
        }
        let shown = app.shown_photos();
        let (Some(&id),) = (shown.get(usize::try_from(from).ok().unwrap_or(usize::MAX)),) else {
            return;
        };
        if app
            .editor
            .move_photo_to(id, usize::try_from(to).unwrap_or(0))
        {
            app.refresh_photos();
            crate::photo_view::refresh(&mut app);
            app.refresh_current();
        }
    });
    let a = app.clone();
    let set = |a: &Shared, sort: i32| {
        let sort = match sort {
            1 => lamina_editor::ordering::SortBy::Name,
            2 => lamina_editor::ordering::SortBy::Custom,
            _ => lamina_editor::ordering::SortBy::Capture,
        };
        a.borrow_mut().set_sort(sort);
    };
    window.on_sort_changed({
        let a = a.clone();
        move |sort| set(&a, sort)
    });
    // O clique no dropdown também chama o callback do global (o popup é uma janela separada; o
    // encadeamento do componente nem sempre chega).
    window.global::<crate::SortUi>().on_changed({
        let a = a.clone();
        move |sort| set(&a, sort)
    });
}
