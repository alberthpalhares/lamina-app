//! Cenários da spec AJ que precisam da janela de verdade, desenhada sem tela. O teste injeta a resposta
//! do sistema: sem janela do sistema, ele nunca responde (ADR 0016). O texto dos `.slint` é conferido em
//! `crates/lamina-app/tests/aj.rs`, e a lógica, no `lamina-editor` (`tests/aj.rs`).

use std::path::{Path, PathBuf};

use lamina_editor::commands::Command;
use lamina_editor::{Cache, Editor, Install, PrefsStore, StripsStore, prepare_example};
use slint::platform::{Key, WindowEvent};
use slint::{Rgb8Pixel, SharedString};

use super::*;
use crate::headless::{self, Screen};
use crate::mailbox::{self, Mailbox};
use crate::{BuildUi, CalibrateUi, DialogFocus, ModelUi};
use crate::{PhotoMenuUi, PresetUi, RecentsUi, RecoverUi, Type};

pub(super) const NOTEBOOK: (u32, u32) = (1366, 668);
const SMALL: (u32, u32) = (800, 600);

/// A janela ligada ao editor, com `janela.json` em `dir`, como o `main` faz.
pub(super) fn session(size: (u32, u32), dir: &Path, album: Option<&Path>) -> (Screen, Shared) {
    let screen = headless::open(size.0, size.1).unwrap();
    let (tx, rx) = Mailbox::new();
    let mut editor = Editor::with_prefs(PrefsStore::load(&dir.join("prefs.json")));
    editor.set_strips(StripsStore::load_for(
        &dir.join("janela.json"),
        Install::Existing,
    ));
    let app: Shared = std::rc::Rc::new(std::cell::RefCell::new(App::new(
        &screen.app,
        Cache::system(),
        editor,
        tx,
    )));
    crate::connect(&screen.app, &app);
    app.borrow_mut().refresh_all();
    if let Some(album) = album {
        crate::files::open_file(&mut app.borrow_mut(), album);
    }
    mailbox::install(rx, &app);
    (screen, app)
}

/// O álbum de exemplo copiado para `dir`.
pub(super) fn example(dir: &Path) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(lamina_editor::EXAMPLE_DIR);
    prepare_example(&source, &dir.join("exemplo")).unwrap()
}

/// Espera as imagens chegarem e os movimentos terminarem.
pub(super) fn settle(screen: &Screen) {
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mailbox::drain();
        screen.draw();
    }
}

pub(super) fn press(screen: &Screen, key: Key) {
    let text = SharedString::from(key);
    screen
        .window
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    screen
        .window
        .dispatch_event(WindowEvent::KeyReleased { text });
}

fn pixel(screen: &Screen, pixels: &[Rgb8Pixel], x: u32, y: u32) -> (u8, u8, u8) {
    let p = pixels[(y * screen.width() + x) as usize];
    (p.r, p.g, p.b)
}

/// Escolhe como a pessoa escolhe: passa pelo editor, e o arquivo fica para a próxima janela.
fn choose(screen: &Screen, theme: i32, backdrop: i32, size: i32) {
    let ui = screen.app.global::<PrefsUi>();
    ui.invoke_theme_picked(theme);
    ui.invoke_backdrop_picked(backdrop);
    ui.invoke_size_picked(size);
}

fn save(screen: &Screen, name: &str) {
    let Ok(folder) = std::env::var("LAMINA_CAPTURAS") else {
        return;
    };
    let pixels = screen.draw();
    let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
    image::RgbImage::from_raw(screen.width(), screen.height(), bytes)
        .unwrap()
        .save(format!("{folder}/{name}.png"))
        .unwrap();
}

/// Abre uma tela da lista da regra 11.
pub(super) type Opener = fn(&Screen, &Shared);

fn run(app: &Shared, command: Command) {
    crate::input::run_command(app, command);
}

