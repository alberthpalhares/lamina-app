//! Os botões e a lista da página ligados ao Rust, e o que fechar solta.

use lamina_editor::updates::news::NewsFilter;
use slint::{ComponentHandle, ModelRc};

use super::{SHOWN, Shown, covers, refresh, rows, use_page};
use crate::news_player;
use crate::{AppWindow, NewsUi};

/// Liga a página à janela. Na abertura depois de atualizar, ela já abriu (`news::on_start`, antes
/// daqui): nada do que está mostrando pode se perder.
pub fn connect(window: &AppWindow) {
    let ui = window.global::<NewsUi>();
    SHOWN.with_borrow(|s| {
        ui.set_entries(ModelRc::from(s.rows.clone()));
        ui.set_highlights(ModelRc::from(s.cards.clone()));
    });
    let w = window.as_weak();
    ui.on_search(move |text| {
        with_window(&w, |w, s| {
            s.query = text.to_string();
            refresh(w, s);
        });
    });
    let w = window.as_weak();
    ui.on_filter_chosen(move |at| {
        with_window(&w, |w, s| {
            let chosen = usize::try_from(at).ok().and_then(|i| s.filters.get(i));
            s.filter = chosen.copied().unwrap_or(NewsFilter::All);
            refresh(w, s);
        });
    });
    let w = window.as_weak();
    ui.on_all_versions(move || {
        with_window(&w, |w, s| {
            if let Some((page, tries)) = s.help.take() {
                s.covers.clear();
                use_page(w, s, page, tries);
            }
        });
    });
    connect_covers(window);
    let w = window.as_weak();
    ui.on_released(move || {
        if let Some(w) = w.upgrade() {
            release(&w);
        }
    });
}

fn connect_covers(window: &AppWindow) {
    let ui = window.global::<NewsUi>();
    let w = window.as_weak();
    ui.on_cover_requested(move |index| {
        if let (Some(w), Ok(index)) = (w.upgrade(), usize::try_from(index)) {
            covers::request(&w, index);
        }
    });
    let w = window.as_weak();
    ui.on_cover_clicked(move |index| {
        if let (Some(w), Ok(index)) = (w.upgrade(), usize::try_from(index)) {
            let bytes = SHOWN.with_borrow(|s| rows::bytes_of(s.all.get(index)?));
            if let Some(bytes) = bytes {
                news_player::toggle(&w, index, bytes);
            }
        }
    });
    let w = window.as_weak();
    ui.on_scrolled(move || {
        if let Some(w) = w.upgrade()
            && news_player::playing().is_some()
        {
            news_player::stop(&w);
        }
    });
}

fn with_window(window: &slint::Weak<AppWindow>, run: impl FnOnce(&AppWindow, &mut Shown)) {
    if let Some(w) = window.upgrade() {
        SHOWN.with_borrow_mut(|s| run(&w, s));
    }
}

/// A página fechou: para a animação, solta as capas e volta ao começo (NV-B17 e PV-B19).
fn release(window: &AppWindow) {
    news_player::stop(window);
    SHOWN.with_borrow_mut(|s| {
        s.rows.set_vec(Vec::new());
        s.cards.set_vec(Vec::new());
        s.covers.clear();
        s.listed.clear();
        s.all.clear();
        s.tries.clear();
        s.help = None;
        s.filters.clear();
        s.filter = NewsFilter::All;
        s.query.clear();
    });
    let ui = window.global::<NewsUi>();
    ui.set_filter(0);
    ui.set_query("".into());
    ui.set_no_results("".into());
    ui.set_filters(ModelRc::default());
}
