//! Frases dos erros da exportação e do arquivo do álbum (`lamina-export`).

use lamina_export::{ExportError, ProjectFileError};

use super::{CHECK_DISK, Human, HumanMessage, REPORT, name, technical};

impl Human for ExportError {
    fn human(&self) -> HumanMessage {
        let details = technical(self);
        let (title, what) = match self {
            ExportError::Quality(_) => (
                "A qualidade do JPEG vai de 1 a 100.".to_owned(),
                "Corrija o campo Qualidade. O padrão é 95.".to_owned(),
            ),
            ExportError::CropMarksNeedPdf => (
                "As marcas de corte só existem no PDF.".to_owned(),
                "Escolha o formato PDF ou desligue as marcas de corte.".to_owned(),
            ),
            ExportError::Format(format) => (
                format!("O Lamina ainda não exporta em {format}."),
                "Escolha JPEG, TIFF ou PDF.".to_owned(),
            ),
            ExportError::BadRange(range) => (
                format!("O intervalo \"{range}\" não foi entendido."),
                "Escreva as lâminas assim: 1, 2, 5-7.".to_owned(),
            ),
            ExportError::RangeTooBig { asked, count } => (
                format!("O intervalo pede a lâmina {asked}, mas o álbum tem {count}."),
                format!("Use lâminas de 1 a {count}."),
            ),
            ExportError::Exists { name: file, folder } => (
                format!("Já existe \"{file}\" na pasta \"{}\".", name(folder)),
                "Confirme Substituir para trocar os arquivos, ou escolha outra pasta.".to_owned(),
            ),
            ExportError::NoSpreads => (
                "O álbum não tem lâminas para exportar.".to_owned(),
                "Monte pelo menos uma lâmina e exporte de novo.".to_owned(),
            ),
            ExportError::Folder { folder, .. } => (
                format!("Não foi possível criar a pasta \"{}\".", name(folder)),
                "Escolha outra pasta para exportar, ou confira se há espaço no disco e se a pasta \
                 pode ser gravada."
                    .to_owned(),
            ),
            ExportError::Write { path, .. } => (
                format!("Não foi possível gravar \"{}\".", name(path)),
                CHECK_DISK.to_owned(),
            ),
            ExportError::Remove { path, .. } => (
                format!(
                    "Não foi possível apagar o arquivo antigo \"{}\".",
                    name(path)
                ),
                "Feche o arquivo, se estiver aberto em outro programa, e exporte de novo."
                    .to_owned(),
            ),
            ExportError::Encode(_) => (
                "Não foi possível gerar a imagem da lâmina.".to_owned(),
                format!("Exporte de novo. {REPORT}"),
            ),
            ExportError::Render(e) => return e.human(),
            ExportError::Cover(e) => return e.human(),
            ExportError::Stopped { done, source } => return stopped(done.len(), source, details),
        };
        HumanMessage::new(title, what, details)
    }
}

/// A exportação parou no meio: a causa mais funda e quantos arquivos ficaram (regra 8).
fn stopped(done: usize, source: &ExportError, details: String) -> HumanMessage {
    let mut cause = source;
    while let ExportError::Stopped { source, .. } = cause {
        cause = source;
    }
    let inner = cause.human();
    let kept = match done {
        0 => "Nenhum arquivo foi gravado.".to_owned(),
        1 => "1 arquivo já foi gravado.".to_owned(),
        n => format!("{n} arquivos já foram gravados."),
    };
    HumanMessage::new(
        format!("A exportação parou: {} {kept}", lower_first(&inner.title)),
        inner.what_to_do,
        details,
    )
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_lowercase().chain(chars).collect::<String>()
    })
}

impl Human for ProjectFileError {
    fn human(&self) -> HumanMessage {
        let details = technical(self);
        let (title, what) = match self {
            ProjectFileError::Read { path, .. } => (
                format!("Não foi possível abrir o álbum \"{}\".", name(path)),
                "Confira se o disco ou o pen drive está conectado e se o arquivo ainda está lá, \
                 e abra de novo pelo Arquivo → Abrir…"
                    .to_owned(),
            ),
            ProjectFileError::Write { path, .. } => (
                format!("Não foi possível salvar \"{}\".", name(path)),
                "Confira se há espaço no disco e se a pasta pode ser gravada, ou use Arquivo → \
                 Salvar como… em outra pasta."
                    .to_owned(),
            ),
            ProjectFileError::MissingPhoto { name: photo, .. } => (
                format!("A foto \"{}\" não está mais onde estava.", name(photo)),
                "Use Arquivo → Reencontrar fotos… e escolha a pasta onde ela está agora."
                    .to_owned(),
            ),
            ProjectFileError::Core(e) => return e.human(),
        };
        HumanMessage::new(title, what, details)
    }
}
