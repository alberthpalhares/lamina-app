//! O roteiro (`docs/novidades/roteiros/<nome>.json`): o que a filmagem faz, passo a passo.

use serde_json::Value;

use super::FilmError;

/// Os nomes de tecla que um roteiro aceita, além de um caractere só.
pub const KEYS: [&str; 15] = [
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Enter",
    "Escape",
    "Tab",
    "Space",
    "PageUp",
    "PageDown",
    "Home",
    "End",
    "Delete",
    "Backspace",
    "F1",
];

/// Quanto a câmera leva para chegar, quando o roteiro não diz.
const CAMERA_MS: u64 = 700;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
}

/// O álbum aberto antes do primeiro passo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opening {
    /// O álbum de exemplo que vem com o programa.
    Example,
    /// Um álbum novo, sem fotos.
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    WaitPhotos,
    Pause { ms: u64 },
    Camera { target: String, zoom: f64, ms: u64 },
    Move { mark: String, ms: u64 },
    Click { button: Button },
    Key { key: String },
    Drag { from: String, to: String, ms: u64 },
    Command { name: String },
    Caption { text: String, ms: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub name: String,
    pub opening: Opening,
    pub window: (u32, u32),
    pub scale: u32,
    pub steps: Vec<Step>,
}

/// O erro do roteiro inteiro, sem passo.
fn whole(message: impl Into<String>) -> FilmError {
    FilmError::Script {
        step: None,
        message: message.into(),
    }
}

/// Os campos de um passo, com o número dele para o erro.
struct Fields<'a> {
    number: usize,
    value: &'a Value,
}

impl Fields<'_> {
    fn error(&self, message: String) -> FilmError {
        FilmError::Script {
            step: Some(self.number),
            message,
        }
    }

    fn text(&self, name: &str) -> Result<String, FilmError> {
        self.value[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| self.error(format!("falta \"{name}\", um texto")))
    }

    fn ms(&self, name: &str) -> Result<u64, FilmError> {
        self.value[name]
            .as_u64()
            .ok_or_else(|| self.error(format!("falta \"{name}\", em ms")))
    }

    fn click(&self) -> Result<Step, FilmError> {
        let button = match &self.value["clicar"] {
            Value::Bool(true) => Button::Left,
            Value::String(side) if side == "direito" => Button::Right,
            other => return Err(self.error(format!("\"clicar\" não aceita {other}"))),
        };
        Ok(Step::Click { button })
    }

    fn key(&self) -> Result<Step, FilmError> {
        let key = self.text("tecla")?;
        if KEYS.contains(&key.as_str()) || key.chars().count() == 1 {
            return Ok(Step::Key { key });
        }
        Err(self.error(format!("a tecla \"{key}\" não existe")))
    }

    fn camera(&self) -> Result<Step, FilmError> {
        Ok(Step::Camera {
            target: self.text("camera")?,
            zoom: self.value["zoom"].as_f64().unwrap_or(1.0),
            ms: self.value["ms"].as_u64().unwrap_or(CAMERA_MS),
        })
    }

    fn step(&self) -> Result<Step, FilmError> {
        let Some(object) = self.value.as_object() else {
            return Err(self.error(format!("{} não é um passo", self.value)));
        };
        let has = |name: &str| object.contains_key(name);
        if has("esperar_fotos") {
            Ok(Step::WaitPhotos)
        } else if has("pausa_ms") {
            Ok(Step::Pause {
                ms: self.ms("pausa_ms")?,
            })
        } else if has("camera") {
            self.camera()
        } else if has("mover") {
            Ok(Step::Move {
                mark: self.text("mover")?,
                ms: self.ms("ms")?,
            })
        } else if has("clicar") {
            self.click()
        } else if has("tecla") {
            self.key()
        } else if has("arrastar") {
            Ok(Step::Drag {
                from: self.text("arrastar")?,
                to: self.text("ate")?,
                ms: self.ms("ms")?,
            })
        } else if has("comando") {
            Ok(Step::Command {
                name: self.text("comando")?,
            })
        } else if has("legenda") {
            Ok(Step::Caption {
                text: self.text("legenda")?,
                ms: self.ms("ms")?,
            })
        } else {
            let names: Vec<&str> = object.keys().map(String::as_str).collect();
            Err(self.error(format!("\"{}\" não existe", names.join(", "))))
        }
    }
}

fn opening(value: &Value) -> Result<Opening, FilmError> {
    match value.as_str() {
        Some("exemplo") => Ok(Opening::Example),
        Some("vazio") => Ok(Opening::Empty),
        _ => Err(whole(format!(
            "\"abrir\" é \"exemplo\" ou \"vazio\", não {value}"
        ))),
    }
}

fn size(value: &Value) -> Option<(u32, u32)> {
    let width = u32::try_from(value.get(0)?.as_u64()?).ok()?;
    let height = u32::try_from(value.get(1)?.as_u64()?).ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

impl Script {
    /// Lê o texto do roteiro.
    pub fn parse(text: &str) -> Result<Script, FilmError> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| whole(format!("linha {}: {e}", e.line())))?;
        let steps = value["passos"]
            .as_array()
            .ok_or_else(|| whole("falta a lista \"passos\""))?
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let number = i + 1;
                Fields { number, value }.step()
            })
            .collect::<Result<Vec<Step>, FilmError>>()?;
        Ok(Script {
            name: value["nome"]
                .as_str()
                .ok_or_else(|| whole("falta \"nome\""))?
                .to_owned(),
            opening: opening(&value["abrir"])?,
            window: size(&value["janela"])
                .ok_or_else(|| whole("\"janela\" é [largura, altura]"))?,
            scale: value["escala"]
                .as_u64()
                .and_then(|s| u32::try_from(s).ok())
                .filter(|s| *s > 0)
                .ok_or_else(|| whole("falta \"escala\""))?,
            steps,
        })
    }
}
