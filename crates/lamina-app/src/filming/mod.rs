//! A filmagem da interface (spec FI): `--filmar <roteiro.json> <pasta>`. Só existe com a feature
//! `filmagem` e nos testes; nunca entra no instalador (FI-B11).

mod film;
mod platform;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::time::Duration;

use lamina_editor::filming::{FilmError, Mode, Script, Startup, isolated};
use lamina_editor::{Cache, example_target};
use slint::Timer;

use self::film::Film;
use self::platform::Stage;

/// O pedido da linha de comando.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub script: PathBuf,
    pub output: PathBuf,
}

/// `--filmar <roteiro.json> <pasta>`, se é essa a chamada.
pub fn requested() -> Option<Request> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|arg| arg == "--filmar")?;
    Some(Request {
        script: PathBuf::from(args.get(at + 1)?),
        output: PathBuf::from(args.get(at + 2)?),
    })
}

fn read_script(path: &Path) -> Result<Script, FilmError> {
    let text = std::fs::read_to_string(path).map_err(|reason| FilmError::Script {
        step: None,
        message: format!("{}: {reason}", path.display()),
    })?;
    Script::parse(&text)
}

/// Recusa filmar com as pastas do fotógrafo: a filmagem abre e grava como o programa de verdade.
fn check_isolated(output: &Path) -> Result<(), FilmError> {
    let vars: Vec<(String, String)> = std::env::vars().collect();
    let absolute = std::path::absolute(output).unwrap_or_else(|_| output.to_path_buf());
    isolated(&vars, &absolute.to_string_lossy())
        .map_err(|variables| FilmError::NotIsolated { variables })
}

/// Filma o roteiro: a janela é a de verdade, sem janela do sistema.
pub fn run(request: &Request) -> Result<(), FilmError> {
    let script = read_script(&request.script)?;
    check_isolated(&request.output)?;
    let (clock, queue) = platform::install()?;
    let failed = |reason: slint::PlatformError| FilmError::Window(reason.to_string());
    let screen =
        crate::headless::show(script.window.0, script.window.1, script.scale).map_err(failed)?;
    let (app, rx, tick) =
        crate::open_app(&screen.app, Cache::system(), Startup::for_mode(Mode::Film));
    crate::mailbox::install(rx, &app);
    Timer::single_shot(Duration::ZERO, crate::mailbox::drain);
    let stage = Stage {
        screen,
        clock,
        queue,
    };
    let film = Film::new(stage, app, tick);
    film.open_album(script.opening, &example_target())?;
    film.shoot(&script, &request.output)
}