/// As telas da regra 11. Os nomes conferem com os diálogos que herdam de `Dialog`.
pub(super) const SCREENS: &[(&str, Opener)] = &[
    ("AboutDialog", |_, a| run(a, Command::About)),
    ("BuildAlbumDialog", |s, _| {
        s.app.global::<BuildUi>().set_show(true)
    }),
    ("CalibrateDialog", |s, _| {
        s.app.global::<CalibrateUi>().set_show(true)
    }),
    ("ClearAllDialog", |_, a| run(a, Command::ClearAllPhotos)),
    ("CoverDialog", |_, a| run(a, Command::CreateCover)),
    ("ConfirmDialog", |s, _| s.app.set_show_confirm(true)),
    ("MessageDialog", |s, _| {
        s.app
            .set_message_title("Não foi possível abrir o álbum.".into());
        s.app
            .set_message_text("Confira se o arquivo está na pasta.".into());
        s.app.set_show_message(true);
    }),
    ("BusyOverlay", |s, _| {
        s.app.set_busy_text("Montando o álbum…".into())
    }),
    ("RecoverDialog", |s, _| {
        s.app.global::<RecoverUi>().set_show(true)
    }),
    ("ExportDialog", |_, a| run(a, Command::Export)),
    ("ModelNameDialog", |s, _| {
        s.app.global::<ModelUi>().set_show_name(true)
    }),
    ("ModelsDialog", |s, _| {
        s.app.global::<ModelUi>().set_show_list(true)
    }),
    ("NewProjectDialog", |_, a| run(a, Command::Specification)),
    ("RemoveAskDialog", |s, _| {
        s.app.global::<PhotoMenuUi>().set_show_ask(true)
    }),
    ("RelinkDialog", |_, a| run(a, Command::FindMissing)),
    ("ShortcutsDialog", |_, a| run(a, Command::ShowShortcuts)),
    ("PreferencesDialog", |_, a| run(a, Command::Preferences)),
    ("PresetNameDialog", |s, _| {
        s.app.global::<PresetUi>().set_show_name(true)
    }),
    ("PresetsDialog", |s, _| {
        s.app.global::<PresetUi>().set_show_list(true)
    }),
    ("ClearRecentsDialog", |s, _| {
        s.app.global::<RecentsUi>().set_show_clear(true)
    }),
];

/// Os nomes de quem herda de `Dialog` nos `.slint`.
fn dialogs_in_the_ui() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "slint") {
            let text = std::fs::read_to_string(path).unwrap();
            names.extend(text.lines().filter_map(|l| {
                let rest = l.strip_prefix("export component ")?;
                let (name, tail) = rest.split_once(' ')?;
                tail.starts_with("inherits Dialog ")
                    .then(|| name.to_owned())
            }));
        }
    }
    names.sort();
    names
}

/// Abre cada tela nesta combinação, confere que coube e guarda a imagem.
fn every_screen(size: (u32, u32), theme: i32, text: i32, tag: &str) {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    // Reencontrar precisa de uma foto faltando (regra 27).
    let missing_dir = tempfile::tempdir().unwrap();
    let missing = example(missing_dir.path());
    remove_one_photo(&missing);
    let mut late = Vec::new();
    for (name, open) in SCREENS {
        let opened = if *name == "RelinkDialog" {
            &missing
        } else {
            &album
        };
        let (screen, app) = session(size, dir.path(), Some(opened));
        choose(&screen, theme, 0, text);
        screen.draw();
        open(&screen, &app);
        screen.draw();
        assert_eq!(
            screen.app.global::<Theme>().get_text_scale(),
            lamina_editor::window_look::scale_of(TEXT_SIZES[usize::try_from(text).unwrap()]),
            "{name} refez o tamanho"
        );
        if !screen.app.global::<DialogFocus>().get_fits() {
            late.push(*name);
        }
        save(&screen, &format!("{tag}-{name}"));
    }
    assert_eq!(late, Vec::<&str>::new(), "{tag}: o rodapé saiu da janela");
}

/// Apaga uma foto da pasta do álbum.
pub(super) fn remove_one_photo(album: &Path) {
    let photos = album.parent().unwrap().join("fotos");
    let first = std::fs::read_dir(photos).unwrap().next().unwrap().unwrap();
    std::fs::remove_file(first.path()).unwrap();
}

