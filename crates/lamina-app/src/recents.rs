//! A tela inicial com os álbuns recentes (spec TI): copia os cartões do `lamina-editor` para a
//! tela e traduz os cliques. A conferência dos arquivos que sumiram e a miniatura rodam fora da
//! linha da janela e voltam por mensagem.

use std::path::PathBuf;

use lamina_editor::error_log::{ErrorLog, LOG_FOLDER};
use lamina_editor::guarded;
use lamina_editor::recents::{
    Card, Clock, Resume, card_label, card_tip, cards, grid_cards, grid_columns, missing_files,
    no_results, recent_menu, resume, resume_height, show_search, write_big_thumbnail,
    write_thumbnail,
};
use slint::ComponentHandle;

use crate::convert::model;
use crate::files;
use crate::state::{App, Msg, Pending};
use crate::{AppWindow, RecentCard, RecentItem, RecentsUi};

type Shared = std::rc::Rc<std::cell::RefCell<App>>;

/// Copia a lista para a tela inicial e para o menu "Abrir recente ▸".
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let store = app.editor.recents();
    let all = cards(store.list(), &app.recent_missing, Clock::local());
    let ui = w.global::<RecentsUi>();
    ui.set_has_recents(!all.is_empty());
    ui.set_show_search(show_search(all.len()));
    let menu = recent_menu(&all)
        .into_iter()
        .map(|m| RecentItem {
            index: index(m.index),
            label: m.label.into(),
            enabled: m.enabled,
        })
        .collect();
    ui.set_menu(model(menu));
    // Buscando, o botão principal não muda: ele diz se algum álbum existe, não o que a busca achou.
    ui.set_main_new(resume(store.list(), &all, "").is_none());
    let current = resume(store.list(), &all, &app.recent_query);
    ui.set_has_resume(current.is_some());
    if let Some(data) = current.as_ref().and_then(|r| resume_data(app, &all, r)) {
        ui.set_resume(data);
    }
    let grid = grid_cards(&all, current.as_ref(), &app.recent_query);
    let searching = !app.recent_query.trim().is_empty();
    ui.set_no_results(if searching && grid.is_empty() {
        no_results(&app.recent_query).into()
    } else {
        Default::default()
    });
    ui.set_cards(model(grid.iter().map(|c| card_data(app, c)).collect()));
}

/// O cartão do álbum em andamento, com a miniatura grande; sem ela, a pequena, ampliada (regra 14).
fn resume_data(app: &App, all: &[Card], current: &Resume) -> Option<RecentCard> {
    let card = all.iter().find(|c| c.index == current.index)?;
    let recents = app.editor.recents();
    let big = recents
        .big_thumb_path(&card.path)
        .and_then(|p| app.seen_thumbs.borrow_mut().get(&p))
        .map(|picture| crate::news_player::image(&picture));
    let mut data = card_data(app, card);
    if let Some(picture) = big.filter(|_| !card.blank) {
        data.thumb = picture;
        data.has_thumb = true;
    }
    data.sentence = current.sentence.as_str().into();
    data.button = current.button.as_str().into();
    data.meta = current.meta.as_str().into();
    data.fold =
        recents.list().get(current.index).is_some_and(|r| r.fold) && app.editor.gutter_shadow_on();
    Some(data)
}

fn card_data(app: &App, c: &Card) -> RecentCard {
    // Pela data do arquivo: `Image::load_from_path` guardava a imagem pelo caminho e o cartão
    // ficava com a miniatura de antes depois de salvar (TI-B17); relida só quando o PNG muda.
    let thumb = app
        .editor
        .recents()
        .thumb_path(&c.path)
        .and_then(|p| app.seen_thumbs.borrow_mut().get(&p))
        .map(|picture| crate::news_player::image(&picture));
    RecentCard {
        index: index(c.index),
        name: c.name.as_str().into(),
        folder: c.folder.as_str().into(),
        when: c.when.as_str().into(),
        counts: c.counts.as_str().into(),
        pinned: c.pinned,
        missing: c.missing,
        has_thumb: thumb.is_some() && !c.blank,
        blank: c.blank,
        thumb: thumb.unwrap_or_default(),
        tip: card_tip(c).into(),
        label: card_label(c).into(),
        ..RecentCard::default()
    }
}

fn index(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

/// Confere, fora da linha da janela, quais arquivos sumiram; o cartão fica normal até a
/// resposta (seção 6, disco de rede).
pub fn check(app: &App) {
    let paths: Vec<PathBuf> = app
        .editor
        .recents()
        .list()
        .iter()
        .map(|r| r.path.clone())
        .collect();
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let gone = guarded(|| missing_files(&paths)).unwrap_or_default();
        // Se a janela já fechou, não há a quem avisar.
        let _ = tx.send(Msg::RecentsChecked(gone));
    });
}

