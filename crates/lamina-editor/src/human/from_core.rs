//! Frases dos erros do núcleo (`lamina-core`).

use lamina_core::{CoreError, Dpi};

use super::{Human, HumanMessage, REPORT, bug, technical};

impl Human for CoreError {
    fn human(&self) -> HumanMessage {
        let details = technical(self);
        let (title, what) = match self {
            CoreError::InvalidDpi(_) => (
                format!("A resolução precisa ficar entre 1 e {} DPI.", Dpi::MAX),
                "Corrija o campo DPI. Para impressão, o comum é 300.".to_owned(),
            ),
            CoreError::InvalidLength { field, .. } => (
                format!(
                    "A {} do álbum precisa ser maior que zero.",
                    length_name(field)
                ),
                "Use um número positivo, em milímetros.".to_owned(),
            ),
            CoreError::InvalidMargin { margin, .. } => (
                format!("A {} não pode ser negativa.", margin_name(margin)),
                "Use 0 ou um número positivo, em milímetros.".to_owned(),
            ),
            CoreError::SafeMarginTooLarge { .. } => (
                "A zona segura é maior que a página.".to_owned(),
                "Diminua a zona segura ou aumente o tamanho do álbum.".to_owned(),
            ),
            CoreError::Migration { .. } => (
                "Não foi possível atualizar este álbum, salvo numa versão antiga do Lamina."
                    .to_owned(),
                format!("Crie o álbum de novo a partir das fotos. {REPORT}"),
            ),
            CoreError::UnsupportedSchema { .. } => (
                "Este álbum foi salvo por uma versão mais nova do Lamina.".to_owned(),
                "Atualize em Ajuda → Verificar atualizações agora e abra o álbum de novo."
                    .to_owned(),
            ),
            CoreError::InvalidAdjust { photo } => (
                format!("A foto {photo} tem um ajuste inválido."),
                "Abra uma cópia de segurança do álbum. O ângulo vai de −45° a 45°; brilho e \
                 contraste, de −100 a 100."
                    .to_owned(),
            ),
            CoreError::InvalidCover { .. } => (
                "A capa deste álbum tem medidas inválidas.".to_owned(),
                "Abra uma cópia de segurança do álbum.".to_owned(),
            ),
            CoreError::InvalidOpacity { value, spread } => (
                format!("Um quadro da lâmina {spread} tem uma opacidade inválida ({value}%)."),
                "Abra uma cópia de segurança do álbum. A opacidade vai de 0 a 100%.".to_owned(),
            ),
            CoreError::InvalidName(name) => (
                format!("O nome \"{name}\" tem caracteres que não servem para nome de arquivo."),
                "Use só letras, números, espaços, - e _.".to_owned(),
            ),
            CoreError::Json(_) => (
                "Este arquivo de álbum está danificado ou não é um álbum do Lamina.".to_owned(),
                "Abra uma cópia de segurança do álbum ou crie o álbum de novo a partir das fotos."
                    .to_owned(),
            ),
            CoreError::InvalidRelRect(_)
            | CoreError::InvalidCrop(_)
            | CoreError::UnknownPhoto(_)
            | CoreError::UnknownSpread(_)
            | CoreError::UnknownFrame(_) => return bug(details),
        };
        HumanMessage::new(title, what, details)
    }
}

fn length_name(field: &str) -> &'static str {
    if field == "page_height" {
        "altura da página"
    } else {
        "largura da página"
    }
}

fn margin_name(margin: &str) -> &'static str {
    if margin == "safe" {
        "zona segura"
    } else {
        "sangria"
    }
}
