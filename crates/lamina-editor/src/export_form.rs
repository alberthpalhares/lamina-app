//! A janela "Exportar" (spec EI, seção 3; specs PD, CT e PR): os campos, as opções que eles dão,
//! o que o projeto lembra e a pergunta antes de substituir. Saiu do `forms.rs` (auditoria do
//! Marco 3, M10).

use std::path::Path;

use lamina_core::{ColorSpace, ExportPrefs, OutputFormat};
use lamina_export::CoverExport;

use crate::exporting::{DEFAULT_QUALITY, ExportFormat, ExportOptions};

/// Posição do formato no campo Formato da janela Exportar (JPEG, PDF, TIFF).
pub fn format_index(format: OutputFormat) -> i32 {
    match format {
        OutputFormat::Jpeg => 0,
        OutputFormat::Pdf => 1,
        OutputFormat::Tiff => 2,
    }
}

/// Os campos da janela "Exportar" (spec EI, seção 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportForm {
    pub quality: i32,
    pub all_spreads: bool,
    pub range: String,
    pub pages: bool,
    /// JPEG, PDF (spec PD) ou TIFF (spec CT).
    pub format: ExportFormat,
    /// Marcas de corte; só valem no PDF.
    pub crop_marks: bool,
    /// sRGB ou Adobe RGB (spec CT).
    pub space: ColorSpace,
    /// A capa: junto, só ela ou sem ela (spec CA, regra 13).
    pub cover: CoverExport,
}

impl ExportForm {
    /// O que o projeto passa a lembrar desta exportação (spec PR, regra 5).
    pub fn prefs(&self) -> ExportPrefs {
        ExportPrefs {
            format: match self.format {
                ExportFormat::Pdf => OutputFormat::Pdf,
                ExportFormat::Tiff => OutputFormat::Tiff,
                _ => OutputFormat::Jpeg,
            },
            space: self.space,
            pages: self.pages,
        }
    }

    /// As opções da exportação; a qualidade fica entre 1 e 100, e as marcas de corte só valem
    /// no PDF (no JPEG a janela nem mostra a opção).
    pub fn options(&self, overwrite: bool) -> ExportOptions {
        ExportOptions {
            quality: u8::try_from(self.quality.clamp(1, 100)).unwrap_or(DEFAULT_QUALITY),
            range: (!self.all_spreads).then(|| self.range.trim().to_owned()),
            pages: self.pages,
            overwrite,
            format: self.format,
            crop_marks: self.format == ExportFormat::Pdf && self.crop_marks,
            space: self.space,
            cover: self.cover,
        }
    }
}

/// A pergunta antes de substituir arquivos que já existem (spec EI, regra 4).
pub fn replace_question(existing: usize, folder: &Path) -> String {
    let files = if existing == 1 {
        "1 arquivo desta exportação".to_owned()
    } else {
        format!("{existing} arquivos desta exportação")
    };
    format!(
        "Já existem {files} em {}. Substituindo, os arquivos lamina-NN, pagina-NN e da capa do \
         mesmo formato que não fizerem parte desta exportação também saem da pasta; os outros \
         arquivos ficam.",
        folder.display()
    )
}
