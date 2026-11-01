//! Cenários da spec FI com a janela de verdade, sem tela. O Slint só aceita uma plataforma com fila
//! de tarefas por processo: todos rodam numa linha só, uma coisa de cada vez.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use lamina_editor::filming::{FilmError, KEYS, Scene, Script};
use lamina_editor::{Cache, Editor};
use slint::{ComponentHandle, Rgb8Pixel, Timer};

use super::film::{Film, hold_until, key_text};
use super::platform::{self, Stage};
use crate::Theme;
use crate::headless;
use crate::state::App;

impl Film {
    /// A janela de um teste, sem nada da pasta do usuário; os arquivos ficam em `folder`.
    pub fn for_test(script: &Script, folder: &Path) -> Film {
        let (clock, queue) = platform::install().unwrap();
        let screen = crate::headless::show(script.window.0, script.window.1, script.scale).unwrap();
        let (tx, rx) = crate::mailbox::Mailbox::new();
        let cache = Cache::new(folder.join("cache"));
        let app: crate::Shared = Rc::new(RefCell::new(App::new(
            &screen.app,
            cache,
            Editor::new(),
            tx,
        )));
        let (timing, tick) = crate::timings::Clock::install(screen.app.window());
        app.borrow_mut().clock = timing;
        crate::connect(&screen.app, &app);
        app.borrow_mut().refresh_all();
        crate::mailbox::install(rx, &app);
        let stage = Stage {
            screen,
            clock,
            queue,
        };
        let film = Film::new(stage, app, tick);
        let target = folder.join("album");
        std::fs::create_dir_all(&target).unwrap();
        film.open_album(script.opening, &target).unwrap();
        film
    }

    /// Dois desenhos: as janelas e as listas que nascem no primeiro se medem no segundo.
    pub fn settle_layout(&mut self) {
        self.stage.frame(|| {});
        self.stage.frame(|| {});
    }

    pub fn open_export(&mut self) {
        self.window().invoke_command("arquivo.exportar".into());
    }
}

type Job = Box<dyn FnOnce() + Send>;

/// Roda `job` na linha da filmagem e espera; o `panic` volta para quem chamou.
fn on_film_thread(job: impl FnOnce() + Send + 'static) {
    static THREAD: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    let sender = THREAD.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || rx.iter().for_each(|job| job()))
            .unwrap();
        Mutex::new(tx)
    });
    let (done, wait) = channel();
    let wrapped: Job = Box::new(move || {
        let result = catch_unwind(AssertUnwindSafe(job));
        let _ = done.send(result);
    });
    sender.lock().unwrap().send(wrapped).unwrap();
    if let Err(panic) = wait.recv().unwrap() {
        resume_unwind(panic);
    }
}

fn with_steps(opening: &str, steps: &str) -> Script {
    let text = format!(
        r#"{{"nome":"t","abrir":"{opening}","janela":[1000,700],"escala":1,"passos":[{steps}]}}"#
    );
    Script::parse(&text).unwrap()
}

fn pngs(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".png"))
        .collect();
    names.sort();
    names
}

#[test]
fn fi_b4_as_marcas_existem_na_janela_de_verdade() {
    on_film_thread(|| {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"nome":"t","abrir":"exemplo","janela":[1600,900],"escala":1,"passos":[]}"#;
        let script = Script::parse(text).unwrap();
        let mut film = Film::for_test(&script, dir.path());
        film.settle_layout();
        let fixed = film.fixed_marks();
        assert!(fixed.iter().any(|name| name == "lamina"));
        for name in fixed
            .iter()
            .map(String::as_str)
            .chain(["quadro-1", "foto-1"])
        {
            let mark = film.mark(name);
            let mark = mark.unwrap_or_else(|| panic!("a marca {name} não está na tela"));
            assert!(mark.w > 0.0 && mark.h > 0.0, "{name}: {mark:?}");
        }
        assert_eq!(film.mark("nao-existe"), None);
    });
}

#[test]
fn fi_b4_a_marca_do_dialogo_aparece_com_o_dialogo_aberto() {
    on_film_thread(|| {
        let dir = tempfile::tempdir().unwrap();
        let script = with_steps("exemplo", "");
        let mut film = Film::for_test(&script, dir.path());
        film.settle_layout();
        assert_eq!(film.mark("exportar-cancelar"), None);
        film.open_export();
        film.settle_layout();
        let mark = film.mark("exportar-cancelar").unwrap();
        assert!(mark.w > 0.0 && mark.h > 0.0, "{mark:?}");
    });
}

#[test]
fn fi_b5_marca_desconhecida_nao_grava_nenhum_quadro() {
    on_film_thread(|| {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("saida");
        let script = with_steps(
            "exemplo",
            r#"{"pausa_ms":100},{"camera":"geral"},{"mover":"nao-existe","ms":100}"#,
        );
        let film = Film::for_test(&script, dir.path());
        assert_eq!(
            film.shoot(&script, &output),
            Err(FilmError::UnknownMark {
                step: 3,
                name: "nao-existe".to_owned()
            })
        );
        assert_eq!(pngs(&output), Vec::<String>::new());
    });
}

