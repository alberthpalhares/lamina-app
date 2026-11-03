//! A filmagem da interface (spec FI): um roteiro em arquivo vira os quadros de um vídeo com a janela
//! de verdade. Aqui ficam o roteiro, a linha do tempo, o que roda na abertura e a escrita dos quadros;
//! o `lamina-app` só traduz para o Slint (regra 1).

mod files;
mod log;
mod marks;
mod script;
mod startup;
mod steps;
mod timeline;

pub use files::{empty_album, isolated, write_frame};
pub use marks::{MarkBox, Scene, is_known_mark};
pub use script::{Button, KEYS, Opening, Script, Step};
pub use startup::{Mode, Startup};
pub use timeline::{Action, Cue, FPS, Frame, Timeline, ease_in_out, frame_time};

/// O que faz a filmagem parar. Nunca chega à janela: só a linha de comando vê (`xtask erros`).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FilmError {
    #[error("{}roteiro inválido: {message}", step.map(|n| format!("passo {n}: ")).unwrap_or_default())]
    Script {
        step: Option<usize>,
        message: String,
    },
    #[error("passo {step}: a marca \"{name}\" não existe")]
    UnknownMark { step: usize, name: String },
    #[error("passo {step}: a marca \"{name}\" não está na tela")]
    MissingMark { step: usize, name: String },
    #[error("as fotos não chegaram em {seconds} s")]
    Timeout { seconds: u64 },
    #[error("não consegui gravar {path}: {message}")]
    Write { path: String, message: String },
    #[error("passo {step}: o comando \"{command}\" não está disponível")]
    Unavailable { step: usize, command: String },
    #[error("a pasta do usuário não está isolada; variáveis: {}", variables.join(", "))]
    NotIsolated { variables: Vec<String> },
    #[error("a janela não abriu: {0}")]
    Window(String),
}
