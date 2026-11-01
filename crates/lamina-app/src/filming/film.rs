//! O roteiro rodando na janela (spec FI): a linha do tempo do editor diz o que fazer em cada quadro, e
//! aqui isso vira evento do Slint e pixels no arquivo.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use lamina_editor::Command;
use lamina_editor::filming::{
    Action, Button, Cue, FilmError, Frame, MarkBox, Opening, Scene, Script, Timeline, empty_album,
    write_frame,
};
use lamina_editor::prepare_example;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model, Rgb8Pixel, Timer};

use super::platform::Stage;
use crate::state::App;
use crate::{AppWindow, Marks, Shared, files, welcome};

/// Quantos quadros seguidos a tela precisa ficar igual para o álbum estar pronto.
const STILL_FRAMES: u32 = 15;
/// Intervalo entre as voltas de quem espera, em tempo de verdade.
const WAIT_TURN: Duration = Duration::from_millis(10);

/// A tecla do roteiro como o Slint a recebe.
pub fn key_text(key: &str) -> String {
    let named = match key {
        "ArrowUp" => Key::UpArrow,
        "ArrowDown" => Key::DownArrow,
        "ArrowLeft" => Key::LeftArrow,
        "ArrowRight" => Key::RightArrow,
        "Enter" => Key::Return,
        "Escape" => Key::Escape,
        "Tab" => Key::Tab,
        "Space" => Key::Space,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Home" => Key::Home,
        "End" => Key::End,
        "Delete" => Key::Delete,
        "Backspace" => Key::Backspace,
        "F1" => Key::F1,
        other => return other.to_owned(),
    };
    char::from(named).to_string()
}

/// Segura o relógio: roda `turn` até devolver `true`; passou do limite, desiste (spec FI, regra 4).
pub fn hold_until(limit: Duration, mut turn: impl FnMut() -> bool) -> Result<(), FilmError> {
    let started = Instant::now();
    loop {
        if turn() {
            return Ok(());
        }
        if started.elapsed() > limit {
            return Err(FilmError::Timeout {
                seconds: limit.as_secs(),
            });
        }
        std::thread::sleep(WAIT_TURN);
    }
}

fn pointer(button: Button) -> PointerEventButton {
    match button {
        Button::Left => PointerEventButton::Left,
        Button::Right => PointerEventButton::Right,
    }
}

/// Nada mais vem das linhas de fundo.
fn background_idle(app: &App) -> bool {
    app.images.screens_idle() && app.roll.worker.idle()
}

fn rgb_bytes(pixels: &[Rgb8Pixel]) -> Vec<u8> {
    pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect()
}

/// As respostas das marcas, pelo nome.
type Answers = Rc<RefCell<BTreeMap<String, MarkBox>>>;

pub struct Film {
    pub(super) stage: Stage,
    app: Shared,
    answers: Answers,
    /// Onde o cursor está para a janela: só um movimento de verdade vira evento.
    cursor: Option<(f64, f64)>,
    /// Quem mantém o relógio dos envios do rolo vivo.
    _tick: Option<Timer>,
}

impl Film {
    pub fn new(stage: Stage, app: Shared, tick: Option<Timer>) -> Film {
        let answers = Answers::default();
        let store = answers.clone();
        let marks = stage.screen.app.global::<Marks>();
        marks.on_report(move |name, x, y, w, h| {
            let found = MarkBox { x, y, w, h };
            store.borrow_mut().insert(name.as_str().to_owned(), found);
        });
        Film {
            stage,
            app,
            answers,
            cursor: None,
            _tick: tick,
        }
    }

    /// Abre o álbum do roteiro; os arquivos dele ficam em `folder`.
    pub fn open_album(&self, opening: Opening, folder: &Path) -> Result<(), FilmError> {
        let mut app = self.app.borrow_mut();
        match opening {
            Opening::Example => {
                let source = welcome::example_source();
                let file = prepare_example(&source, folder)
                    .map_err(|reason| FilmError::Window(reason.to_string()))?;
                files::open_file(&mut app, &file);
            }
            Opening::Empty => {
                let target = folder.join("album-vazio.lamina");
                let _ = std::fs::remove_file(&target);
                let project = empty_album().map_err(FilmError::Window)?;
                app.editor
                    .adopt_saved(project, &target)
                    .map_err(|reason| FilmError::Window(reason.to_string()))?;
                app.project_changed();
            }
        }
        Ok(())
    }

    pub(super) fn window(&self) -> &AppWindow {
        &self.stage.screen.app
    }

    pub fn fixed_marks(&self) -> Vec<String> {
        let list = self.window().global::<Marks>().get_fixed();
        list.iter().map(|name| name.as_str().to_owned()).collect()
    }

