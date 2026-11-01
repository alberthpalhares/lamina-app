//! Conteúdo das lâminas: quadros e cores. O recorte da foto está em `crop.rs`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Crop, FrameId, Opacity, PhotoId, SpreadId, TextBox};
use crate::{Mm, PageSide, RelRect};

/// Cor RGB de 8 bits (sRGB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
}

/// O que um quadro mostra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrameContent {
    /// Quadro vazio, esperando uma foto.
    Empty,
    Photo {
        photo: PhotoId,
        #[serde(default)]
        crop: Crop,
    },
    /// Quadro de texto (spec TX).
    Text(TextBox),
}

/// Borda desenhada para dentro do quadro (spec AP, regra 4). Largura 0 = sem borda.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Border {
    pub width: Mm,
    pub color: Rgb,
}

impl Default for Border {
    fn default() -> Self {
        Border {
            width: Mm(0.0),
            color: Rgb::WHITE,
        }
    }
}

/// Um quadro na lâmina. A posição é relativa à lâmina no corte final ([`crate::AlbumSpec::trim_rect`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Frame {
    pub id: FrameId,
    pub rect: RelRect,
    pub content: FrameContent,
    /// Borda da foto (spec AP).
    #[serde(default)]
    pub border: Border,
    /// Raio dos cantos arredondados; 0 = cantos retos (spec AP, regra 5).
    #[serde(default)]
    pub radius: Mm,
    /// Opacidade da foto (spec OP); só vai para o arquivo quando não é 100.
    #[serde(
        rename = "opacidade",
        default,
        skip_serializing_if = "Opacity::is_full"
    )]
    pub opacity: Opacity,
}

/// Uma lâmina: duas páginas abertas lado a lado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Spread {
    pub id: SpreadId,
    /// Fundo da lâmina (da página esquerda, quando a direita tem o seu).
    pub background: Rgb,
    /// Fundo próprio da página direita (spec AP, regra 3); `None` = o mesmo da esquerda.
    #[serde(default)]
    pub background_right: Option<Rgb>,
    /// Espaço entre fotos desta lâmina; `None` = o do álbum (spec AP, regra 1).
    #[serde(default)]
    pub gap: Option<Mm>,
    pub frames: Vec<Frame>,
    /// Quadros de foto mexidos à mão (spec TX, regra 8): o espaço entre fotos e a troca da
    /// especificação não refazem o desenho desta lâmina.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub manual: bool,
    /// A chave do modelo do fotógrafo em uso (spec MD, regra 8): o espaço entre fotos e a troca
    /// da especificação mantêm os quadros dele.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// As páginas em branco de propósito (spec PB, regra 1): impressas, mas fora da diagramação.
    #[serde(
        rename = "em_branco",
        default,
        skip_serializing_if = "BlankPages::is_none"
    )]
    pub blank: BlankPages,
}

/// Quais páginas de uma lâmina estão em branco de propósito (spec PB, regra 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct BlankPages {
    #[serde(
        rename = "esquerda",
        default,
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub left: bool,
    #[serde(
        rename = "direita",
        default,
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub right: bool,
}

impl BlankPages {
    /// Nenhuma página em branco: o campo nem vai para o arquivo.
    pub fn is_none(&self) -> bool {
        !self.left && !self.right
    }

    /// A página `side` está em branco?
    pub fn has(self, side: PageSide) -> bool {
        match side {
            PageSide::Left => self.left,
            PageSide::Right => self.right,
        }
    }

    /// Marca ou desmarca a página `side`.
    pub fn set(&mut self, side: PageSide, blank: bool) {
        match side {
            PageSide::Left => self.left = blank,
            PageSide::Right => self.right = blank,
        }
    }
}

impl Spread {
    /// O fundo de uma página.
    pub fn page_background(&self, side: PageSide) -> Rgb {
        match side {
            PageSide::Left => self.background,
            PageSide::Right => self.background_right.unwrap_or(self.background),
        }
    }
}
