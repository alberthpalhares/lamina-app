//! Cada passo do roteiro vira trechos da linha do tempo; câmera e legenda só entram no registro.

use super::FilmError;
use super::log::{CameraLog, CaptionLog};
use super::marks::{Scene, is_known_mark};
use super::script::{Button, Step};
use super::timeline::{Action, FPS, Timeline};

/// O alvo da câmera que mostra a janela inteira.
const OVERVIEW: &str = "geral";
/// Quanto o botão fica apertado, em quadros, para o clique aparecer no vídeo.
const HOLD_FRAMES: u64 = 2;
/// A ida até a origem de um arrasto.
const REACH_MS: u64 = 300;

fn frames_in(ms: u64) -> u64 {
    ms * FPS / 1000
}

/// Um pedaço de um passo.
#[derive(Debug)]
pub(super) enum Piece {
    Hold(u64),
    Travel { to: (f64, f64), frames: u64 },
    Once(Action),
}

impl Timeline {
    /// Confere todo nome do roteiro contra as marcas conhecidas, antes do primeiro quadro.
    pub fn check_marks(&self, known: &[String]) -> Result<(), FilmError> {
        for (i, step) in self.steps.iter().enumerate() {
            let names: Vec<&str> = match step {
                Step::Move { mark, .. } => vec![mark],
                Step::Drag { from, to, .. } => vec![from, to],
                Step::Camera { target, .. } if target != OVERVIEW => vec![target],
                _ => Vec::new(),
            };
            if let Some(name) = names.iter().find(|n| !is_known_mark(n, known)) {
                return Err(FilmError::UnknownMark {
                    step: i + 1,
                    name: (*name).to_owned(),
                });
            }
        }
        Ok(())
    }

    /// O centro da marca na hora do passo, e guarda o retângulo no registro.
    fn place(&mut self, scene: &mut dyn Scene, name: &str) -> Result<(f64, f64), FilmError> {
        let step = self.step;
        let found = scene.mark(name).filter(|m| m.w > 0.0 && m.h > 0.0);
        let Some(mark) = found else {
            return Err(FilmError::MissingMark {
                step,
                name: name.to_owned(),
            });
        };
        self.log.mark(step, self.frame, name, mark);
        Ok(mark.center())
    }

    fn click(&mut self, button: Button) {
        self.pieces.push_back(Piece::Once(Action::Press(button)));
        self.pieces.push_back(Piece::Hold(HOLD_FRAMES - 1));
        self.pieces.push_back(Piece::Once(Action::Release(button)));
    }

    fn camera(
        &mut self,
        scene: &mut dyn Scene,
        target: &str,
        zoom: f64,
        ms: u64,
    ) -> Result<(), FilmError> {
        let (x, y) = if target == OVERVIEW {
            (
                f64::from(self.log.janela[0]) / 2.0,
                f64::from(self.log.janela[1]) / 2.0,
            )
        } else {
            self.place(scene, target)?
        };
        self.log.cameras.push(CameraLog {
            quadro: self.frame,
            ms,
            x,
            y,
            zoom,
        });
        Ok(())
    }

    fn drag(
        &mut self,
        scene: &mut dyn Scene,
        from: &str,
        to: &str,
        ms: u64,
    ) -> Result<(), FilmError> {
        let start = self.place(scene, from)?;
        let end = self.place(scene, to)?;
        self.pieces.push_back(Piece::Travel {
            to: start,
            frames: frames_in(REACH_MS),
        });
        self.pieces
            .push_back(Piece::Once(Action::Press(Button::Left)));
        self.pieces.push_back(Piece::Hold(HOLD_FRAMES));
        self.pieces.push_back(Piece::Travel {
            to: end,
            frames: frames_in(ms),
        });
        self.pieces.push_back(Piece::Hold(HOLD_FRAMES));
        self.pieces
            .push_back(Piece::Once(Action::Release(Button::Left)));
        Ok(())
    }

    /// Põe o passo na fila de trechos. Devolve `true` quando é a espera pelas fotos.
    pub(super) fn begin(&mut self, step: Step, scene: &mut dyn Scene) -> Result<bool, FilmError> {
        match step {
            Step::WaitPhotos => return Ok(true),
            Step::Pause { ms } => self.pieces.push_back(Piece::Hold(frames_in(ms))),
            Step::Camera { target, zoom, ms } => self.camera(scene, &target, zoom, ms)?,
            Step::Move { mark, ms } => {
                let to = self.place(scene, &mark)?;
                self.pieces.push_back(Piece::Travel {
                    to,
                    frames: frames_in(ms),
                });
            }
            Step::Click { button } => self.click(button),
            Step::Key { key } => self.pieces.push_back(Piece::Once(Action::Key(key))),
            Step::Drag { from, to, ms } => self.drag(scene, &from, &to, ms)?,
            Step::Command { name } => {
                if !scene.available(&name) {
                    return Err(FilmError::Unavailable {
                        step: self.step,
                        command: name,
                    });
                }
                self.pieces.push_back(Piece::Once(Action::Command(name)));
            }
            Step::Caption { text, ms } => self.log.legendas.push(CaptionLog {
                de: self.frame,
                ate: self.frame + frames_in(ms),
                texto: text,
            }),
        }
        Ok(false)
    }
}