#[test]
fn aj_b3_teclado_na_tela() {
    let dir = tempfile::tempdir().unwrap();
    let (screen, app) = session(NOTEBOOK, dir.path(), None);
    run(&app, Command::Preferences);
    settle(&screen);
    let ui = screen.app.global::<PrefsUi>();
    assert!(ui.get_show());
    // O foco entra na escolha atual do Tema; a seta anda sem escolher, e Enter escolhe.
    press(&screen, Key::RightArrow);
    settle(&screen);
    assert_eq!(app.borrow().editor.window_look().theme, ThemeChoice::Dark);
    press(&screen, Key::Return);
    assert_eq!(app.borrow().editor.window_look().theme, ThemeChoice::Light);
    assert!(ui.get_show());
    // Mais Tabs do que paradas: nada fecha, e o foco continua na tela.
    for _ in 0..40 {
        press(&screen, Key::Tab);
    }
    assert!(ui.get_show());
    press(&screen, Key::Escape);
    screen.draw();
    assert!(!ui.get_show());
}

#[test]
fn aj_b4_vale_na_hora_sem_fechar() {
    let dir = tempfile::tempdir().unwrap();
    let (screen, app) = session(NOTEBOOK, dir.path(), None);
    run(&app, Command::Preferences);
    screen.draw();
    let ui = screen.app.global::<PrefsUi>();
    assert!(screen.app.global::<Theme>().get_dark());
    ui.invoke_theme_picked(1);
    assert!(!screen.app.global::<Theme>().get_dark());
    assert_eq!(ui.get_theme(), 1);
    assert!(ui.get_show());
}

#[test]
fn aj_b10_seguir_o_sistema_na_janela() {
    let dir = tempfile::tempdir().unwrap();
    let (screen, app) = session(NOTEBOOK, dir.path(), None);
    let ui = screen.app.global::<PrefsUi>();
    ui.invoke_theme_picked(2);
    let theme = screen.app.global::<Theme>();
    for (answer, dark, note) in [
        (Some(false), false, "Agora: claro"),
        (Some(true), true, "Agora: escuro"),
    ] {
        let mut a = app.borrow_mut();
        a.system_dark = answer;
        apply(&a);
        drop(a);
        assert_eq!(theme.get_dark(), dark);
        assert_eq!(ui.get_theme_note(), note);
    }
}

#[test]
fn aj_b11_a_barra_de_titulo_acompanha() {
    assert_eq!(
        title_theme(ThemeChoice::Dark),
        Some(winit::window::Theme::Dark)
    );
    assert_eq!(
        title_theme(ThemeChoice::Light),
        Some(winit::window::Theme::Light)
    );
    assert_eq!(title_theme(ThemeChoice::System), None);
}

#[test]
fn aj_b12_todas_as_telas_nos_dois_temas_e_em_130() {
    let mut listed: Vec<String> = SCREENS.iter().map(|(n, _)| (*n).to_owned()).collect();
    listed.sort();
    assert_eq!(listed, dialogs_in_the_ui(), "tela nova fora da lista");
    every_screen(NOTEBOOK, 0, 1, "escuro-100");
    every_screen(NOTEBOOK, 1, 1, "claro-100");
    every_screen(NOTEBOOK, 0, 4, "escuro-130");
    every_screen(NOTEBOOK, 1, 4, "claro-130");
}

#[test]
fn aj_b13_janela_pequena() {
    every_screen(SMALL, 0, 4, "pequena-escuro-130");
    every_screen(SMALL, 1, 4, "pequena-claro-130");
}

/// Os pixels de dentro da lâmina na janela de 1366 × 668.
fn inside_the_spread(screen: &Screen, pixels: &[Rgb8Pixel]) -> Vec<(u8, u8, u8)> {
    let mut out = Vec::new();
    for y in 130..340 {
        for x in 300..720 {
            out.push(pixel(screen, pixels, x, y));
        }
    }
    out
}

