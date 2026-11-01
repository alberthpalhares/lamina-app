//! As animações da página "O que há de novo" (spec NV, regra 12): um PNG animado lido um quadro de
//! cada vez, em laço. Com "Reduzir movimento", ou num arquivo parado, fica o primeiro quadro.

use std::io::Cursor;

use image::codecs::png::PngDecoder;
use image::{AnimationDecoder, Frames};

use super::{Illustration, illustration};

/// O menor tempo de um quadro: um arquivo com tempo zero não vira um laço sem pausa.
const MIN_DELAY_MS: u32 = 20;

/// Um quadro da animação e quanto tempo ele fica na tela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub picture: Illustration,
    pub delay_ms: u32,
}

/// Uma ilustração aberta para tocar. Só o quadro da vez fica na memória.
pub struct Animation<'a> {
    bytes: &'a [u8],
    first: Frame,
    /// Os quadros seguintes; `None` numa ilustração que não anima.
    frames: Option<Frames<'a>>,
    /// O segundo quadro, lido no `open` para saber se o arquivo anima.
    pending: Option<Frame>,
}

impl<'a> Animation<'a> {
    /// Abre a ilustração (PNG animado, PNG ou JPEG). `None` se nem o primeiro quadro lê.
    pub fn open(bytes: &'a [u8], reduce_motion: bool) -> Option<Self> {
        let Some(mut frames) = apng(bytes) else {
            let picture = illustration(bytes)?;
            return Some(Self::still(
                bytes,
                Frame {
                    picture,
                    delay_ms: 0,
                },
            ));
        };
        let first = frames.next().and_then(Result::ok).map(frame)?;
        if reduce_motion {
            return Some(Self::still(bytes, first));
        }
        let Some(second) = frames.next().and_then(Result::ok).map(frame) else {
            return Some(Self::still(bytes, first));
        };
        Some(Self {
            bytes,
            first,
            frames: Some(frames),
            pending: Some(second),
        })
    }

    fn still(bytes: &'a [u8], first: Frame) -> Self {
        Self {
            bytes,
            first,
            frames: None,
            pending: None,
        }
    }

    /// O primeiro quadro: o que aparece ao escolher a novidade e o que fica com "Reduzir movimento".
    pub fn first(&self) -> &Frame {
        &self.first
    }

    /// `true` se há quadros depois do primeiro.
    pub fn animated(&self) -> bool {
        self.frames.is_some()
    }

    /// O quadro seguinte; depois do último (ou de um quadro estragado), o primeiro de novo.
    /// `None` numa ilustração que não anima.
    pub fn advance(&mut self) -> Option<Frame> {
        if let Some(next) = self.pending.take() {
            return Some(next);
        }
        let frames = self.frames.as_mut()?;
        if let Some(next) = frames.next().and_then(Result::ok) {
            return Some(frame(next));
        }
        // Volta ao começo: o primeiro já está guardado, e a leitura recomeça logo depois dele.
        let mut again = apng(self.bytes)?;
        again.next();
        self.frames = Some(again);
        Some(self.first.clone())
    }
}

impl std::fmt::Debug for Animation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animation")
            .field("first_delay_ms", &self.first.delay_ms)
            .field("animated", &self.animated())
            .finish()
    }
}

/// `true` se o arquivo é um PNG animado. Só lê o cabeçalho: a página pergunta de cada linha.
pub fn animates(bytes: &[u8]) -> bool {
    apng(bytes).is_some()
}

/// Os quadros de um PNG animado; `None` se o arquivo não é um.
fn apng(bytes: &[u8]) -> Option<Frames<'_>> {
    let decoder = PngDecoder::new(Cursor::new(bytes)).ok()?;
    if !decoder.is_apng().ok()? {
        return None;
    }
    Some(decoder.apng().ok()?.into_frames())
}

fn frame(f: image::Frame) -> Frame {
    let (numer, denom) = f.delay().numer_denom_ms();
    let delay_ms = (numer / denom.max(1)).max(MIN_DELAY_MS);
    let picture = f.into_buffer();
    Frame {
        picture: Illustration {
            width: picture.width(),
            height: picture.height(),
            rgba: picture.into_raw(),
        },
        delay_ms,
    }
}
