//! A sessão na aprovação do cliente (spec AC, seção 3 e regras 15 e 16): a pasta de saída de cada álbum
//! e a conferência antes de gerar.

use std::path::{Path, PathBuf};

use lamina_export::{
    ApprovalOptions, approval_existing, approval_photos_found, approval_warnings, html_cover_notice,
};

use super::Session;
use crate::EditorError;
use crate::approval_form::ApprovalCheck;

/// Pasta padrão da primeira geração, ao lado do arquivo do álbum.
const APPROVAL_FOLDER: &str = "aprovacao-cliente";

impl Session {
    /// A última pasta usada neste álbum, nesta sessão; a primeira vez, `aprovacao-cliente` ao lado do
    /// arquivo do álbum (ou das fotos, num álbum ainda sem arquivo).
    pub fn approval_folder(&self) -> PathBuf {
        if let Some(folder) = &self.approval_folder {
            return folder.clone();
        }
        let base = self
            .file
            .as_deref()
            .and_then(Path::parent)
            .or_else(|| self.album().photos.first().and_then(|p| p.path.parent()))
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        base.join(APPROVAL_FOLDER)
    }

    pub fn set_approval_folder(&mut self, folder: &Path) {
        self.approval_folder = Some(folder.to_path_buf());
    }

    /// Antes de gerar: foto não encontrada para tudo, e os avisos e arquivos a substituir vão para a
    /// caixa. Um intervalo inválido dá o erro da EX.
    pub fn check_approval(
        &self,
        folder: &Path,
        options: &ApprovalOptions,
    ) -> Result<ApprovalCheck, EditorError> {
        let project = self.album();
        approval_photos_found(project, options)?;
        Ok(ApprovalCheck {
            warnings: approval_warnings(project, options)?,
            existing: approval_existing(project, folder, options)?,
            cover: html_cover_notice(project, options).map(str::to_owned),
        })
    }
}
