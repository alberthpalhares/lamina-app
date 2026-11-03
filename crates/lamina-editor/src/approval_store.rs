//! Os dados do fotógrafo na aprovação do cliente (spec AC, regra 22; spec AH, seção 4): a marca-d'água,
//! o WhatsApp e o e-mail, na pasta de configuração, valendo para todos os álbuns. O arquivo do álbum
//! não os guarda: são dele, não do álbum.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Versão do formato do arquivo. O WhatsApp e o e-mail entraram depois, opcionais, sem mudar a versão.
const VERSION: u32 = 1;
/// Nome do arquivo dentro da pasta de configuração.
pub const APPROVAL_FILE: &str = "aprovacao.json";
/// Sem jargão: o aviso vai para a caixa, e o que estava no arquivo não serve para o fotógrafo.
const UNREADABLE: &str =
    "Não foi possível ler os dados guardados da aprovação do cliente. Os campos começam vazios.";

#[derive(Debug, Serialize, Deserialize)]
struct OnDisk {
    versao: u32,
    marca_dagua_ligada: bool,
    marca_dagua_texto: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    whatsapp: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    email: String,
}

/// O que a caixa lembra de uma vez para outra, como foi digitado.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhotographerData {
    pub watermark_on: bool,
    pub watermark_text: String,
    pub whatsapp: String,
    pub email: String,
}

/// Os dados e onde eles moram. Sem arquivo (`path` = `None`), vivem só na memória.
#[derive(Debug, Default)]
pub struct ApprovalStore {
    path: Option<PathBuf>,
    pub data: PhotographerData,
    /// Aviso de leitura para a caixa mostrar uma vez.
    pub warning: Option<String>,
}

impl ApprovalStore {
    /// Lê o arquivo. Sem arquivo, começa vazio; ilegível ou de versão mais nova, começa vazio com
    /// aviso e não o sobrescreve até gerar com sucesso (AC-B31).
    pub fn load(path: &Path) -> Self {
        let mut store = ApprovalStore {
            path: Some(path.to_path_buf()),
            ..ApprovalStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match parse(&text) {
            Some(data) => store.data = data,
            None => store.warning = Some(UNREADABLE.to_owned()),
        }
        store
    }

    /// Grava com segurança (temporário + renomear), depois de gerar com sucesso.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write(path, &self.data)
            .map_err(|reason| format!("Não foi possível gravar os dados da aprovação: {reason}."))
    }
}

fn parse(text: &str) -> Option<PhotographerData> {
    let disk: OnDisk = serde_json::from_str(text).ok()?;
    (disk.versao <= VERSION).then_some(PhotographerData {
        watermark_on: disk.marca_dagua_ligada,
        watermark_text: disk.marca_dagua_texto,
        whatsapp: disk.whatsapp,
        email: disk.email,
    })
}

fn write(path: &Path, data: &PhotographerData) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let disk = OnDisk {
        versao: VERSION,
        marca_dagua_ligada: data.watermark_on,
        marca_dagua_texto: data.watermark_text.clone(),
        whatsapp: data.whatsapp.clone(),
        email: data.email.clone(),
    };
    let json = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