#[test]
fn fi_b5_botao_de_dialogo_que_nao_abriu_reprova_e_os_quadros_ficam() {
    on_film_thread(|| {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("saida");
        let script = with_steps(
            "exemplo",
            r#"{"pausa_ms":100},{"mover":"exportar-cancelar","ms":100}"#,
        );
        let film = Film::for_test(&script, dir.path());
        assert_eq!(
            film.shoot(&script, &output),
            Err(FilmError::MissingMark {
                step: 2,
                name: "exportar-cancelar".to_owned()
            })
        );
        assert_eq!(
            pngs(&output),
            vec!["quadro-00001.png", "quadro-00002.png", "quadro-00003.png"]
        );
    });
}

#[test]
fn fi_b5_comando_que_nao_existe_reprova_com_o_passo() {
    on_film_thread(|| {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("saida");
        let script = with_steps("vazio", r#"{"comando":"voar.longe"}"#);
        let film = Film::for_test(&script, dir.path());
        assert_eq!(
            film.shoot(&script, &output),
            Err(FilmError::Unavailable {
                step: 1,
                command: "voar.longe".to_owned()
            })
        );
    });
}

#[test]
fn fi_b6_o_relogio_simulado_anda_os_timers() {
    on_film_thread(|| {
        let (clock, _queue) = platform::install().unwrap();
        slint::platform::update_timers_and_animations();
        let fired = Arc::new(AtomicBool::new(false));
        let flag = fired.clone();
        Timer::single_shot(Duration::from_secs(4), move || {
            flag.store(true, Ordering::SeqCst);
        });
        for _ in 0..119 {
            clock.next_frame();
            slint::platform::update_timers_and_animations();
        }
        assert!(
            !fired.load(Ordering::SeqCst),
            "disparou antes do quadro 120"
        );
        clock.next_frame();
        slint::platform::update_timers_and_animations();
        assert!(fired.load(Ordering::SeqCst), "não disparou no quadro 120");
    });
}

fn corner(pixels: &[Rgb8Pixel]) -> (u8, u8, u8) {
    let p = pixels[pixels.len() / 2];
    (p.r, p.g, p.b)
}

#[test]
fn fi_b7_as_tarefas_de_fundo_chegam_antes_do_desenho() {
    on_film_thread(|| {
        let (clock, queue) = platform::install().unwrap();
        let screen = headless::show(400, 300, 1).unwrap();
        let stage = platform::Stage {
            screen,
            clock,
            queue,
        };
        let film_thread = std::thread::current().id();
        let window = stage.screen.app.as_weak();
        let dark = corner(&stage.frame(|| {}));
        let worker = std::thread::spawn(move || {
            window
                .upgrade_in_event_loop(move |w| {
                    assert_eq!(std::thread::current().id(), film_thread);
                    w.global::<Theme>().set_dark(false);
                })
                .unwrap();
        });
        worker.join().unwrap();
        let light = corner(&stage.frame(|| {}));
        assert_ne!(dark, light, "o desenho veio antes da tarefa");
    });
}

#[test]
fn fi_b8_esperar_as_fotos_segura_o_relogio() {
    on_film_thread(|| {
        let (clock, _queue) = platform::install().unwrap();
        let before = clock.frame();
        let mut turns = 0;
        let released = hold_until(Duration::from_secs(5), || {
            turns += 1;
            turns == 3
        });
        assert_eq!(released, Ok(()));
        assert_eq!(turns, 3);
        assert_eq!(clock.frame(), before, "o relógio andou na espera");
        let stuck = hold_until(Duration::from_millis(50), || false);
        assert_eq!(stuck, Err(FilmError::Timeout { seconds: 0 }));
    });
}

#[test]
fn fi_b10_mesmo_roteiro_mesmos_quadros() {
    on_film_thread(|| {
        let text = include_str!("../../tests/dados/filmagem.json");
        let script = Script::parse(text).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let shoot = |name: &str| {
            let output = dir.path().join(name);
            let film = Film::for_test(&script, dir.path());
            film.shoot(&script, &output).unwrap();
            output
        };
        let first = shoot("a");
        let second = shoot("b");
        let names = pngs(&first);
        assert!(names.len() > 20, "{} quadros", names.len());
        assert_eq!(names, pngs(&second));
        for name in names {
            let a = std::fs::read(first.join(&name)).unwrap();
            let b = std::fs::read(second.join(&name)).unwrap();
            assert!(a == b, "{name} mudou");
        }
        let marks = |folder: &Path| std::fs::read_to_string(folder.join("marcas.json")).unwrap();
        assert_eq!(marks(&first), marks(&second));
    });
}

#[test]
fn fi_b11_o_instalador_nao_leva_a_filmagem() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("filmagem = []"), "a feature sumiu");
    for line in manifest.lines().filter(|l| l.starts_with("default")) {
        assert!(!line.contains("filmagem"), "{line}");
    }
    let release =
        std::fs::read_to_string(root.join("../../.github/workflows/release.yml")).unwrap();
    assert!(release.contains("--profile dist -p lamina-app"));
    assert!(!release.contains("filmagem"), "a release liga a filmagem");
    assert!(!release.contains("--all-features"), "a release liga tudo");
    let main = include_str!("../main.rs").replace("\r\n", "\n");
    assert!(main.contains("#[cfg(any(test, feature = \"filmagem\"))]\n#[cfg_attr(not(feature = \"filmagem\"), allow(dead_code))]\nmod filming;"));
}

#[test]
fn fi_b1_as_teclas_do_roteiro_viram_teclas_do_slint() {
    for key in KEYS {
        assert_eq!(key_text(key).chars().count(), 1, "{key}");
        assert_ne!(key_text(key), key, "{key}");
    }
    assert_eq!(key_text("Space"), " ");
    assert_eq!(key_text("t"), "t");
}
