//! A faixa de layouts pela janela (spec FL): copia os cartões do editor para o global `BandUi`,
//! com uma miniatura de cada opção feita com as fotos de verdade (só com a faixa aberta; regra
//! 13), e leva o mouse parado num cartão à prévia na lâmina grande (regras 3 e 5).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use lamina_core::PhotoId;
use lamina_editor::motion::panel_key;
use lamina_editor::{Command, Part};
use slint::{ComponentHandle, Timer, TimerMode};

use crate::convert::{model, spread_data};
use crate::state::App;
use crate::texts::SMALL;
use crate::{AppWindow, BandUi, LayoutItem};

type Shared = Rc<RefCell<App>>;

/// Quanto o mouse fica parado num cartão antes da prévia: passar depressa não faz prévia
/// nenhuma (regra 5).
const REST: Duration = Duration::from_millis(100);

pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<BandUi>();
    let band = app.editor.band();
    ui.set_open(band.open);
    ui.set_list(i32::from(app.editor.reordered()));
    // Fechada, nada é calculado nem desenhado (regra 13).
    let Some(session) = app.editor.session().filter(|_| band.open) else {
        return;
    };
    let (project, index) = (session.project_shared(), session.current());
    let missing = session.missing().to_vec();
    let panel = app.editor.panel(Part::Spread);
    ui.set_has_reordered(app.editor.has_reordered());
    ui.set_in_use(
        panel
            .in_use
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(-1),
    );
    ui.set_notice(panel.notice().into());
    ui.set_hint(panel.hint().into());
    // As mesmas miniaturas: nada a refazer, só a opção em uso muda (spec MV, regra 3; MV-B17).
    let key = panel_key(&panel, session);
    if app.lists.panel.as_ref() == Some(&key) {
        return;
    }
    let thumbs = app.thumbs(&project);
    let image = |id: PhotoId| thumbs.get(&id).cloned();
    let cards = panel.cards();
    let mut items: Vec<LayoutItem> = Vec::with_capacity(cards.len());
    app.editor.for_each_preview(&panel, |i, preview| {
        if let Some(card) = cards.get(i) {
            items.push(LayoutItem {
                data: spread_data(preview, index, &missing, &image, (false, SMALL)),
                title: card.title.as_str().into(),
                favorite: card.favorite,
            });
        }
    });
    ui.set_items(model(items));
    app.lists.panel = Some(key);
}

/// A prévia da opção (ou a lâmina real, com `None`), medida como um comando (spec MV, seção 4).
fn preview(app: &Shared, option: Option<usize>) {
    app.borrow_mut().timed(
        "Prévia do layout",
        |app| app.editor.hover_option(option),
        crate::crop::refresh_spread,
    );
}

/// Clique num cartão: aplica e a faixa continua aberta (regra 6). A faixa subindo não aplica.
fn apply(app: &Shared, i: i32) {
    let Ok(i) = usize::try_from(i) else {
        return;
    };
    if !app.borrow().editor.band().open {
        return;
    }
    app.borrow_mut().timed(
        "Aplicar o layout da faixa",
        |app| app.editor.choose_option(Part::Spread, i),
        |app| {
            app.refresh_strip();
            app.refresh_current();
        },
    );
}

