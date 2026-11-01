//! Interface gráfica do Lamina (spec `janela-principal`; ADR 0002).
//!
//! Este binário só traduz (regra 1): as telas estão em `ui/*.slint` e toda decisão vem do
//! `lamina-editor`. O trabalho longo roda em outras linhas e volta por um canal, lido a cada
//! 50 ms pela linha da janela.

#![windows_subsystem = "windows"]

mod adjusted;
mod backup;
mod band;
mod browser;
mod build_album;
mod calibrate;
mod clear_all;
mod context;
mod convert;
mod cover;
mod crop;
mod details;
mod dialogs;
mod drag_motion;
mod drops;
mod export;
mod files;
mod font_list;
mod frame_panel;
mod ghost;
mod grid;
mod hand;
#[cfg(any(test, feature = "filmagem"))]
#[cfg_attr(not(test), allow(dead_code))]
mod headless;
mod help;
mod history;
mod images;
mod importing;
mod input;
mod install;
mod line_window;
mod lists;
mod loader;
mod look;
mod mailbox;
mod models;
mod monitor;
mod motion;
mod msg;
mod new_album;
mod new_project;
mod new_project_fields;
mod new_project_file;
mod new_project_mount;
mod new_project_size;
mod news;
mod news_player;
mod number_field;
mod opened;
mod organize;
mod palette;
mod panel_width;
mod photo_menu;
mod photo_panel;
mod presets;
mod recents;
mod refresh;
mod relink;
mod relink_dialog;
mod roll;
mod roll_cells;
mod roll_connect;
mod roll_stats;
mod roll_worker;
mod shortcuts;
mod slots;
mod spec_dialog;
mod state;
mod texts;
mod thumb_loader;
mod thumbs;
mod timings;
mod updates;
mod verify;
mod waits;
mod welcome;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lamina_editor::backup::{BACKUP_FOLDER, Backups};
use lamina_editor::{
    Cache, Editor, Install, MODELS_FILE, ModelsStore, PREFS_FILE, PRESETS_FILE, PrefsStore,
    PresetsStore, RECENTS_FILE, RecentsStore, STRIPS_FILE, StripsStore, config_dir,
};
use slint::{CloseRequestResponse, ComponentHandle, Timer};

use crate::state::{App, Pending};

/// As telas compiladas pelo Slint. É código gerado: só aqui o `unsafe` é liberado.
#[allow(
    unsafe_code,
    missing_debug_implementations,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
mod ui {
    slint::include_modules!();
}

pub use ui::*;

type Shared = Rc<RefCell<App>>;

fn connect(window: &AppWindow, app: &Shared) {
    input::connect(window, app);
    shortcuts::connect(window);
    news::connect(window);
    let a = app.clone();
    window.global::<NewsUi>().on_open_video(move |address| {
        help::open_video(&a.borrow(), &address);
    });
    let a = app.clone();
    window.on_go_to(move |index| {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        a.borrow_mut().timed(
            "Ir para a lâmina",
            |app| app.editor.go_to_spread(index),
            App::refresh_current,
        );
    });
    browser::connect(window, app);
    build_album::connect(window, app);
    grid::connect(window, app);
    connect_dialogs(window, app);
    band::connect(window, app);
    drops::connect(window, app);
    drag_motion::connect_motion(window, app);
    organize::connect(window, app);
    context::connect(window, app);
    cover::connect(window, app);
    backup::connect(window, app);
    palette::connect(window, app);
    panel_width::connect(window, app);
    photo_panel::connect(window, app);
    crop::connect(window, app);
    look::connect(window, app);
    hand::connect(window, app);
    frame_panel::connect(window, app);
    font_list::connect(window, app);
    models::connect(window, app);
    history::connect(window, app);
    recents::connect(window, app);
    clear_all::connect(window, app);
    relink_dialog::connect(window, app);
    photo_menu::connect(window, app);
    number_field::connect(window);
    importing::connect(window, app);
    calibrate::connect(window, app);
    roll_connect::connect(window, app);
}