    fn known_marks(&self) -> Vec<String> {
        let marks = self.window().global::<Marks>();
        let on_demand = marks.get_on_demand();
        let mut known = self.fixed_marks();
        known.extend(on_demand.iter().map(|name| name.as_str().to_owned()));
        known
    }

    /// Roda o roteiro inteiro; os quadros prontos ficam, mesmo quando ele reprova.
    pub fn shoot(mut self, script: &Script, output: &Path) -> Result<(), FilmError> {
        std::fs::create_dir_all(output).map_err(|reason| FilmError::Write {
            path: output.display().to_string(),
            message: reason.to_string(),
        })?;
        let mut timeline = Timeline::new(script);
        timeline.check_marks(&self.known_marks())?;
        // As marcas só valem depois de um desenho (regra 6).
        self.stage.frame(|| {});
        while let Some(cue) = timeline.next(&mut self)? {
            match cue {
                Cue::WaitPhotos => self.wait_photos(timeline.wait_limit())?,
                Cue::Frame(frame) => self.shoot_frame(&frame, output)?,
            }
        }
        let path = output.join("marcas.json");
        std::fs::write(&path, timeline.marks_json()).map_err(|reason| FilmError::Write {
            path: path.display().to_string(),
            message: reason.to_string(),
        })
    }

    fn shoot_frame(&mut self, frame: &Frame, output: &Path) -> Result<(), FilmError> {
        let pixels = self.stage.frame(|| self.apply(frame));
        self.cursor = Some(frame.cursor);
        let screen = &self.stage.screen;
        let name = format!("quadro-{:05}.png", frame.number + 1);
        write_frame(
            &output.join(name),
            screen.width(),
            screen.height(),
            &rgb_bytes(&pixels),
        )
    }

    /// A entrada do quadro: passa pelo `dispatch_event`, como a de verdade (regra 7).
    fn apply(&self, frame: &Frame) {
        let window = self.window().window();
        #[allow(clippy::cast_possible_truncation)]
        let position = LogicalPosition::new(frame.cursor.0 as f32, frame.cursor.1 as f32);
        if self.cursor != Some(frame.cursor) {
            window.dispatch_event(WindowEvent::PointerMoved { position });
        }
        for action in &frame.actions {
            match action {
                Action::Press(button) => window.dispatch_event(WindowEvent::PointerPressed {
                    position,
                    button: pointer(*button),
                }),
                Action::Release(button) => window.dispatch_event(WindowEvent::PointerReleased {
                    position,
                    button: pointer(*button),
                }),
                Action::Key(key) => {
                    let text = key_text(key);
                    window.dispatch_event(WindowEvent::KeyPressed {
                        text: text.clone().into(),
                    });
                    window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
                }
                Action::Command(name) => self.window().invoke_command(name.into()),
            }
        }
    }

    /// As fotos chegando (regra 4): primeiro as miniaturas, com o relógio parado; depois o que precisa
    /// do relógio andando (a foto nítida e as colunas do rolo), em quadros que não vão para o vídeo,
    /// até a tela parar de mudar.
    fn wait_photos(&mut self, limit: Duration) -> Result<(), FilmError> {
        let (stage, app) = (&self.stage, &self.app);
        hold_until(limit, || {
            let ran = stage.queue.run_all();
            slint::platform::update_timers_and_animations();
            if ran > 0 {
                stage.screen.paint();
            }
            let pending = app.try_borrow().map_or(1, |a| a.images.thumbs_pending());
            ran == 0 && pending == 0
        })?;
        let mut last = stage.frame(|| {});
        let mut still = 0;
        hold_until(limit, || {
            let pixels = stage.frame(|| {});
            still = if pixels == last { still + 1 } else { 0 };
            last = pixels;
            let idle = app.try_borrow().is_ok_and(|a| background_idle(&a));
            still >= STILL_FRAMES && idle
        })
    }
}

impl Scene for Film {
    fn mark(&mut self, name: &str) -> Option<MarkBox> {
        self.answers.borrow_mut().clear();
        let marks = self.window().global::<Marks>();
        marks.set_ask(marks.get_ask() + 1);
        slint::platform::update_timers_and_animations();
        let found = self.answers.borrow().get(name).copied()?;
        let size = self.window().window().size();
        let scale = self.window().window().scale_factor();
        #[allow(clippy::cast_precision_loss)]
        let (width, height) = (size.width as f32 / scale, size.height as f32 / scale);
        let (x, y) = (found.x + found.w / 2.0, found.y + found.h / 2.0);
        (x >= 0.0 && y >= 0.0 && x < width && y < height).then_some(found)
    }

    fn available(&self, command: &str) -> bool {
        let found = Command::from_name(command);
        found.is_some_and(|c| self.app.borrow().editor.available(c))
    }
}
