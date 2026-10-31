//! A lista de recentes (spec TI): os álbuns abertos por último, com a lâmina em que o fotógrafo
//! parou, em `recentes.json` na pasta de configuração, e a miniatura de cada um em `recentes/`.
//! O mesmo padrão de `janela.json` (spec PN): sem arquivo começa vazia; ilegível começa vazia com
//! aviso e só é regravada na próxima mudança; gravação segura (temporário + renomear).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod list;
mod resume;
mod screen;
mod thumbs;

pub use resume::{
    RESUME_SIDE, Resume, card_label, card_tip, grid_cards, grid_columns, no_results, resume,
    resume_height,
};
pub use screen::{
    Card, Clock, MenuEntry, WelcomeButtons, cards, counts_text, recent_menu, search, show_search,
    welcome_buttons, when_text,
};
pub use thumbs::{
    MISSING_GRAY, THUMB_SIDE, ThumbCache, missing_files, read_thumbnail, spread_image,
    thumbnail_image, write_big_thumbnail, write_thumbnail,
};

/// Versão do formato do arquivo.
const VERSION: u32 = 1;
/// Arquivo da lista, na pasta de configuração.
pub const RECENTS_FILE: &str = "recentes.json";
/// Pasta das miniaturas, ao lado da lista.
pub const THUMBS_DIR: &str = "recentes";
/// O fim do nome da miniatura grande, para achar e apagar a do álbum anterior.
const BIG_THUMB_END: &str = "-grande.png";
/// Até 20 álbuns na lista e 8 fixados (regra 2).
pub const MAX_RECENTS: usize = 20;
pub const MAX_PINNED: usize = 8;
/// Mensagens da seção 6.
pub const RECENTS_UNREADABLE: &str =
    "Não foi possível ler a lista de álbuns recentes; ela começa vazia.";
pub const PINNED_FULL: &str = "Já há 8 álbuns fixados. Desafixe um deles para fixar este.";

/// Um álbum recente: só caminho, nome, contagens e datas (regra 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    #[serde(rename = "caminho")]
    pub path: PathBuf,
    #[serde(rename = "nome")]
    pub name: String,
    /// Segundos desde 1970 (UTC).
    #[serde(rename = "aberto_em")]
    pub opened_at: i64,
    #[serde(rename = "laminas")]
    pub spreads: usize,
    #[serde(rename = "fotos")]
    pub photos: usize,
    /// A lâmina em que parou, contada a partir de 0 (como `Session::current`).
    #[serde(rename = "lamina_atual")]
    pub current: usize,
    #[serde(rename = "fixado")]
    pub pinned: bool,
    /// A imagem do álbum tem dobra: a primeira lâmina tem duas páginas (spec DI, regra 12).
    #[serde(rename = "dobra", default = "with_fold")]
    pub fold: bool,
}

/// Quem veio de antes da spec DI: a tela só perde a sombra ao abrir o álbum de novo.
fn with_fold() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
struct OnDisk {
    versao: u32,
    albuns: Vec<Recent>,
}

/// A lista e onde ela mora. Sem arquivo (`path` = `None`), vive só na memória (testes, linha de
/// comando e controle por IA). A ordem da lista é a da tela: os fixados primeiro, na ordem em que
/// foram fixados; depois os outros, do aberto mais recentemente para o mais antigo.
#[derive(Debug, Default)]
pub struct RecentsStore {
    path: Option<PathBuf>,
    list: Vec<Recent>,
    /// Aviso de leitura para a janela mostrar uma vez.
    pub warning: Option<String>,
}

impl RecentsStore {
    /// Lê o arquivo. Sem arquivo, começa vazia; ilegível, vazia com aviso (TI-B7).
    pub fn load(path: &Path) -> Self {
        let mut store = RecentsStore {
            path: Some(path.to_path_buf()),
            ..RecentsStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match serde_json::from_str::<OnDisk>(&text) {
            Ok(disk) if disk.versao <= VERSION => {
                store.list = disk.albuns;
                store.tidy();
            }
            _ => store.warning = Some(RECENTS_UNREADABLE.to_owned()),
        }
        store
    }

    /// Os álbuns, na ordem da tela.
    pub fn list(&self) -> &[Recent] {
        &self.list
    }

    /// O álbum de `file`, se estiver na lista.
    pub fn find(&self, file: &Path) -> Option<&Recent> {
        let key = key(file);
        self.list.iter().find(|r| self::key(&r.path) == key)
    }

    /// Onde fica a miniatura de `file`: `<config>/recentes/<código do caminho>.png`. Nenhum
    /// lugar quando a lista vive só na memória.
    pub fn thumb_path(&self, file: &Path) -> Option<PathBuf> {
        let folder = self.path.as_ref()?.parent()?.join(THUMBS_DIR);
        Some(folder.join(format!("{:016x}.png", code(&key(file)))))
    }

    /// Onde fica a miniatura grande de `file` (spec DI, regra 14): só o álbum em andamento tem.
    pub fn big_thumb_path(&self, file: &Path) -> Option<PathBuf> {
        let folder = self.path.as_ref()?.parent()?.join(THUMBS_DIR);
        Some(folder.join(format!("{:016x}{BIG_THUMB_END}", code(&key(file)))))
    }

    /// Grava com segurança (temporário + renomear). `Err` com a mensagem da seção 6.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write(path, &self.list).map_err(|reason| {
            format!("Não foi possível guardar a lista de álbuns recentes: {reason}.")
        })
    }

    /// Apaga a miniatura de `file` (o álbum saiu da lista). Sem arquivo, nada a apagar.
    fn forget_thumb(&self, file: &Path) {
        for thumb in [self.thumb_path(file), self.big_thumb_path(file)]
            .into_iter()
            .flatten()
        {
            // A miniatura que não existe (nunca salvou) já está como deveria.
            let _ = std::fs::remove_file(thumb);
        }
    }
}

/// O caminho como a lista compara: absoluto, sem `.` e `..`, e no Windows sem diferença de
/// maiúsculas (regra 1).
pub fn key(file: &Path) -> String {
    let text = normalized(file).to_string_lossy().into_owned();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

/// O caminho que a lista guarda: absoluto e sem `.` e `..`, com as maiúsculas de quem abriu.
pub(crate) fn normalized(file: &Path) -> PathBuf {
    let full = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    let mut clean = PathBuf::new();
    for part in full.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other),
        }
    }
    clean
}

/// Código estável do caminho (FNV-1a de 64 bits), para o nome da miniatura.
fn code(key: &str) -> u64 {
    key.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Agora, em segundos desde 1970 (UTC).
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn write(path: &Path, list: &[Recent]) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let disk = OnDisk {
        versao: VERSION,
        albuns: list.to_vec(),
    };
    let json = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
