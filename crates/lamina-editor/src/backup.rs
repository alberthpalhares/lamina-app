//! A cópia de segurança (spec CS): enquanto o álbum tem alterações não salvas, uma cópia dele numa
//! pasta do Lamina, para recuperar o trabalho se o programa fechar sem salvar. O arquivo `.lamina`
//! do fotógrafo só muda quando ele salva.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use lamina_core::Project;
use lamina_export::{read_project, save_project};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::recents::{Clock, now, when_text};

/// A pasta das cópias, dentro da pasta de configuração.
pub const BACKUP_FOLDER: &str = "recuperacao";

/// De quanto em quanto tempo a janela pede uma cópia nova (regra 1).
pub const BACKUP_EVERY: Duration = Duration::from_secs(60);

/// A chave do álbum ainda sem arquivo.
const NO_FILE: &str = "sem-arquivo";

/// O que acompanha a cópia: de que álbum ela é, quando foi feita e a lâmina em que estava.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Meta {
    arquivo: Option<PathBuf>,
    nome: String,
    quando: i64,
    lamina: usize,
}

/// As cópias de uma pasta e a última que esta janela gravou.
#[derive(Debug)]
pub struct Backups {
    folder: PathBuf,
    /// Conta cada apagar: uma cópia pedida antes de um apagar não grava depois dele (a cópia
    /// grava fora da linha da janela, e salvar logo depois não pode deixar uma cópia para trás).
    turn: Arc<Mutex<u64>>,
    /// A chave e o projeto da última cópia: sem mudança desde ela, não grava de novo.
    written: Option<(String, Project)>,
}

/// Uma cópia para gravar fora da linha da janela.
#[derive(Debug)]
pub struct BackupJob {
    folder: PathBuf,
    turn: Arc<Mutex<u64>>,
    asked_at: u64,
    key: String,
    project: Project,
    meta: Meta,
}

/// Uma cópia achada na abertura: o trabalho que ficou sem salvar.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundBackup {
    pub key: String,
    pub name: String,
    /// O arquivo do álbum; `None` se ele nunca foi salvo.
    pub file: Option<PathBuf>,
    pub when: i64,
    pub spread: usize,
}

impl FoundBackup {
    /// A pergunta da abertura (regra 4).
    pub fn question(&self, clock: Clock) -> String {
        format!(
            "O Lamina fechou sem salvar \"{}\". Recuperar as alterações de {}?",
            self.name,
            when_text(self.when, clock)
        )
    }
}

