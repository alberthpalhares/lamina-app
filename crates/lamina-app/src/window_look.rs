//! A aparência da janela (spec AJ): liga a tela de Preferências ao editor e aplica o tema, o fundo, o
//! tamanho do texto e a barra de título. A leitura do sistema segue o ADR 0016: o tema vem de um
//! componente que nunca escreve no `Palette`, e o movimento, do editor, fora da linha da janela.

use lamina_editor::motion::Durations;
use lamina_editor::window_look::{
    BackdropChoice, MotionChoice, TEXT_SIZES, ThemeChoice, WindowLook, menu_differs, scale_of,
    system_is_dark, system_reduces_motion,
};
use slint::ComponentHandle;
use slint::winit_030::{WinitWindowAccessor, winit};

use crate::msg::Msg;
use crate::state::App;
use crate::{AppWindow, Backdrop, PrefsUi, Shared, SystemScheme, Theme};

const THEMES: [ThemeChoice; 3] = [ThemeChoice::Dark, ThemeChoice::Light, ThemeChoice::System];
const BACKDROPS: [BackdropChoice; 4] = [
    BackdropChoice::Theme,
    BackdropChoice::Dark,
    BackdropChoice::Gray,
    BackdropChoice::Light,
];
const MOTIONS: [MotionChoice; 3] = [MotionChoice::System, MotionChoice::On, MotionChoice::Off];

fn position<T: PartialEq>(list: &[T], value: T) -> i32 {
    list.iter()
        .position(|v| *v == value)
        .and_then(|i| i32::try_from(i).ok())
        .unwrap_or(0)
}

fn pick<T: Copy>(list: &[T], index: i32) -> Option<T> {
    usize::try_from(index)
        .ok()
        .and_then(|i| list.get(i))
        .copied()
}

/// O tema pedido à barra de título (regra 10): `None` deixa com o sistema.
pub fn title_theme(choice: ThemeChoice) -> Option<winit::window::Theme> {
    match choice {
        ThemeChoice::Dark => Some(winit::window::Theme::Dark),
        ThemeChoice::Light => Some(winit::window::Theme::Light),
        ThemeChoice::System => None,
    }
}

/// O que aparece embaixo do tema: o que "Seguir o sistema" resolveu (regra 4), e o aviso de que a barra de
/// menus só muda ao abrir de novo (regra 34).
fn theme_note(choice: ThemeChoice, system: Option<bool>, menu_stays: bool) -> String {
    let now = match (choice, choice.is_dark(system)) {
        (ThemeChoice::System, true) => "Agora: escuro",
        (ThemeChoice::System, false) => "Agora: claro",
        _ => "",
    };
    let menu = if menu_stays {
        "A barra de menus (Arquivo, Editar…) muda quando você abrir o Lamina de novo."
    } else {
        ""
    };
    if now.is_empty() || menu.is_empty() {
        format!("{now}{menu}")
    } else {
        format!("{now}. {menu}")
    }
}

/// O que `Seguir o sistema` mostra embaixo do movimento (regra 4).
fn motion_note(choice: MotionChoice, system: Option<bool>) -> &'static str {
    match (choice, choice.reduces(system)) {
        (MotionChoice::System, true) => "Agora: sem movimento",
        (MotionChoice::System, false) => "Agora: com movimento",
        _ => "",
    }
}

/// Copia as escolhas para os globais da janela.
fn paint(app: &App, w: &AppWindow, look: WindowLook) {
    let (system_dark, system_motion) = (app.system_dark, app.editor.system_motion());
    let dark = look.theme.is_dark(system_dark);
    let theme = w.global::<Theme>();
    theme.set_dark(dark);
    theme.set_text_scale(scale_of(look.text_size));
    w.global::<Backdrop>()
        .set_choice(position(&BACKDROPS, look.backdrop));
    let ui = w.global::<PrefsUi>();
    ui.set_theme(position(&THEMES, look.theme));
    ui.set_backdrop(position(&BACKDROPS, look.backdrop));
    ui.set_size(position(&TEXT_SIZES, look.text_size));
    ui.set_motion(position(&MOTIONS, look.motion));
    let menu_stays = cfg!(windows) && menu_differs(dark, app.opened_dark, system_dark);
    ui.set_theme_note(theme_note(look.theme, system_dark, menu_stays).into());
    ui.set_motion_note(motion_note(look.motion, system_motion).into());
    ui.set_reduced(look.motion.reduces(system_motion));
    ui.set_demo_ms(i32::try_from(Durations::new(false).navigate).unwrap_or(0));
}

/// Aplica o que o editor tem agora. Na abertura, vale antes do primeiro desenho (regra 26).
pub fn apply(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let look = app.editor.window_look();
    paint(app, &w, look);
    crate::motion::refresh(&w, app.editor.durations(), app.editor.reduce_motion());
    apply_title(&w, look.theme);
}

/// A barra de título (ADR 0016, decisão 2). Antes de a janela existir, não faz nada.
fn apply_title(w: &AppWindow, choice: ThemeChoice) {
    let theme = title_theme(choice);
    w.window()
        .with_winit_window(|window| window.set_theme(theme));
}

