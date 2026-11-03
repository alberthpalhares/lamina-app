//! A aparência da janela (spec AJ): as quatro escolhas da tela de Preferências, como ficam em
//! `janela.json` e como cada uma se resolve com a resposta do sistema. Nada disso vai para o álbum
//! nem para a exportação (regra 24).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

use crate::text_scale::{TEXT_SCALE_RANGE, text_scale};

/// O tema (regra 8): escuro é o padrão, mesmo com o sistema em claro.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    #[default]
    #[serde(rename = "escuro")]
    Dark,
    #[serde(rename = "claro")]
    Light,
    #[serde(rename = "sistema")]
    System,
}

impl ThemeChoice {
    /// O tema vale escuro? `system`: se o sistema está em escuro; sem resposta, escuro (regra 9).
    pub fn is_dark(self, system: Option<bool>) -> bool {
        match self {
            ThemeChoice::Dark => true,
            ThemeChoice::Light => false,
            ThemeChoice::System => system.unwrap_or(true),
        }
    }
}

/// O fundo em volta da lâmina (regra 13), independente do tema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackdropChoice {
    #[default]
    #[serde(rename = "tema")]
    Theme,
    #[serde(rename = "escuro")]
    Dark,
    #[serde(rename = "cinza")]
    Gray,
    #[serde(rename = "claro")]
    Light,
}

/// O movimento (regra 18): o padrão é seguir o sistema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotionChoice {
    #[default]
    #[serde(rename = "sistema")]
    System,
    #[serde(rename = "com")]
    On,
    #[serde(rename = "sem")]
    Off,
}

impl MotionChoice {
    /// Reduz o movimento? `system`: se o sistema pede; sem resposta, com movimento (regra 18).
    pub fn reduces(self, system: Option<bool>) -> bool {
        match self {
            MotionChoice::System => system.unwrap_or(false),
            MotionChoice::On => false,
            MotionChoice::Off => true,
        }
    }

    /// O que `ver.reduzir-movimento` faz (regra 7): de "sem" a "sistema", dos outros a "sem".
    pub fn toggled(self) -> MotionChoice {
        match self {
            MotionChoice::Off => MotionChoice::System,
            MotionChoice::System | MotionChoice::On => MotionChoice::Off,
        }
    }
}

/// Os tamanhos do texto, em porcentagem (regra 15).
pub const TEXT_SIZES: [u32; 5] = [90, 100, 110, 120, 130];

/// O tamanho do texto de quem nunca escolheu.
pub const TEXT_SIZE_DEFAULT: u32 = 100;

/// Um tamanho válido: preso à faixa, na dezena mais próxima (em empate, a de cima).
pub fn text_size(percent: f64) -> u32 {
    let (low, high) = TEXT_SCALE_RANGE;
    let clamped = percent.clamp(f64::from(low) * 100.0, f64::from(high) * 100.0);
    let tens = (clamped / 10.0 + 0.5).floor();
    // A faixa cabe de sobra num `u32`; um valor inválido não chega aqui (regra 23).
    format!("{tens:.0}").parse::<u32>().unwrap_or(10) * 10
}

/// O fator de `Theme.text-scale` para um tamanho em porcentagem.
pub fn scale_of(size: u32) -> f32 {
    let hundredths = u16::try_from(size).map_or(f32::NAN, f32::from);
    text_scale(hundredths / 100.0)
}

/// As quatro escolhas juntas, como a tela de Preferências as mostra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowLook {
    pub theme: ThemeChoice,
    pub backdrop: BackdropChoice,
    pub text_size: u32,
    pub motion: MotionChoice,
}

/// Lê um campo sem derrubar o arquivo (regra 23): o que o Lamina não conhece vale o padrão.
pub(crate) fn tolerant<'de, D, T>(from: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = serde_json::Value::deserialize(from)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

/// `tamanho_texto`: número vai para a dezena válida; outra coisa vale o padrão (regras 15 e 23).
pub(crate) fn tolerant_size<'de, D: Deserializer<'de>>(from: D) -> Result<u32, D::Error> {
    let value = serde_json::Value::deserialize(from)?;
    Ok(value.as_f64().map_or(TEXT_SIZE_DEFAULT, text_size))
}

/// `movimento`: presente, mas estranho, vale o padrão; ausente, quem decide é `reduzir_movimento`
/// (regras 21 e 23).
pub(crate) fn tolerant_motion<'de, D: Deserializer<'de>>(
    from: D,
) -> Result<Option<MotionChoice>, D::Error> {
    let value = serde_json::Value::deserialize(from)?;
    Ok(Some(serde_json::from_value(value).unwrap_or_default()))
}

/// O que `defaults read com.apple.universalaccess reduceMotion` responde (ADR 0016).
pub fn mac_reduces(reply: &str) -> Option<bool> {
    match reply.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

/// O que `gsettings get org.gnome.desktop.interface enable-animations` responde (ADR 0016):
/// a pergunta é o contrário, se há animações.
pub fn gnome_reduces(reply: &str) -> Option<bool> {
    match reply.trim() {
        "false" => Some(true),
        "true" => Some(false),
        _ => None,
    }
}

/// Se a barra de menus nativa fica diferente do tema (spec AJ, regra 34): a cor dela é fixada ao criar a
/// janela. `opened`: o tema pedido ao abrir (`None` = seguindo o sistema).
pub fn menu_differs(dark: bool, opened: Option<bool>, system: Option<bool>) -> bool {
    opened.or(system).is_some_and(|menu| menu != dark)
}

/// O tema do sistema no Windows (ADR 0016): direto, porque o valor do Slint guarda o tema pedido ao
/// abrir. Nos outros sistemas, `None`: quem responde é o componente de leitura.
pub fn system_is_dark() -> Option<bool> {
    read_dark()
}

#[cfg(windows)]
fn read_dark() -> Option<bool> {
    use windows::UI::ViewManagement::{UIColorType, UISettings};
    let color = UISettings::new()
        .ok()?
        .GetColorValue(UIColorType::Background)
        .ok()?;
    Some(color.R < 128)
}

#[cfg(not(windows))]
fn read_dark() -> Option<bool> {
    None
}

/// Lê se o sistema pede menos movimento (ADR 0016, decisão 3). Pode levar alguns milissegundos:
/// quem chama fica fora da linha da janela. `None` = sem resposta.
pub fn system_reduces_motion() -> Option<bool> {
    read_system()
}

#[cfg(windows)]
fn read_system() -> Option<bool> {
    let settings = windows::UI::ViewManagement::UISettings::new().ok()?;
    settings.AnimationsEnabled().ok().map(|enabled| !enabled)
}

#[cfg(target_os = "macos")]
fn read_system() -> Option<bool> {
    mac_reduces(&reply_of(
        "defaults",
        &["read", "com.apple.universalaccess", "reduceMotion"],
    )?)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn read_system() -> Option<bool> {
    gnome_reduces(&reply_of(
        "gsettings",
        &["get", "org.gnome.desktop.interface", "enable-animations"],
    )?)
}

#[cfg(not(windows))]
fn reply_of(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