#[test]
fn aj_b14_a_lamina_nao_muda() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session(NOTEBOOK, dir.path(), Some(&album));
    settle(&screen);
    let reference = inside_the_spread(&screen, &screen.draw());
    let ui = screen.app.global::<PrefsUi>();
    for theme in 0..2 {
        for backdrop in 0..4 {
            for size in [1, 4] {
                ui.invoke_theme_picked(theme);
                ui.invoke_backdrop_picked(backdrop);
                ui.invoke_size_picked(size);
                app.borrow_mut().refresh_all();
                settle(&screen);
                let now = inside_the_spread(&screen, &screen.draw());
                assert!(
                    now == reference,
                    "mudou com tema {theme}, fundo {backdrop} e tamanho {size}"
                );
            }
        }
    }
}

#[test]
fn aj_b17_o_fundo_vale_onde_deve() {
    const GRAY: (u8, u8, u8) = (0x77, 0x77, 0x77);
    const LIGHT: (u8, u8, u8) = (0xdc, 0xdc, 0xdc);
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session(NOTEBOOK, dir.path(), Some(&album));
    let ui = screen.app.global::<PrefsUi>();
    ui.invoke_theme_picked(1);
    ui.invoke_backdrop_picked(2);
    settle(&screen);
    let at = |screen: &Screen| {
        let pixels = screen.draw();
        pixel(screen, &pixels, 20, 110)
    };
    assert_eq!(at(&screen), GRAY, "na lâmina");
    // As faixas seguem o tema: a caixa da capa, sem fundo próprio, não mostra o fundo da lâmina.
    let pixels = screen.draw();
    assert_ne!(pixel(&screen, &pixels, 3, 500), GRAY, "na faixa");
    run(&app, Command::Organize);
    settle(&screen);
    let pixels = screen.draw();
    assert_eq!(pixel(&screen, &pixels, 40, 300), GRAY, "no modo organizar");
    // Sem álbum, e com a mesma escolha gravada: a tela inicial segue o tema.
    let (first, _other) = session(NOTEBOOK, dir.path(), None);
    settle(&first);
    let pixels = first.draw();
    assert_eq!(pixel(&first, &pixels, 3, 300), LIGHT, "na tela inicial");
}

#[test]
fn aj_b37_no_meio_com_o_veu_escuro() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    let (screen, app) = session(NOTEBOOK, dir.path(), Some(&album));
    settle(&screen);
    let before = screen.draw();
    run(&app, Command::Preferences);
    settle(&screen);
    let pixels = screen.draw();
    // O card fica no meio: 560 px nos 1366 da janela. Por fora, o véu escuro, igual ao dos outros
    // diálogos: o mesmo ponto fica mais escuro que antes de a tela abrir.
    let row = 354;
    let panel = pixel(&screen, &pixels, 403 + 6, row);
    assert_eq!(panel, pixel(&screen, &pixels, 963 - 6, row));
    let sum = |c: (u8, u8, u8)| u32::from(c.0) + u32::from(c.1) + u32::from(c.2);
    let out = pixel(&screen, &pixels, 403 - 6, row);
    let was = pixel(&screen, &before, 403 - 6, row);
    assert!(
        sum(out) < sum(was),
        "o véu escurece: {out:?} depois, {was:?} antes"
    );
}

#[test]
fn aj_b38_a_demonstracao_vale_mesmo_sem_movimento() {
    let dir = tempfile::tempdir().unwrap();
    let (screen, app) = session(NOTEBOOK, dir.path(), None);
    run(&app, Command::Preferences);
    let ui = screen.app.global::<PrefsUi>();
    ui.invoke_motion_picked(2);
    assert!(ui.get_reduced());
    assert_eq!(screen.app.global::<crate::Motion>().get_navigate_ms(), 0);
    assert!(ui.get_demo_ms() > 0);
}

#[test]
fn aj_b39_avisa_que_a_barra_de_menus_muda_ao_abrir_de_novo() {
    assert_eq!(theme_note(ThemeChoice::Dark, None, false), "");
    assert_eq!(
        theme_note(ThemeChoice::System, Some(false), false),
        "Agora: claro"
    );
    assert!(
        theme_note(ThemeChoice::Light, None, true)
            .starts_with("A barra de menus (Arquivo, Editar…)")
    );
    assert!(cfg!(not(windows)) || opened_dark(ThemeChoice::Light) == Some(false));
    assert_eq!(opened_dark(ThemeChoice::System), None);
}

