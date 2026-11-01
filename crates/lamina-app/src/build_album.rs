//! A janela "Montar o álbum" (spec BR): abre com a sugestão, confere o número de lâminas a cada
//! mudança e monta num trabalhador, com "Montando o álbum…" e o editor ocupado; a edição entra no
//! histórico quando a conta termina. As contas são do editor (`lamina_editor::building`).

use lamina_core::Project;
use lamina_editor::building::build_unused;
use lamina_editor::{Human, HumanMessage, guarded};
use lamina_layout::Reuse;
use slint::ComponentHandle;

use crate::state::{App, Msg};
use crate::{AppWindow, BuildUi};

type Shared = std::rc::Rc<std::cell::RefCell<App>>;

/// O nível de variedade da janela (os três da etapa 2 do Novo álbum).
fn reuse_of(index: i32) -> Reuse {
    match index {
        0 => Reuse::Low,
        2 => Reuse::High,
        _ => Reuse::Medium,
    }
}

/// "Montar o álbum…" pelo menu, pelo botão do painel de fotos ou pelo convite: abre a janela; sem
/// fotos não usadas, a mensagem da seção 6 (o item do menu fica sempre ativo; regra 12).
pub fn open(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let plan = match app.editor.album_build() {
        Ok(plan) => plan,
        Err(message) => {
            let human = HumanMessage::new(message, "", "");
            crate::files::show_error(app, &human);
            return;
        }
    };
    let ui = w.global::<BuildUi>();
    ui.set_what(plan.what().into());
    ui.set_suggestion(plan.suggestion_text().into());
    ui.set_most(i32::try_from(plan.unused.len()).unwrap_or(i32::MAX));
    ui.set_spreads(i32::try_from(plan.suggestion).unwrap_or(1));
    refresh(app);
    ui.set_show(true);
}

/// O número de lâminas mudou: a mensagem do plano (Montar desabilitado) ou o resultado.
fn refresh(app: &App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    let ui = w.global::<BuildUi>();
    let Ok(plan) = app.editor.album_build() else {
        return;
    };
    let spreads = usize::try_from(ui.get_spreads()).unwrap_or(0);
    let project = session.album();
    match plan.check(project, spreads) {
        Ok(()) => {
            let (line, warning) = plan.result(project, spreads);
            ui.set_error("".into());
            ui.set_result(line.into());
            ui.set_warning(warning.unwrap_or_default().into());
        }
        Err(e) => ui.set_error(e.human().line().into()),
    }
}

/// Montar: a conta num trabalhador (com centenas de fotos leva segundos; regra 11).
fn start(app: &mut App) {
    let (Some(w), Some(session)) = (app.window.upgrade(), app.session()) else {
        return;
    };
    let ui = w.global::<BuildUi>();
    let Ok(plan) = app.editor.album_build() else {
        return;
    };
    let spreads = usize::try_from(ui.get_spreads()).unwrap_or(0);
    if plan.check(session.album(), spreads).is_err() {
        return;
    }
    let project = session.album_shared();
    let options = app.editor.build_options(reuse_of(ui.get_reuse()));
    let photos = plan.unused.len();
    ui.set_show(false);
    w.set_busy_text("Montando o álbum…".into());
    app.editor.set_busy(true);
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        // Uma falha inesperada também libera a janela (spec EH, regra 6).
        let result = guarded(|| {
            build_unused(&project, spreads, &options)
                .map(|(built, _)| (built, spreads, photos))
                .map_err(|e| e.human())
        })
        .and_then(|r| r);
        // Se a janela já fechou, não há a quem avisar.
        let _ = tx.send(Msg::Built(result));
    });
}

/// A conta terminou: a edição entra no histórico e a janela mostra a primeira lâmina nova.
pub fn finished(app: &mut App, result: Result<(Project, usize, usize), HumanMessage>) {
    if let Some(w) = app.window.upgrade() {
        w.set_busy_text("".into());
        w.invoke_focus_main();
    }
    app.editor.set_busy(false);
    match result {
        Ok((built, spreads, photos)) => {
            if app.editor.finish_build(built, spreads, photos) {
                app.refresh_all();
            }
        }
        Err(e) => crate::files::show_error(app, &e),
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<BuildUi>();
    let a = app.clone();
    ui.on_changed(move || refresh(&a.borrow()));
    let a = app.clone();
    ui.on_build(move || start(&mut a.borrow_mut()));
    let w = window.as_weak();
    ui.on_closed(move || {
        if let Some(w) = w.upgrade() {
            w.invoke_focus_main();
        }
    });
}
