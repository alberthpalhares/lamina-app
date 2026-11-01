//! O quadro de texto (spec TX): o texto, a fonte e como ele se arruma no quadro.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Rgb;

/// Fonte de quem não escolheu outra (spec TX, seção 3).
pub const DEFAULT_FONT: &str = "Inter";
/// Maior texto aceito, em caracteres (spec TX, seção 6).
pub const MAX_TEXT_CHARS: usize = 5000;
/// Faixa do tamanho, em pontos.
pub const TEXT_SIZE_PT: (f64, f64) = (4.0, 300.0);
/// Faixa da entrelinha, em vezes o tamanho.
pub const LINE_SPACING: (f64, f64) = (0.8, 3.0);

/// Alinhamento das linhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Onde o bloco de linhas fica na altura do quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextVAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// O conteúdo de um quadro de texto. Os campos depois de `color` chegaram com a spec TX; um
/// projeto que não os tem abre com os padrões.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextBox {
    pub text: String,
    /// Tamanho da fonte em pontos tipográficos.
    pub size_pt: f64,
    pub color: Rgb,
    /// Nome da família; uma desconhecida sai em [`DEFAULT_FONT`], mas o nome fica guardado.
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub valign: TextVAlign,
    /// Distância entre as linhas, em vezes o tamanho.
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f64,
    /// O texto em pé, para a lombada (spec CA, regra 9); o quadro continua retângulo.
    #[serde(
        rename = "giro",
        default,
        skip_serializing_if = "TextRotation::is_zero"
    )]
    pub rotation: TextRotation,
}

/// O giro do bloco de texto dentro do quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum TextRotation {
    #[default]
    #[serde(rename = "zero")]
    Zero,
    /// 90° para a esquerda: lê de baixo para cima.
    #[serde(rename = "cima")]
    Up,
    /// 90° para a direita: lê de cima para baixo.
    #[serde(rename = "baixo")]
    Down,
}

impl TextRotation {
    /// Sem giro: o campo nem vai para o arquivo.
    pub fn is_zero(&self) -> bool {
        *self == TextRotation::Zero
    }
}

fn default_font() -> String {
    DEFAULT_FONT.to_owned()
}

fn default_line_spacing() -> f64 {
    1.2
}

impl TextBox {
    /// Um texto vazio com os padrões, na cor dada.
    pub fn new(color: Rgb) -> TextBox {
        TextBox {
            text: String::new(),
            size_pt: 24.0,
            color,
            font: default_font(),
            bold: false,
            italic: false,
            align: TextAlign::default(),
            valign: TextVAlign::default(),
            line_spacing: default_line_spacing(),
            rotation: TextRotation::Zero,
        }
    }

    /// Só espaços (ou nada): não sai na impressão (regra 4).
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}

impl Rgb {
    /// A cor padrão do texto sobre um fundo: quase preto no claro, branco no escuro (regra 11).
    pub fn text_on(background: Rgb) -> Rgb {
        let luma = 0.2126 * f64::from(background.r)
            + 0.7152 * f64::from(background.g)
            + 0.0722 * f64::from(background.b);
        if luma > 127.5 {
            Rgb {
                r: 0x1A,
                g: 0x1A,
                b: 0x1A,
            }
        } else {
            Rgb::WHITE
        }
    }
}

/// Limpa o texto colado: tabulação vira espaço, outros caracteres de controle saem (fora a
/// quebra de linha) e o resto para em [`MAX_TEXT_CHARS`] (spec TX, seção 6).
pub fn clean_text(text: &str) -> String {
    text.chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| *c == '\n' || !c.is_control())
        .take(MAX_TEXT_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cor_padrao_depende_do_fundo() {
        assert_eq!(Rgb::text_on(Rgb::WHITE).r, 0x1A);
        assert_eq!(Rgb::text_on(Rgb::BLACK), Rgb::WHITE);
    }

    #[test]
    fn texto_colado_perde_tabulacao_e_controles() {
        assert_eq!(clean_text("a\tb\u{7}c\nd\r"), "a bc\nd");
        assert_eq!(clean_text(&"x".repeat(6000)).len(), MAX_TEXT_CHARS);
    }

    #[test]
    fn texto_so_com_espacos_e_vazio() {
        let mut t = TextBox::new(Rgb::BLACK);
        assert!(t.is_blank());
        t.text = "  \n ".to_owned();
        assert!(t.is_blank());
        t.text = "Ana".to_owned();
        assert!(!t.is_blank());
    }

    #[test]
    fn texto_antigo_abre_com_os_padroes() {
        let json = r#"{"text":"Oi","size_pt":12.0,"color":{"r":0,"g":0,"b":0}}"#;
        let t: TextBox = serde_json::from_str(json).unwrap();
        assert_eq!(t.font, DEFAULT_FONT);
        assert_eq!(t.align, TextAlign::Center);
        assert_eq!(t.valign, TextVAlign::Middle);
        assert!((t.line_spacing - 1.2).abs() < 1e-12);
    }
}
