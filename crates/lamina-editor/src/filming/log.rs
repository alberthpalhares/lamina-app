//! O `marcas.json`: o que o Remotion precisa para compor o vídeo (spec FI, seção 4).

use std::collections::BTreeMap;

use serde::Serialize;

use super::marks::MarkBox;
use super::script::Script;
use super::timeline::FPS;

/// Por quadro: o cursor, o botão apertado (0, 1 esquerdo, 2 direito) e as teclas.
#[derive(Debug, Serialize)]
pub(super) struct FrameLog {
    pub x: f64,
    pub y: f64,
    pub botao: u8,
    pub teclas: Vec<String>,
}

/// Por passo: os retângulos das marcas que ele usou.
#[derive(Debug, Serialize)]
pub(super) struct StepLog {
    pub passo: usize,
    pub quadro: u64,
    pub marcas: BTreeMap<String, MarkBox>,
}

#[derive(Debug, Serialize)]
pub(super) struct CameraLog {
    pub quadro: u64,
    pub ms: u64,
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct CaptionLog {
    pub de: u64,
    pub ate: u64,
    pub texto: String,
}

#[derive(Debug, Serialize)]
pub(super) struct Log {
    pub nome: String,
    pub fps: u64,
    pub janela: [u32; 2],
    pub escala: u32,
    pub quadros: Vec<FrameLog>,
    pub passos: Vec<StepLog>,
    pub cameras: Vec<CameraLog>,
    pub legendas: Vec<CaptionLog>,
}

impl Log {
    pub fn new(script: &Script) -> Log {
        Log {
            nome: script.name.clone(),
            fps: FPS,
            janela: [script.window.0, script.window.1],
            escala: script.scale,
            quadros: Vec::new(),
            passos: Vec::new(),
            cameras: Vec::new(),
            legendas: Vec::new(),
        }
    }

    /// O retângulo que o passo `step` usou, a partir do quadro `frame`.
    pub fn mark(&mut self, step: usize, frame: u64, name: &str, mark: MarkBox) {
        if self.passos.last().is_none_or(|p| p.passo != step) {
            self.passos.push(StepLog {
                passo: step,
                quadro: frame,
                marcas: BTreeMap::new(),
            });
        }
        if let Some(entry) = self.passos.last_mut() {
            entry.marcas.insert(name.to_owned(), mark);
        }
    }
}