/// Depois de salvar ou abrir: a miniatura da primeira lâmina, fora da linha da janela (regra 5).
pub fn thumbnail(app: &App) {
    let Some((project, target)) = app.editor.thumbnail_job() else {
        return;
    };
    // O álbum aberto é o álbum em andamento: só ele tem a miniatura grande (spec DI, regra 14).
    let big = app.editor.big_thumbnail_target();
    let (cache, tx) = (app.cache.clone(), app.tx.clone());
    std::thread::spawn(move || {
        // Se falhar, o cartão mostra o desenho neutro, sem aviso na tela, e o motivo vai para o
        // registro (spec TI, regra 5).
        let failed = match guarded(|| {
            write_thumbnail(&project, &cache, &target)?;
            big.map_or(Ok(()), |big| write_big_thumbnail(&project, &cache, &big))
        }) {
            Ok(Ok(())) => None,
            Ok(Err(reason)) => Some(reason),
            Err(panic) => Some(panic.details),
        };
        if let Some(reason) = failed {
            let log = ErrorLog::new(lamina_editor::config_dir().join(LOG_FOLDER));
            log.note(&format!("miniatura do álbum recente não saiu: {reason}"));
        }
        let _ = tx.send(Msg::ThumbReady);
    });
}

/// As respostas do trabalho em segundo plano.
pub fn message(app: &mut App, msg: Msg) {
    match msg {
        Msg::RecentsChecked(gone) => app.recent_missing = gone,
        Msg::ThumbReady => {}
        _ => return,
    }
    refresh(app);
}

/// Abre o álbum da posição `i` da lista, já na lâmina em que parou (depois da pergunta).
pub fn open(app: &mut App, i: usize) {
    let Some(file) = app.editor.recents().list().get(i).map(|r| r.path.clone()) else {
        return;
    };
    files::open_file(app, &file);
}

/// "Procurar…": escolhe o arquivo de novo; o cartão troca o caminho e mantém o resto.
fn browse(app: &mut App, i: usize) {
    let answer = crate::dialogs::dialog(app, "Procurar o álbum")
        .add_filter("Projeto do Lamina", &["lamina"])
        .pick_file();
    crate::dialogs::then(answer, move |app, file| {
        if let Some(file) = file {
            app.editor.change_recents(|s| s.relocate(i, file.path()));
            after_change(app);
        }
    });
}

fn pin(app: &mut App, i: usize) {
    if let Err(message) = app.editor.change_recents(|s| s.toggle_pin(i)) {
        app.notice = message;
    }
    after_change(app);
}

fn after_change(app: &mut App) {
    if let Some(notice) = app.editor.take_notice() {
        app.notice = notice;
    }
    app.refresh_current();
    refresh(app);
    check(app);
}

/// "Limpar recentes não fixados…": pede confirmação antes (regra 7).
pub fn ask_clear(app: &App) {
    if let Some(w) = app.window.upgrade() {
        w.global::<RecentsUi>().set_show_clear(true);
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<RecentsUi>();
    let a = app.clone();
    ui.on_open(move |i| {
        if let Ok(i) = usize::try_from(i) {
            files::request(&mut a.borrow_mut(), Pending::OpenRecent(i));
        }
    });
    let a = app.clone();
    ui.on_pin(move |i| at(&a, i, pin));
    let a = app.clone();
    ui.on_browse(move |i| at(&a, i, browse));
    let a = app.clone();
    ui.on_forget(move |i| {
        at(&a, i, |app, i| {
            app.editor.change_recents(|s| s.remove(i));
            after_change(app);
        })
    });
    let a = app.clone();
    ui.on_search(move |text| {
        let mut app = a.borrow_mut();
        app.recent_query = text.to_string();
        refresh(&app);
    });
    ui.on_grid_columns(|width| i32::try_from(grid_columns(width)).unwrap_or(1));
    ui.on_resume_height(resume_height);
    let a = app.clone();
    ui.on_clear_confirmed(move || {
        let mut app = a.borrow_mut();
        app.editor
            .change_recents(lamina_editor::RecentsStore::clear_unpinned);
        after_change(&mut app);
    });
}

fn at(app: &Shared, i: i32, action: fn(&mut App, usize)) {
    if let Ok(i) = usize::try_from(i) {
        action(&mut app.borrow_mut(), i);
    }
}
