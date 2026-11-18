//! Frases dos erros das fotos (`lamina-media`) e da renderização (`lamina-render`).

use std::io::ErrorKind;

use lamina_media::MediaError;
use lamina_render::RenderError;

use super::{Human, HumanMessage, REPORT, bug, name, technical};

/// O que fazer com uma foto que não abre: ela pode ter mudado de lugar ou estar corrompida.
const PHOTO_WHAT: &str = "Se ela mudou de lugar, use Arquivo → Reencontrar fotos…; se \
                          estiver corrompida, exporte-a de novo pelo seu editor.";

impl Human for MediaError {
    fn human(&self) -> HumanMessage {
        let details = technical(self);
        let (title, what) = match self {
            MediaError::Io { path, source } => {
                let file = name(&path.to_string_lossy());
                if source.kind() == ErrorKind::PermissionDenied {
                    (
                        format!("O Lamina não tem permissão para ler \"{file}\"."),
                        "Copie as fotos para uma pasta sua e tente de novo.".to_owned(),
                    )
                } else {
                    (
                        format!("Não foi possível ler \"{file}\"."),
                        "Confira se o disco ou o pen drive está conectado e se o arquivo ainda \
                         está lá, e tente de novo."
                            .to_owned(),
                    )
                }
            }
            MediaError::Image { path, .. } => (
                format!(
                    "A foto \"{}\" está corrompida ou num formato que o Lamina não lê.",
                    name(&path.to_string_lossy())
                ),
                "Exporte a foto de novo pelo seu editor, em JPEG ou TIFF, e tente outra vez."
                    .to_owned(),
            ),
            MediaError::Catalog(_) => (
                "O Lamina não conseguiu usar a pasta de miniaturas das fotos.".to_owned(),
                format!("Feche e abra o Lamina de novo. {REPORT}"),
            ),
            MediaError::TooLarge { path, .. } => (
                format!(
                    "A foto \"{}\" é grande demais para importar.",
                    name(&path.to_string_lossy())
                ),
                "O Lamina lê arquivos de até 1,5 GB. Exporte uma versão menor da foto, em JPEG ou \
                 TIFF, e importe de novo."
                    .to_owned(),
            ),
        };
        HumanMessage::new(title, what, details)
    }
}

impl Human for RenderError {
    fn human(&self) -> HumanMessage {
        let details = technical(self);
        let (title, what) = match self {
            RenderError::NoSpread { number, count } => (
                format!("A lâmina {number} não existe; o álbum tem {count}."),
                format!("Escolha uma lâmina de 1 a {count}."),
            ),
            RenderError::Photo { name: photo, .. } => (
                format!("Não foi possível ler a foto \"{}\".", name(photo)),
                PHOTO_WHAT.to_owned(),
            ),
            RenderError::Resize { name: photo, .. } => (
                format!(
                    "Não foi possível preparar a foto \"{}\" para o quadro.",
                    name(photo)
                ),
                format!("Tente de novo. {REPORT}"),
            ),
            RenderError::Canvas { .. } => (
                "A imagem da lâmina ficou grande demais para a memória do computador.".to_owned(),
                "Feche outros programas e exporte de novo, ou use um DPI menor.".to_owned(),
            ),
            RenderError::PreviewTooSmall => return bug(details),
            RenderError::Core(e) => return e.human(),
        };
        HumanMessage::new(title, what, details)
    }
}
