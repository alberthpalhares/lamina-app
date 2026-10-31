//! A cópia de segurança no editor (spec CS): o passo de cada minuto, o fim direito e a
//! recuperação depois de uma queda.

use lamina_export::read_project;

use super::Editor;
use crate::Session;
use crate::backup::{BackupJob, Backups, FoundBackup};

impl Editor {
    /// Passa a guardar cópias na pasta de `backups` (a janela; os testes escolhem a pasta).
    pub fn set_backups(&mut self, backups: Backups) {
        self.backups = Some(backups);
    }

    /// A cópia a gravar agora, se o álbum mudou desde a última (regras 1 a 3).
    pub fn backup_step(&mut self) -> Option<BackupJob> {
        let session = self.session.as_ref();
        self.backups.as_mut()?.step(session)
    }

    /// O Lamina fecha direito: a cópia desta execução não fica (regras 3 e 9).
    pub fn finish_backups(&mut self) {
        if let Some(backups) = self.backups.as_mut() {
            backups.finish();
        }
    }

    /// As cópias que ficaram de uma queda, a mais nova primeiro (regra 4).
    pub fn found_backups(&self) -> Vec<FoundBackup> {
        self.backups
            .as_ref()
            .map(Backups::found)
            .unwrap_or_default()
    }

    /// "Recuperar" (regra 5): abre o álbum da cópia, na lâmina em que estava, com as alterações
    /// ainda não salvas. `false` se a cópia não abriu (ela é descartada).
    pub fn recover(&mut self, found: &FoundBackup) -> bool {
        let Some(backups) = self.backups.as_ref() else {
            return false;
        };
        let Some(project) = backups.project(found) else {
            backups.discard(found);
            return false;
        };
        let on_disk = found.file.as_deref().and_then(|f| read_project(f).ok());
        let session = Session::recovered(project, found.file.clone(), on_disk, found.spread);
        if let Some(backups) = self.backups.as_mut() {
            backups.recovered(found, &session);
        }
        self.remember_current();
        self.forget_view();
        self.session = Some(session);
        // Na lâmina da cópia, não na dos recentes.
        self.note_session();
        true
    }

    /// "Descartar" (regra 6): a cópia vai embora e o arquivo do álbum fica como estava.
    pub fn discard_backup(&mut self, found: &FoundBackup) {
        if let Some(backups) = self.backups.as_ref() {
            backups.discard(found);
        }
    }
}
