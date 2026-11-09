//! O arquivo de preferências de layout do fotógrafo (spec TL, regra 7): favoritos e rejeitados,
//! na pasta de configuração do sistema, valendo para todos os projetos.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use lamina_layout::Preferences;
use serde::{Deserialize, Serialize};

/// Versão do formato do arquivo.
const VERSION: u32 = 1;
/// Nome do arquivo dentro da pasta de configuração.
pub const PREFS_FILE: &str = "preferencias-de-layout.json";

#[derive(Debug, Serialize, Deserialize)]
struct OnDisk {
    versao: u32,
    favoritos: Vec<String>,
    rejeitados: Vec<String>,
    /// Modelos de fábrica ocultos (spec RM, beta 9.1).
    #[serde(default)]
    ocultos: Vec<String>,
}

/// As preferências e onde elas moram. Sem arquivo (`path` = `None`), vivem só na memória.
#[derive(Debug, Default)]
pub struct PrefsStore {
    path: Option<PathBuf>,
    pub prefs: Preferences,
    /// Aviso de leitura para a janela mostrar uma vez.
    pub warning: Option<String>,
}

/// Pasta de configuração do Lamina no sistema.
pub fn config_dir() -> PathBuf {
    let env = |name| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(target_os = "windows") {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Application Support"))
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config")))
    };
    base.unwrap_or_else(std::env::temp_dir).join("Lamina")
}

impl PrefsStore {
    /// Lê o arquivo. Sem arquivo, começa vazio; ilegível, começa vazio com aviso e não o
    /// sobrescreve até a próxima marcação (TL-B11).
    pub fn load(path: &Path) -> Self {
        let mut store = PrefsStore {
            path: Some(path.to_path_buf()),
            ..PrefsStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match parse(&text) {
            Ok(prefs) => store.prefs = prefs,
            Err(reason) => {
                store.warning = Some(format!(
                    "Não foi possível ler as preferências de layout: {reason}."
                ));
            }
        }
        store
    }

    /// Grava com segurança (temporário + renomear). `Err` com a mensagem da seção 6.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write(path, &self.prefs).map_err(|reason| {
            format!("Não foi possível gravar as preferências de layout: {reason}.")
        })
    }
}

fn parse(text: &str) -> Result<Preferences, String> {
    let disk: OnDisk = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if disk.versao > VERSION {
        return Err(format!(
            "versão {} do arquivo; este programa entende até a {VERSION}",
            disk.versao
        ));
    }
    Ok(Preferences {
        favorites: disk.favoritos.into_iter().collect(),
        rejected: disk.rejeitados.into_iter().collect(),
        hidden: disk.ocultos.into_iter().collect(),
    })
}

fn write(path: &Path, prefs: &Preferences) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let disk = OnDisk {
        versao: VERSION,
        favoritos: prefs.favorites.iter().cloned().collect(),
        rejeitados: prefs.rejected.iter().cloned().collect(),
        ocultos: prefs.hidden.iter().cloned().collect(),
    };
    let json = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
