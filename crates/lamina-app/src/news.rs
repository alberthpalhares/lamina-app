//! "O que há de novo" (spec NV): as notas e as ilustrações vão dentro do programa (regra 11), e a
//! janela mostra o que o `lamina_editor::updates::news` decide. Aqui só a tradução para o Slint.

use lamina_editor::updates::news::{
    Animation, NewsEntry, NewsPage, NewsSection, after_update, entries, from_help, next, parse,
};
use slint::{ComponentHandle, Image};

use crate::convert::model;
use crate::news_player;
use crate::state::App;
use crate::{AppWindow, NewsEntryData, NewsUi};

/// As notas de todas as versões, as mesmas da página da versão no GitHub.
const NOTES: &str = include_str!("../dist/novidades.md");

/// As ilustrações de `dist/novidades/` (geradas pelo `build.rs`).
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/novidades.rs"));
}

/// `LAMINA_NOVIDADES=<versão>`: o Lamina compilado abre a janela como se viesse dessa versão, sem
/// gravar nada (seção 3).
const SIMULATE: &str = "LAMINA_NOVIDADES";

fn sections() -> Vec<NewsSection> {
    parse(NOTES)
}

/// Na abertura, antes de a janela aparecer (regra 3): lembra a versão e, se o Lamina instalado
/// acabou de atualizar e há notas, abre a janela.
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
        show(app, &page);
    }
}

/// Ajuda → "O que há de novo…" (regra 5).
pub fn open_help(app: &App) {
    show(app, &from_help(&sections(), app.updates.current()));
}

fn show(app: &App, page: &NewsPage) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewsUi>();
    let list = entries(page);
    let reduce = app.editor.reduce_motion();
    ui.set_title(page.title.clone().into());
    ui.set_subtitle(page.subtitle.clone().into());
    ui.set_empty(page.empty.clone().into());
    ui.set_entries(model(list.iter().map(|e| entry_data(e, reduce)).collect()));
    ui.set_selected(0);
    ui.set_show(true);
    let bytes = list
        .iter()
        .map(|e| e.item.image.as_deref().and_then(embedded_bytes));
    news_player::load(bytes.collect(), reduce);
    news_player::choose(&w, 0);
}

fn entry_data(entry: &NewsEntry, reduce_motion: bool) -> NewsEntryData {
    let item = &entry.item;
    let image = item
        .image
        .as_deref()
        .and_then(|n| picture(n, reduce_motion));
    NewsEntryData {
        heading: entry.heading.clone().unwrap_or_default().into(),
        intro: entry.intro.clone().into(),
        current: entry.current,
        label: entry.label.clone().into(),
        title: item.title.clone().unwrap_or_default().into(),
        text: item.text.clone().into(),
        try_it: item.try_it.clone().unwrap_or_default().into(),
        command: item.command.clone().unwrap_or_default().into(),
        has_image: image.is_some(),
        image: image.unwrap_or_default(),
    }
}

/// Os bytes da ilustração embutida de nome `name`.
fn embedded_bytes(name: &str) -> Option<&'static [u8]> {
    let (_, bytes) = embedded::IMAGES.iter().find(|(n, _)| *n == name)?;
    Some(bytes)
}

/// O primeiro quadro da ilustração embutida de nome `name`, pronto para a janela: o que aparece ao
/// escolher a novidade e o que fica com "Reduzir movimento".
fn picture(name: &str, reduce_motion: bool) -> Option<Image> {
    let animation = Animation::open(embedded_bytes(name)?, reduce_motion)?;
    Some(news_player::image(&animation.first().picture))
}

pub fn connect(window: &AppWindow) {
    let ui = window.global::<NewsUi>();
    let w = window.as_weak();
    ui.on_next(move || {
        if let Some(w) = w.upgrade() {
            let ui = w.global::<NewsUi>();
            let count = slint::Model::row_count(&ui.get_entries());
            let at = usize::try_from(ui.get_selected()).unwrap_or(0);
            select(&w, next(at, count));
        }
    });
    let w = window.as_weak();
    ui.on_chose(move |index| {
        if let Some(w) = w.upgrade() {
            select(&w, usize::try_from(index).unwrap_or(0));
        }
    });
    let w = window.as_weak();
    ui.on_released(move || {
        if let Some(w) = w.upgrade() {
            release(&w);
        }
    });
}

/// A janela fechou: para a animação e solta as ilustrações, que somam dezenas de MB (NV-B17).
fn release(window: &AppWindow) {
    news_player::stop();
    let ui = window.global::<NewsUi>();
    ui.set_playing(false);
    ui.set_frame(Image::default());
    ui.set_entries(model(Vec::new()));
    ui.set_selected(0);
}

/// Escolhe a novidade da linha `index` e toca a ilustração dela.
fn select(window: &AppWindow, index: usize) {
    let ui = window.global::<NewsUi>();
    ui.set_selected(i32::try_from(index).unwrap_or(0));
    news_player::choose(window, index);
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    use slint::platform::{Platform, WindowAdapter};
    use slint::{Model, PlatformError};

    use super::*;

    /// A plataforma sem janela do sistema, para criar a `AppWindow` de verdade no teste.
    struct Headless(Rc<MinimalSoftwareWindow>);

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn nv_b17_fechar_solta_as_ilustracoes_da_memoria() {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window))).unwrap();
        let app = AppWindow::new().unwrap();
        connect(&app);
        let ui = app.global::<NewsUi>();
        let entry = NewsEntryData {
            has_image: true,
            ..NewsEntryData::default()
        };
        ui.set_entries(model(vec![entry.clone(), entry]));
        ui.set_selected(1);
        ui.set_playing(true);
        ui.set_show(true);

        ui.invoke_close();

        assert!(!ui.get_show());
        assert_eq!(ui.get_entries().row_count(), 0);
        assert_eq!(ui.get_selected(), 0);
        assert!(!ui.get_playing());
    }

    #[test]
    fn as_notas_embutidas_leem_e_toda_ilustracao_citada_esta_embutida() {
        let all = sections();
        assert!(!all.is_empty());
        assert!(all.iter().all(|s| !s.items.is_empty()));
        for item in all.iter().flat_map(|s| &s.items) {
            if let Some(name) = &item.image {
                assert!(picture(name, false).is_some(), "{name} não está embutida");
            }
        }
    }
}
