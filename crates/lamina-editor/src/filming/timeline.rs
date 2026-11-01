//! A linha do tempo: o que acontece em cada quadro, sem hora do computador (spec FI, regra 2). Cada
//! passo vira uma fila de trechos, e cada trecho, quadros; câmera e legenda só entram no registro,
//! sem gastar quadro, para acontecerem junto do passo seguinte.

use std::collections::VecDeque;
use std::time::Duration;

use super::FilmError;
use super::log::{FrameLog, Log};
use super::marks::Scene;
use super::script::{Button, Script, Step};
use super::steps::Piece;

pub const FPS: u64 = 30;

/// O tempo do quadro, de uma divisão só, sem somar arredondamentos.
pub fn frame_time(frame: u64) -> Duration {
    let nanos = u128::from(frame) * 1_000_000_000 / u128::from(FPS);
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

/// A curva do cursor: acelera e freia.
pub fn ease_in_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Press(Button),
    Release(Button),
    Key(String),
    Command(String),
}

/// Um quadro: o tempo, onde o cursor está e o que acontece antes de desenhar.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub number: u64,
    pub time: Duration,
    pub cursor: (f64, f64),
    pub actions: Vec<Action>,
}

/// O que vem a seguir.
#[derive(Debug, Clone, PartialEq)]
pub enum Cue {
    Frame(Frame),
    /// Segurar o relógio até as fotos chegarem.
    WaitPhotos,
}

#[derive(Debug)]
struct Travel {
    from: (f64, f64),
    to: (f64, f64),
    done: u64,
    frames: u64,
}

#[derive(Debug)]
pub struct Timeline {
    pub(super) steps: VecDeque<Step>,
    pub(super) step: usize,
    pub(super) pieces: VecDeque<Piece>,
    travel: Option<Travel>,
    pub(super) frame: u64,
    cursor: (f64, f64),
    pressed: u8,
    /// Onde o último quadro mostrou o cursor: o fim do roteiro precisa mostrar a chegada.
    shown: (f64, f64),
    wait_limit: Duration,
    pub(super) log: Log,
}

impl Timeline {
    pub fn new(script: &Script) -> Timeline {
        let center = (
            f64::from(script.window.0) / 2.0,
            f64::from(script.window.1) / 2.0,
        );
        Timeline {
            steps: script.steps.iter().cloned().collect(),
            step: 0,
            pieces: VecDeque::new(),
            travel: None,
            frame: 0,
            cursor: center,
            pressed: 0,
            shown: center,
            wait_limit: Duration::from_secs(60),
            log: Log::new(script),
        }
    }

    #[must_use]
    pub fn with_wait_limit(mut self, limit: Duration) -> Timeline {
        self.wait_limit = limit;
        self
    }

    pub fn wait_limit(&self) -> Duration {
        self.wait_limit
    }

    /// Escreve um quadro, com o cursor de agora.
    fn emit(&mut self, actions: Vec<Action>) -> Cue {
        for action in &actions {
            match action {
                Action::Press(Button::Left) => self.pressed = 1,
                Action::Press(Button::Right) => self.pressed = 2,
                Action::Release(_) => self.pressed = 0,
                Action::Key(_) | Action::Command(_) => {}
            }
        }
        let teclas = actions
            .iter()
            .filter_map(|a| match a {
                Action::Key(key) => Some(key.clone()),
                _ => None,
            })
            .collect();
        let comandos = actions
            .iter()
            .filter_map(|a| match a {
                Action::Command(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        self.log.quadros.push(FrameLog {
            x: self.cursor.0,
            y: self.cursor.1,
            botao: self.pressed,
            teclas,
            comandos,
        });
        let frame = Frame {
            number: self.frame,
            time: frame_time(self.frame),
            cursor: self.cursor,
            actions,
        };
        self.shown = self.cursor;
        self.frame += 1;
        Cue::Frame(frame)
    }

    /// Um quadro do trajeto de agora: o primeiro ainda mostra o cursor de onde ele sai.
    fn travel_frame(&mut self) -> Option<Cue> {
        let travel = self.travel.as_mut()?;
        #[allow(clippy::cast_precision_loss)]
        let e = ease_in_out(travel.done as f64 / travel.frames as f64);
        let at = (
            travel.from.0 + (travel.to.0 - travel.from.0) * e,
            travel.from.1 + (travel.to.1 - travel.from.1) * e,
        );
        travel.done += 1;
        let ended = (travel.done >= travel.frames).then_some(travel.to);
        self.cursor = at;
        let cue = self.emit(Vec::new());
        if let Some(to) = ended {
            self.cursor = to;
            self.travel = None;
        }
        Some(cue)
    }

    /// O próximo quadro ou espera; `None` no fim do roteiro.
    pub fn next(&mut self, scene: &mut dyn Scene) -> Result<Option<Cue>, FilmError> {
        loop {
            if let Some(cue) = self.travel_frame() {
                return Ok(Some(cue));
            }
            match self.pieces.pop_front() {
                Some(Piece::Hold(0)) => {}
                Some(Piece::Hold(left)) => {
                    self.pieces.push_front(Piece::Hold(left - 1));
                    return Ok(Some(self.emit(Vec::new())));
                }
                Some(Piece::Travel { to, frames: 0 }) => self.cursor = to,
                Some(Piece::Travel { to, frames }) => {
                    self.travel = Some(Travel {
                        from: self.cursor,
                        to,
                        done: 0,
                        frames,
                    });
                }
                Some(Piece::Once(action)) => return Ok(Some(self.emit(vec![action]))),
                None => {
                    let Some(step) = self.steps.pop_front() else {
                        if self.shown != self.cursor {
                            return Ok(Some(self.emit(Vec::new())));
                        }
                        return Ok(None);
                    };
                    self.step += 1;
                    if self.begin(step, scene)? {
                        return Ok(Some(Cue::WaitPhotos));
                    }
                }
            }
        }
    }

    /// O `marcas.json`: por quadro, o cursor, o botão e as teclas; por passo, as marcas, a câmera e
    /// as legendas.
    pub fn marks_json(&self) -> String {
        serde_json::to_string(&self.log).unwrap_or_default()
    }
}