/// O que o componente de leitura diz do sistema agora; desconhecido antes de a janela existir.
fn scheme_answer(scheme: &SystemScheme) -> Option<bool> {
    match (scheme.get_dark(), scheme.get_light()) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}

/// Lê o sistema de novo: ao abrir, ao mudar e ao voltar ao primeiro plano (regras 9 e 18).
fn read_scheme(app: &mut App) {
    let answer = system_is_dark().or_else(|| app.scheme.as_ref().and_then(scheme_answer));
    if answer.is_some() {
        app.system_dark = answer;
    }
    apply(app);
}

/// O tema que a janela nasce pedindo (regra 34): a barra de menus do Windows fica com ele. Fora do Windows,
/// o sistema decide.
pub fn opening_theme(choice: ThemeChoice) -> Option<winit::window::Theme> {
    if cfg!(windows) {
        title_theme(choice)
    } else {
        None
    }
}

/// Se o tema pedido ao abrir era escuro; `None` = seguindo o sistema (regra 34).
pub fn opened_dark(choice: ThemeChoice) -> Option<bool> {
    match opening_theme(choice) {
        Some(winit::window::Theme::Dark) => Some(true),
        Some(winit::window::Theme::Light) => Some(false),
        None => None,
    }
}

/// Pergunta ao sistema sobre o movimento, fora da linha da janela (ADR 0016, decisão 3).
pub fn read_motion(app: &App) {
    // Só "Seguir o sistema" usa a resposta.
    if app.editor.window_look().motion != MotionChoice::System {
        return;
    }
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        // Janela fechando: a resposta não tem quem receba.
        let _ = tx.send(Msg::SystemMotion(system_reduces_motion()));
    });
}

/// A resposta do sistema sobre o movimento chegou.
pub fn motion_read(app: &mut App, reduces: Option<bool>) {
    app.editor.set_system_motion(reduces);
    apply(app);
}

/// O que interessa aqui nos eventos da janela. Chamado pelo gancho de `importing::connect`.
pub fn on_event(app: &Shared, event: &winit::event::WindowEvent) {
    let Ok(mut a) = app.try_borrow_mut() else {
        return;
    };
    match event {
        // O valor do processo só muda depois deste gancho: vale o que o evento traz.
        winit::event::WindowEvent::ThemeChanged(theme) => {
            a.system_dark = Some(*theme == winit::window::Theme::Dark);
            apply(&a);
        }
        // O Windows devolve a barra à cor do sistema a cada aviso de configuração (ADR 0016).
        winit::event::WindowEvent::Focused(true) => {
            read_scheme(&mut a);
            read_motion(&a);
        }
        _ => {}
    }
}

/// Abre a tela (regra 1), pelo menu ou por `Ctrl+,`.
pub fn open(app: &App) {
    apply(app);
    if let Some(w) = app.window.upgrade() {
        w.global::<PrefsUi>().set_show(true);
    }
}

/// Depois de uma escolha: a janela muda, e um erro ao gravar vira aviso (regra 6).
fn changed(app: &mut App) {
    apply(app);
    if let Some(notice) = app.editor.take_notice() {
        app.notice = notice;
        if let Some(w) = app.window.upgrade() {
            w.set_notice(app.notice.clone().into());
        }
    }
}

/// Liga os botões da tela e o componente que lê o sistema.
pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PrefsUi>();
    let a = app.clone();
    ui.on_theme_picked(move |i| {
        let mut app = a.borrow_mut();
        if let Some(choice) = pick(&THEMES, i) {
            app.editor.set_theme_choice(choice);
            // Sair do tema forçado pede a resposta de agora (regra 9).
            changed(&mut app);
            read_scheme(&mut app);
        }
    });
    let a = app.clone();
    ui.on_backdrop_picked(move |i| {
        let mut app = a.borrow_mut();
        if let Some(choice) = pick(&BACKDROPS, i) {
            app.editor.set_backdrop_choice(choice);
            changed(&mut app);
        }
    });
    let a = app.clone();
    ui.on_size_picked(move |i| {
        let mut app = a.borrow_mut();
        if let Some(size) = pick(&TEXT_SIZES, i) {
            app.editor.set_text_size(size);
            changed(&mut app);
        }
    });
    let a = app.clone();
    ui.on_motion_picked(move |i| {
        let mut app = a.borrow_mut();
        if let Some(choice) = pick(&MOTIONS, i) {
            app.editor.set_motion_choice(choice);
            changed(&mut app);
            read_motion(&app);
        }
    });
    let a = app.clone();
    ui.on_reset(move || {
        let mut app = a.borrow_mut();
        app.editor.reset_window_look();
        changed(&mut app);
    });
    let scheme = SystemScheme::new().ok();
    if let Some(scheme) = &scheme {
        let a = app.clone();
        scheme.on_answered(move || {
            if let Ok(mut app) = a.try_borrow_mut() {
                read_scheme(&mut app);
            }
        });
    }
    let mut a = app.borrow_mut();
    a.scheme = scheme;
    apply(&a);
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_focus;
