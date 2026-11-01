//! Frases dos erros do editor, dos modelos e dos presets (`lamina-editor`).

use super::{CHECK_DISK, Human, HumanMessage, bug, name, technical};
use crate::{EditorError, ModelError, PresetError};

impl Human for EditorError {
    fn human(&self) -> HumanMessage {
        let details = technical(self);
        let (title, what) = match self {
            EditorError::MissingFolder(folder) => (
                format!("A pasta \"{}\" não existe.", name(folder)),
                "Confira se o disco ou o pen drive está conectado e escolha a pasta de novo."
                    .to_owned(),
            ),
            EditorError::NoPhotos(folder) => (
                format!("Não encontrei fotos na pasta \"{}\".", name(folder)),
                "Escolha uma pasta com fotos em JPEG, PNG ou TIFF.".to_owned(),
            ),
            EditorError::OnlyOtherFiles { folder, others } => (
                format!("Não encontrei fotos na pasta \"{}\".", name(folder)),
                format!(
                    "Ela tem {}, mas nenhum é foto em JPEG, PNG ou TIFF. Escolha a pasta das                      fotos.",
                    crate::empty_import::files(*others, "arquivo")
                ),
            ),
            EditorError::NoPhotoReadable { folder, found, .. } => (
                format!(
                    "Não consegui ler {} da pasta \"{}\".",
                    if *found == 1 { "a foto".to_owned() } else { format!("as {found} fotos") },
                    name(folder)
                ),
                "Confira se o disco está conectado e se o Lamina tem permissão para ler essa                  pasta; os motivos estão nos detalhes."
                    .to_owned(),
            ),
            EditorError::NoProject => (
                "Nenhum álbum está aberto.".to_owned(),
                "Abra um álbum ou crie um novo.".to_owned(),
            ),
            EditorError::NoFile => (
                "Este álbum ainda não foi salvo.".to_owned(),
                "Use Arquivo → Salvar como… para escolher onde guardar.".to_owned(),
            ),
            EditorError::NoExample(_) => (
                "O álbum de exemplo não veio com esta instalação.".to_owned(),
                "Instale o Lamina de novo pelo guia, ou crie um álbum com as suas fotos em Novo \
                 álbum…"
                    .to_owned(),
            ),
            EditorError::CopyExample { .. } => (
                "Não foi possível preparar o álbum de exemplo.".to_owned(),
                CHECK_DISK.to_owned(),
            ),
            EditorError::SpreadFull { spread, capacity } => (
                format!(
                    "A lâmina {} já tem {capacity} fotos, o máximo que cabe nela.",
                    spread + 1
                ),
                "Solte a foto sobre um quadro para trocar, ou leve-a para outra lâmina.".to_owned(),
            ),
            EditorError::PhotoFileMissing(_) => (
                "Não achei esta foto no computador.".to_owned(),
                "Ela pode ter sido movida ou apagada. Confira se o disco está conectado; se a foto \
                 mudou de pasta, use Reencontrar fotos."
                    .to_owned(),
            ),
            EditorError::AlreadyInSpread { spread } => (
                format!("Esta foto já está na lâmina {}.", spread + 1),
                format!(
                    "Para mudá-la de lugar dentro dela, vá até a lâmina {} e arraste lá.",
                    spread + 1
                ),
            ),
            EditorError::LastPhoto { spread } => (
                format!("A lâmina {} ficaria sem fotos.", spread + 1),
                "Leve outra foto para ela antes, ou apague a lâmina se não precisa mais dela."
                    .to_owned(),
            ),
            EditorError::BlankPage => (
                crate::blank::BLANK_SPREAD.to_owned(),
                "Clique com o botão direito no fundo da página e desmarque Página em branco."
                    .to_owned(),
            ),
            EditorError::NoFrame => return bug(details),
            EditorError::Media(e) => return e.human(),
            EditorError::Build(e) => return e.human(),
            EditorError::File(e) => return e.human(),
            EditorError::Export(e) => return e.human(),
            EditorError::Core(e) => return e.human(),
            EditorError::Layout(e) => return e.human(),
        };
        HumanMessage::new(title, what, details)
    }
}

impl Human for ModelError {
    fn human(&self) -> HumanMessage {
        let (title, what) = match self {
            ModelError::EmptyName => (
                "O modelo precisa de um nome.".to_owned(),
                "Escreva um nome e salve de novo.".to_owned(),
            ),
            ModelError::NoPhotoFrame => (
                "Esta lâmina não tem quadro de foto para virar modelo.".to_owned(),
                "Escolha uma lâmina com fotos.".to_owned(),
            ),
            ModelError::TooMany {
                frames,
                pages,
                capacity,
            } => (
                format!("São {frames} fotos para {pages} páginas, e cabem no máximo {capacity}."),
                "Escolha uma lâmina com menos fotos para virar modelo.".to_owned(),
            ),
            ModelError::Exists(model) => (
                format!("Já existe um modelo chamado \"{model}\"."),
                "Escolha outro nome.".to_owned(),
            ),
            ModelError::Write(_) => (
                "Não foi possível guardar os modelos.".to_owned(),
                CHECK_DISK.to_owned(),
            ),
        };
        HumanMessage::new(title, what, technical(self))
    }
}

impl Human for PresetError {
    fn human(&self) -> HumanMessage {
        let (title, what) = match self {
            PresetError::EmptyName => (
                "O preset precisa de um nome.".to_owned(),
                "Escreva um nome e salve de novo.".to_owned(),
            ),
            PresetError::Builtin => (
                "Esse nome já é de um preset do Lamina.".to_owned(),
                "Escolha outro nome.".to_owned(),
            ),
            PresetError::LimitsOrder => (
                "O mínimo de páginas passa do máximo.".to_owned(),
                "Corrija o mínimo ou o máximo de páginas.".to_owned(),
            ),
            // O texto já vem da frase humana do núcleo (presets.rs).
            PresetError::Spec(reason) => (
                "As medidas deste preset não servem.".to_owned(),
                reason.clone(),
            ),
            PresetError::Exists(preset) => (
                format!("Já existe um preset chamado \"{preset}\"."),
                "Escolha outro nome.".to_owned(),
            ),
            PresetError::Write(_) => (
                "Não foi possível guardar os presets.".to_owned(),
                CHECK_DISK.to_owned(),
            ),
        };
        HumanMessage::new(title, what, technical(self))
    }
}