/// ☆ e × num cartão (regra 9).
fn mark(app: &Shared, i: i32, favorite: bool) {
    let mut app = app.borrow_mut();
    let open = app.editor.band().open;
    if open && usize::try_from(i).is_ok_and(|i| app.editor.mark_option(i, favorite)) {
        app.refresh_strip();
        app.refresh_current();
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<BandUi>();
    let a = app.clone();
    ui.on_apply(move |i| apply(&a, i));
    let a = app.clone();
    ui.on_favorite(move |i| mark(&a, i, true));
    let a = app.clone();
    ui.on_reject(move |i| mark(&a, i, false));
    let a = app.clone();
    ui.on_list_changed(move |list| {
        let mut app = a.borrow_mut();
        if (list == 1) != app.editor.reordered() && app.editor.run(Command::ToggleReordered) {
            app.refresh_current();
        }
    });
    let a = app.clone();
    ui.on_close(move || {
        let mut app = a.borrow_mut();
        if app.editor.close_band() {
            app.refresh_current();
        }
    });
    connect_hover(&ui, app);
}

/// O mouse nos cartões: a prévia só depois de parar `REST` num cartão; no vão entre dois ela
/// fica; fora da faixa, a lâmina volta na hora (regra 5).
fn connect_hover(ui: &BandUi<'_>, app: &Shared) {
    let rest = Rc::new(Timer::default());
    let waiting: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    let (a, r, wait) = (app.clone(), rest.clone(), waiting.clone());
    ui.on_hover(move |i, inside| {
        let Ok(i) = usize::try_from(i) else {
            return;
        };
        if inside {
            wait.set(Some(i));
            let a = a.clone();
            r.start(TimerMode::SingleShot, REST, move || preview(&a, Some(i)));
        } else if wait.get() == Some(i) {
            wait.set(None);
            r.stop();
        }
    });
    let a = app.clone();
    ui.on_leave(move || {
        rest.stop();
        waiting.set(None);
        preview(&a, None);
    });
}

#[cfg(test)]
mod tests {
    use lamina_core::{AlbumSpec, Dpi, Mm, Project};
    use lamina_editor::{Command, Editor};

    /// As abas do painel da direita na ordem da tela, com o número de cada uma.
    fn tabs() -> Vec<(String, i32)> {
        let side = include_str!("../ui/side.slint");
        let start = side.find("export component PanelTabs").unwrap_or(0);
        side[start..]
            .lines()
            .take_while(|line| !line.starts_with('}'))
            .filter_map(|line| {
                let text = line.trim().strip_prefix("Tab { text: \"")?;
                let (label, rest) = text.split_once('"')?;
                let number = rest.split("root.tab == ").nth(1)?.split(';').next()?;
                Some((label.to_owned(), number.trim().parse().ok()?))
            })
            .collect()
    }

    #[test]
    fn com_a_faixa_aberta_a_roda_fora_dela_nao_mexe_no_rolo() {
        // A camada que fecha a faixa num clique fora também fica com a roda (medida da FL,
        // 2026-10-22): antes, a roda passava para o rolo da RL e a lâmina andava sob a prévia.
        let app = include_str!("../ui/app.slint");
        let start = app.find("BandUi.open: TouchArea {").unwrap_or(0);
        let layer: String = app[start..].lines().take_while(|l| *l != "    }").collect();
        assert!(layer.contains("BandUi.close()"), "a camada fecha a faixa");
        assert!(
            layer.contains("scroll-event(e) => { accept }"),
            "e engole a roda"
        );
    }

    #[test]
    fn fl_b9_o_painel_da_direita_sem_a_aba_layouts() {
        let tabs = tabs();
        let labels: Vec<&str> = tabs.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, ["Detalhes", "Aparência", "Quadro", "Histórico"]);
        let app = include_str!("../ui/app.slint");
        let start = app
            .split("in-out property <int> panel-tab:")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .map(str::trim);
        assert_eq!(start, Some("1"), "a aba de partida é declarada");
        assert_eq!(tabs[0].1, 1, "a de partida é Detalhes, a primeira");
        assert_eq!(crate::details::DETAILS_TAB, tabs[0].1);
        assert!(!include_str!("../ui/panel.slint").contains("Layouts"));
        // O recorte e o modo organizar continuam pedindo a aba Detalhes (AF-B23).
        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap());
        let mut project = Project::new("abas", spec.unwrap());
        let photo = project.add_photo("a.jpg", 3000, 2000);
        let mut editor = Editor::new();
        editor.adopt(project);
        editor.set_selection(vec![photo]);
        assert!(editor.run(Command::BuildFromSelection));
        assert!(editor.enter_crop(0) && editor.take_details_request());
        editor.exit_crop();
        assert!(editor.run(Command::Organize) && editor.take_details_request());
    }
}