fn connect_dialogs(window: &AppWindow, app: &Shared) {
    new_project::connect(window, app);
    let a = app.clone();
    window.on_confirm(move |choice| files::answer(&mut a.borrow_mut(), choice));
    connect_export(window, app);
    presets::connect(window, app);
    let a = app.clone();
    window.on_update_restart(move || {
        let mut app = a.borrow_mut();
        if app.updates.begin_restart() {
            files::request(&mut app, Pending::Update);
        }
    });
    let a = app.clone();
    window.on_update_dismiss(move || updates::dismiss(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_notice_dismiss(move || a.borrow_mut().dismiss_notice());
    let a = app.clone();
    window.window().on_close_requested(move || {
        let mut app = a.borrow_mut();
        if app.editor.needs_confirmation() {
            files::request(&mut app, Pending::Close);
            CloseRequestResponse::KeepWindowShown
        } else {
            CloseRequestResponse::HideWindow
        }
    });
}

/// A janela Exportar e o resumo com "Abrir a pasta" (spec EI).
fn connect_export(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_ex_pick_folder(move || export::pick_folder(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_changed(move || export::changed(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_export(move || export::export(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_go_to(move |i| export::go_to(&mut a.borrow_mut(), i));
    let a = app.clone();
    window.on_ex_ignore(move |i| export::ignore(&mut a.borrow_mut(), i));
    let a = app.clone();
    window.on_ex_proceed(move || export::proceed(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_ex_replace(move || export::run(&mut a.borrow_mut(), true));
    let a = app.clone();
    window.on_ex_back(move || export::back(&mut a.borrow_mut()));
    let a = app.clone();
    window.on_open_folder(move || export::open_folder(&a.borrow()));
}

/// Os ganchos do Velopack (spec DS, regra 10): instalar, atualizar e desinstalar chamam o
/// Lamina com `--veloapp-*`, que responde e sai sem janela. Precisa ser o primeiro passo.
fn velopack_hooks() {
    let app = velopack::VelopackApp::build();
    #[cfg(windows)]
    let app = app
        .on_after_install_fast_callback(|_| install::register())
        .on_after_update_fast_callback(|_| install::register())
        .on_before_uninstall_fast_callback(|_| install::unregister());
    let mut app = app;
    app.run();
}

fn main() -> anyhow::Result<()> {
    velopack_hooks();
    let verifying = verify::requested();
    if let Some(album) = &verifying
        && let Err(reason) = verify::check(album)
    {
        eprintln!("--verificar: {reason}");
        std::process::exit(1);
    }
    let importing = verify::import_requested();
    if let Some(folder) = &importing
        && let Err(reason) = verify::check_import(folder)
    {
        eprintln!("--verificar-importacao: {reason}");
        std::process::exit(1);
    }
    // A verificação da abertura (spec MA): sem o relatório, sai com 1 antes da janela.
    let opening = opened::requested();
    #[cfg(target_os = "linux")]
    if verifying.is_none() {
        install::linux_desktop();
    }
    // Medir na máquina compartilhada (`LAMINA_SEM_FOCO=1`, portão G1 da spec RL): a janela abre
    // sem tomar o foco, e o teclado de quem está trabalhando em outra janela continua lá.
    // Abre maximizada: numa tela pequena, as faixas de baixo ficariam fora (auditoria da Fase 4, 7).
    // Pedida ao criar a janela, não depois: maximizar a janela antes de ela existir no sistema
    // deixava o Windows sem o tamanho de "restaurar" e o primeiro maximizar falhava (spec FB,
    // regra 5).
    let unfocused = std::env::var("LAMINA_SEM_FOCO").is_ok_and(|v| v == "1");
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(move |a| a.with_maximized(true).with_active(!unfocused))
        .select()?;
    let window = AppWindow::new()?;
    // `LAMINA_GALERIA=1`: abre a galeria de componentes por cima da janela (spec DF, saída 6).
    window.set_gallery(std::env::var("LAMINA_GALERIA").is_ok_and(|v| v == "1"));
    let (tx, rx) = mailbox::Mailbox::new();
    let prefs = PrefsStore::load(&config_dir().join(PREFS_FILE));
    let mut editor = Editor::with_prefs(prefs);
    // `atualizacoes.json` antes de `janela.json`: sem ele, é uma instalação nova e o painel da
    // direita começa escondido (spec JE, regras 2 e 3).
    let opened = updates::state();
    let install = Install::from_last_version(opened.store.prefs.last_version.as_deref());
    editor.set_strips(StripsStore::load_for(
        &config_dir().join(STRIPS_FILE),
        install,
    ));
    editor.set_recents(RecentsStore::load(&config_dir().join(RECENTS_FILE)));
    editor.set_log(help::error_log());
    editor.set_backups(Backups::new(config_dir().join(BACKUP_FOLDER)));
    let app: Shared = Rc::new(RefCell::new(App::new(&window, Cache::system(), editor, tx)));
    // Ligada por `LAMINA_TEMPOS=1`; o temporizador precisa viver até a janela fechar.
    let (clock, _tick) = timings::Clock::install(window.window());
    {
        let mut a = app.borrow_mut();
        a.clock = clock;
        a.presets = PresetsStore::load(&config_dir().join(PRESETS_FILE));
        let models = ModelsStore::load(&config_dir().join(MODELS_FILE));
        a.editor.set_models(models);
        if let Some(warning) = a.presets.warning.take() {
            a.notice = warning;
        }
        a.updates = opened;
        help::start(&a);
        // As novidades da versão nova, uma vez, antes de a janela aparecer (spec NV, regra 3); o
        // teste de fumaça não é interrompido.
        if verifying.is_none() && importing.is_none() {
            news::on_start(&mut a);
        }
        updates::show(&a);
    }
    connect(&window, &app);
    app.borrow_mut().refresh_all();
    recents::refresh(&app.borrow());
    recents::check(&app.borrow());
    // `lamina-app projeto.lamina` abre o projeto direto: o duplo clique no Windows e no Linux e a
    // volta depois de atualizar (spec DS, regra 5; spec MA, regra 9). No Mac, chega por evento.
    if let Some(album) = &verifying {
        files::open_file(&mut app.borrow_mut(), album);
    } else if importing.is_none() && opening.is_none() {
        opened::from_args(&mut app.borrow_mut());
    }
    // O trabalho que ficou sem salvar numa queda (spec CS, regra 4); os testes de fumaça seguem.
    if verifying.is_none() && importing.is_none() {
        backup::ask(&mut app.borrow_mut());
    }
    let _backups = backup::start(&app);
    // O teste de fumaça da importação (spec TR): fecha sozinho quando as fotos entram.
    if let Some(folder) = importing.clone()
        && let Err(reason) = verify::start_import(&mut app.borrow_mut(), folder)
    {
        eprintln!("--verificar-importacao: {reason}");
        std::process::exit(1);
    }
    let a = app.clone();
    if verifying.is_some() {
        Timer::single_shot(verify::SHOW_FOR, move || {
            // Fechando: um erro aqui só deixa a janela aberta até o tempo limite do CI.
            let _ = slint::quit_event_loop();
        });
    } else if importing.is_none() && opening.is_none() {
        // Os testes de fumaça não procuram atualização: não dependem da rede.
        Timer::single_shot(updates::CHECK_DELAY, move || {
            updates::on_open(&mut a.borrow_mut());
        });
    }

    // As mensagens acordam a janela na hora, sem conferência repetida (spec LV, regra 7). Esta
    // única pega as que chegaram antes de a caixa estar ligada.
    mailbox::install(rx, &app);
    Timer::single_shot(Duration::ZERO, mailbox::drain);
    // Os arquivos que o sistema entrega, com o Lamina aberto ou abrindo (spec MA).
    let _listener = opened::listen(&app, opening);
    window.invoke_focus_main();
    window.run()?;
    // Sair do Lamina guarda a lâmina em que o fotógrafo parou (spec TI, regra 3) e não deixa
    // cópia de segurança: a pergunta de salvar já foi respondida (spec CS, regra 3).
    app.borrow_mut().editor.remember_current();
    app.borrow_mut().editor.finish_backups();
    updates::on_exit(&app.borrow());
    Ok(())
}
