//! A página "O que há de novo" (specs NV e PV): as notas e as capas vão dentro do programa, e a
//! página mostra o que o `lamina_editor::updates::news` decide. Aqui só a tradução para o Slint.

mod connect;
mod covers;
mod rows;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_focus;
#[cfg(test)]
mod tests_layout;

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::updates::news::{
    NewsEntry, NewsFilter, NewsPage, NewsSection, TryNow, after_update, available_filters, entries,
    from_help, highlights, no_results, parse, shown, shows_highlights,
};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::news_player;
use crate::state::App;
use crate::{AppWindow, NewsCardData, NewsEntryData, NewsFilterData, NewsUi};
pub use connect::connect;
use covers::Covers;

/// As notas de todas as versões, as mesmas da página da versão no GitHub.
const NOTES: &str = include_str!("../dist/novidades.md");

/// As ilustrações de `dist/novidades/` (geradas pelo `build.rs`).
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/novidades.rs"));
}

/// `LAMINA_NOVIDADES=<versão>`: o Lamina compilado abre a página como se viesse dessa versão, sem
/// gravar nada (NV, seção 3).
const SIMULATE: &str = "LAMINA_NOVIDADES";

fn sections() -> Vec<NewsSection> {
    parse(NOTES)
}

/// A página a abrir: o que aparece, a lista de "Ver todas as versões" e o "Experimente agora" de
/// cada item, na ordem de cada lista.
struct Opening {
    page: NewsPage,
    tries: Vec<TryNow>,
    help: NewsPage,
    help_tries: Vec<TryNow>,
    reduce_motion: bool,
}

/// O que a página aberta guarda: as listas da tela ficam em propriedades próprias, gravadas uma vez
/// (spec RL, regra 32); uma capa que chega vai só na linha dela.
struct Shown {
    page: NewsPage,
    all: Vec<NewsEntry>,
    tries: Vec<TryNow>,
    help: Option<(NewsPage, Vec<TryNow>)>,
    filters: Vec<NewsFilter>,
    filter: NewsFilter,
    query: String,
    reduce_motion: bool,
    /// O índice do item de cada linha da lista.
    listed: Vec<usize>,
    rows: Rc<VecModel<NewsEntryData>>,
    cards: Rc<VecModel<NewsCardData>>,
    covers: Covers,
    /// Quantas vezes a lista mudou: a página volta ao começo.
    generation: i32,
}

impl Default for Shown {
    fn default() -> Self {
        Shown {
            page: from_help(&[], &lamina_editor::updates::Version::default()),
            all: Vec::new(),
            tries: Vec::new(),
            help: None,
            filters: Vec::new(),
            filter: NewsFilter::All,
            query: String::new(),
            reduce_motion: false,
            listed: Vec::new(),
            rows: Rc::new(VecModel::default()),
            cards: Rc::new(VecModel::default()),
            covers: Covers::default(),
            generation: 0,
        }
    }
}

thread_local! {
    static SHOWN: RefCell<Shown> = RefCell::new(Shown::default());
}

/// Na abertura, antes de a janela aparecer (NV, regra 3): lembra a versão e, se o Lamina instalado
/// acabou de atualizar e há notas, abre a página.
pub fn on_start(app: &mut App) {
    let step = match std::env::var(SIMULATE) {
        Ok(previous) => app.updates.simulate_start(&previous),
        Err(_) => app.updates.on_start(),
    };
    if let Some(notice) = step.notice {
        app.notice = notice;
    }
    let Some(previous) = step.news_since else {
        return;
    };
    if let Some(page) = after_update(&sections(), &previous, app.updates.current()) {
        show(app, page);
    }
}

/// Ajuda → "O que há de novo…" (NV, regra 5).
pub fn open_help(app: &App) {
    show(app, from_help(&sections(), app.updates.current()));
}

fn show(app: &App, page: NewsPage) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let tries_of = |p: &NewsPage| -> Vec<TryNow> {
        entries(p)
            .iter()
            .map(|e| app.editor.try_now(&e.item))
            .collect()
    };
    let help = from_help(&sections(), app.updates.current());
    open_page(
        &w,
        Opening {
            tries: tries_of(&page),
            help_tries: tries_of(&help),
            page,
            help,
            reduce_motion: app.editor.reduce_motion(),
        },
    );
}

fn open_page(window: &AppWindow, opening: Opening) {
    SHOWN.with_borrow_mut(|s| {
        s.reduce_motion = opening.reduce_motion;
        s.help = (opening.page.current.is_none()).then_some((opening.help, opening.help_tries));
        s.filter = NewsFilter::All;
        s.query.clear();
        s.covers.clear();
        use_page(window, s, opening.page, opening.tries);
    });
    let ui = window.global::<NewsUi>();
    ui.set_query("".into());
    ui.set_show(true);
}

/// Troca a página mostrada: ao abrir e em "Ver todas as versões" (a busca continua valendo).
fn use_page(window: &AppWindow, s: &mut Shown, page: NewsPage, tries: Vec<TryNow>) {
    s.all = entries(&page);
    s.tries = tries;
    s.filters = if s.all.is_empty() {
        Vec::new()
    } else {
        available_filters(&page)
    };
    let ui = window.global::<NewsUi>();
    ui.set_title(page.title.clone().into());
    ui.set_subtitle(page.subtitle.clone().into());
    ui.set_empty(page.empty.clone().into());
    ui.set_more(s.help.is_some());
    let names = s.filters.iter().map(|f| NewsFilterData {
        label: f.label().into(),
    });
    ui.set_filters(ModelRc::new(VecModel::from(names.collect::<Vec<_>>())));
    let picked = highlights(&page);
    let cards = picked.iter().map(|(_, e)| rows::card(e, &s.covers));
    s.cards.set_vec(cards.collect::<Vec<_>>());
    s.page = page;
    refresh(window, s);
}

/// O filtro ou a busca mudou: a lista inteira é refeita, e a página volta ao começo.
fn refresh(window: &AppWindow, s: &mut Shown) {
    news_player::stop(window);
    let list = shown(&s.page, s.filter, &s.query);
    s.listed = list.iter().map(|e| e.index).collect();
    let data = list
        .iter()
        .map(|e| rows::row(e, s.tries.get(e.index), s.reduce_motion, &s.covers));
    s.rows.set_vec(data.collect::<Vec<_>>());
    s.generation += 1;
    let ui = window.global::<NewsUi>();
    ui.set_generation(s.generation);
    ui.set_show_highlights(shows_highlights(s.filter, &s.query));
    let at = s.filters.iter().position(|f| *f == s.filter).unwrap_or(0);
    ui.set_filter(i32::try_from(at).unwrap_or(0));
    let none = list.is_empty() && !s.all.is_empty();
    ui.set_no_results(
        if none {
            no_results(&s.query)
        } else {
            String::new()
        }
        .into(),
    );
}