#[test]
fn aj_b21_o_tamanho_muda_so_o_texto() {
    let dir = tempfile::tempdir().unwrap();
    let (screen, _app) = session(NOTEBOOK, dir.path(), None);
    let ui = screen.app.global::<PrefsUi>();
    ui.invoke_size_picked(4);
    let scale = screen.app.global::<Theme>().get_text_scale();
    assert!((scale - 1.3).abs() < 0.001, "{scale}");
    let body = screen.app.global::<Type>().get_body();
    assert!((body - 16.9).abs() < 0.01, "{body}");
}

#[test]
fn aj_b32_a_janela_nasce_como_foi_escolhida() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("janela.json"),
        r#"{"versao": 3, "tema": "claro", "fundo_lamina": "cinza", "tamanho_texto": 130}"#,
    )
    .unwrap();
    let album = example(dir.path());
    let (screen, _app) = session(NOTEBOOK, dir.path(), Some(&album));
    assert!(!screen.app.global::<Theme>().get_dark());
    assert!((screen.app.global::<Theme>().get_text_scale() - 1.3).abs() < 0.001);
    let pixels = screen.draw();
    assert_eq!(pixel(&screen, &pixels, 20, 110), (0x77, 0x77, 0x77));
}

#[test]
fn aj_b33_o_logotipo_acompanha_o_tema() {
    let dir = tempfile::tempdir().unwrap();
    let (screen, _app) = session(NOTEBOOK, dir.path(), None);
    screen.app.global::<PrefsUi>().invoke_theme_picked(1);
    let pixels = screen.draw();
    let mut dark = 0;
    for y in 44..80 {
        for x in 8..120 {
            let (r, g, b) = pixel(&screen, &pixels, x, y);
            if u32::from(r) + u32::from(g) + u32::from(b) < 300 {
                dark += 1;
            }
        }
    }
    assert!(
        dark > 100,
        "as letras não apareceram: {dark} pixels escuros"
    );
}

#[test]
fn aj_b34_nada_de_azul_no_zoom() {
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    for theme in 0..2 {
        let (screen, _app) = session(NOTEBOOK, dir.path(), Some(&album));
        screen.app.global::<PrefsUi>().invoke_theme_picked(theme);
        let pixels = screen.draw();
        // O azul que o `Slider` padrão usa quando o sistema não diz a cor de destaque.
        let blue = pixels
            .iter()
            .filter(|p| {
                (p.r, p.g, p.b) == (0x00, 0x5f, 0xb8) || (p.r, p.g, p.b) == (0x60, 0xcd, 0xff)
            })
            .count();
        assert_eq!(blue, 0, "tema {theme}");
    }
}

/// As imagens da janela para o dono conferir (`LAMINA_CAPTURAS`).
#[test]
fn capturas_das_telas_principais() {
    if std::env::var("LAMINA_CAPTURAS").is_err() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let album = example(dir.path());
    for (tag, size, theme, backdrop, text) in [
        ("escuro-100", NOTEBOOK, 0, 0, 1),
        ("claro-100", NOTEBOOK, 1, 0, 1),
        ("claro-cinza", NOTEBOOK, 1, 2, 1),
        ("escuro-cinza", NOTEBOOK, 0, 2, 1),
        ("escuro-claro", NOTEBOOK, 0, 3, 1),
        ("claro-escuro", NOTEBOOK, 1, 1, 1),
        ("claro-130", NOTEBOOK, 1, 0, 4),
        ("escuro-130-800", SMALL, 0, 0, 4),
    ] {
        for (name, with_album) in [("janela", true), ("inicial", false)] {
            let opened = with_album.then_some(album.as_path());
            let (screen, app) = session(size, dir.path(), opened);
            choose(&screen, theme, backdrop, text);
            settle(&screen);
            save(&screen, &format!("{tag}-{name}"));
            if with_album {
                run(&app, Command::Preferences);
                settle(&screen);
                save(&screen, &format!("{tag}-preferencias"));
                let ui = screen.app.global::<PrefsUi>();
                ui.set_show(false);
                run(&app, Command::Organize);
                settle(&screen);
                save(&screen, &format!("{tag}-organizar"));
            }
        }
    }
}
