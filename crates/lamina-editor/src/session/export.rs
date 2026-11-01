//! A sessão na exportação pela janela (spec EI, seção 3 e regras 2 e 4): a pasta de saída e a
//! conferência antes de exportar.

use std::path::{Path, PathBuf};

use lamina_export::{ExportOptions, output_names, parse_range, preflight};

use super::Session;
use crate::EditorError;
use crate::exporting::ExportCheck;

/// Pasta padrão da primeira exportação, ao lado do arquivo do projeto.
const EXPORT_FOLDER: &str = "exportacao";

impl Session {
    /// A última pasta usada neste projeto; a primeira vez, `exportacao` ao lado do arquivo do
    /// projeto (ou das fotos, num álbum ainda sem arquivo).
    pub fn export_folder(&self) -> PathBuf {
        if let Some(folder) = &self.export_folder {
            return folder.clone();
        }
        let base = self
            .file
            .as_deref()
            .and_then(Path::parent)
            .or_else(|| self.album().photos.first().and_then(|p| p.path.parent()))
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        base.join(EXPORT_FOLDER)
    }

    pub fn set_export_folder(&mut self, folder: &Path) {
        self.export_folder = Some(folder.to_path_buf());
    }

    /// Antes de exportar: os avisos das lâminas escolhidas e os arquivos de saída que já existem
    /// em `folder`. Opções inválidas (intervalo, qualidade) dão o erro da EX.
    pub fn check_export(
        &self,
        folder: &Path,
        options: &ExportOptions,
    ) -> Result<ExportCheck, EditorError> {
        let project = self.album();
        let names = output_names(project, options)?;
        let total = project.spreads.len();
        let spreads = match &options.range {
            Some(text) => parse_range(text, total)?,
            None => (0..total).collect(),
        };
        Ok(ExportCheck {
            warnings: preflight(project, &spreads),
            existing: names
                .into_iter()
                .filter(|name| folder.join(name).exists())
                .collect(),
            album: self.page_warning(),
        })
    }
}