impl Backups {
    pub fn new(folder: PathBuf) -> Self {
        Backups {
            folder,
            turn: Arc::default(),
            written: None,
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// O passo de cada minuto e de cada salvar ou fechar (regras 1 a 3): a cópia a gravar, se o
    /// álbum mudou desde a última; sem alterações (ou sem álbum), a cópia desta execução é apagada.
    /// A que ficou de uma queda espera a resposta do fotógrafo (regra 9).
    pub fn step(&mut self, session: Option<&Session>) -> Option<BackupJob> {
        let key = session.map(|s| key_of(s.file()));
        if let Some((old, _)) = &self.written
            && key.as_ref() != Some(old)
        {
            self.remove(&old.clone());
            self.written = None;
        }
        let (session, key) = (session?, key?);
        if !session.is_dirty() {
            if let Some((old, _)) = self.written.take() {
                self.remove(&old);
            }
            return None;
        }
        let project = session.for_disk();
        if self
            .written
            .as_ref()
            .is_some_and(|(k, p)| *k == key && *p == project)
        {
            return None;
        }
        self.written = Some((key.clone(), project.clone()));
        let asked_at = *self.turn.lock().unwrap_or_else(PoisonError::into_inner);
        Some(BackupJob {
            folder: self.folder.clone(),
            turn: Arc::clone(&self.turn),
            asked_at,
            meta: Meta {
                arquivo: session.file().map(Path::to_path_buf),
                nome: session.title().to_owned(),
                quando: now(),
                lamina: session.current(),
            },
            key,
            project,
        })
    }

    /// Fechou direito (o fotógrafo já respondeu "Salvar as alterações?"): a cópia desta execução
    /// não fica. A de uma queda ainda sem resposta volta a ser perguntada (regra 9).
    pub fn finish(&mut self) {
        if let Some((old, _)) = self.written.take() {
            self.remove(&old);
        }
    }

    /// "Recuperar": a cópia passa a ser desta execução, e salvar ou fechar a apaga (regras 2 e 3).
    pub fn recovered(&mut self, found: &FoundBackup, session: &Session) {
        if let Some((old, _)) = self.written.take()
            && old != found.key
        {
            self.remove(&old);
        }
        self.written = Some((found.key.clone(), session.for_disk()));
    }

    /// As cópias que ficaram de uma queda, a mais nova primeiro. Uma cópia que não dá para ler
    /// é apagada (regra 7).
    pub fn found(&self) -> Vec<FoundBackup> {
        let Ok(entries) = std::fs::read_dir(&self.folder) else {
            return Vec::new();
        };
        let mut found: Vec<FoundBackup> = entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let path = e.path();
                let key = path.file_stem()?.to_str()?.to_owned();
                (path.extension()? == "json").then_some(key)
            })
            .filter_map(|key| self.read(&key))
            .collect();
        found.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| a.key.cmp(&b.key)));
        found
    }

    fn read(&self, key: &str) -> Option<FoundBackup> {
        let meta = std::fs::read_to_string(self.meta_path(key))
            .ok()
            .and_then(|text| serde_json::from_str::<Meta>(&text).ok());
        let Some(meta) = meta.filter(|_| self.copy_path(key).is_file()) else {
            self.remove(key);
            return None;
        };
        Some(FoundBackup {
            key: key.to_owned(),
            name: meta.nome,
            file: meta.arquivo,
            when: meta.quando,
            spread: meta.lamina,
        })
    }

    /// O projeto da cópia, com as fotos achadas como no álbum.
    pub fn project(&self, found: &FoundBackup) -> Option<Project> {
        read_project(&self.copy_path(&found.key)).ok()
    }

    /// "Descartar": a cópia vai embora e o arquivo do álbum fica como estava.
    pub fn discard(&self, found: &FoundBackup) {
        self.remove(&found.key);
    }

    fn copy_path(&self, key: &str) -> PathBuf {
        self.folder.join(format!("{key}.lamina"))
    }

    fn meta_path(&self, key: &str) -> PathBuf {
        self.folder.join(format!("{key}.json"))
    }

    fn remove(&self, key: &str) {
        let mut turn = self.turn.lock().unwrap_or_else(PoisonError::into_inner);
        *turn += 1;
        // Já apagada ou sem pasta: nada a fazer.
        let _ = std::fs::remove_file(self.meta_path(key));
        let _ = std::fs::remove_file(self.copy_path(key));
    }
}

impl BackupJob {
    /// Grava a cópia e, depois, o que a acompanha (sem ele, a cópia não aparece na abertura).
    pub fn write(self) -> Result<(), String> {
        let turn = self.turn.lock().unwrap_or_else(PoisonError::into_inner);
        if *turn != self.asked_at {
            // Apagada depois do pedido (salvou ou fechou): a cópia ficou velha.
            return Ok(());
        }
        std::fs::create_dir_all(&self.folder).map_err(|e| e.to_string())?;
        save_project(&self.project, &self.folder, &format!("{}.lamina", self.key))
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&self.meta).map_err(|e| e.to_string())?;
        let mut tmp = tempfile::NamedTempFile::new_in(&self.folder).map_err(|e| e.to_string())?;
        tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        tmp.persist(self.folder.join(format!("{}.json", self.key)))
            .map_err(|e| e.error.to_string())?;
        drop(turn);
        Ok(())
    }
}

/// A chave do álbum: um código do caminho do arquivo, igual em toda abertura.
fn key_of(file: Option<&Path>) -> String {
    match file {
        Some(file) => {
            let path = file.to_string_lossy();
            let code = path.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
            format!("album-{code:016x}")
        }
        None => NO_FILE.to_owned(),
    }
}
