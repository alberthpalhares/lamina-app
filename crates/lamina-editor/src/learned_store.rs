//! O arquivo do aprendizado de layouts (spec AL, regras 15 a 19): lê ao abrir, grava fora da linha
//! da janela e apaga depois de a fila acabar. Só a janela do Lamina usa: a IA e a linha de comando
//! não leem nem gravam (regra 17).

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};

use lamina_layout::Learned;
use serde::{Deserialize, Serialize};

use crate::ErrorLog;

/// Versão do formato do arquivo.
const VERSION: u32 = 1;
/// Nome do arquivo dentro da pasta de configuração.
pub const LEARNED_FILE: &str = "aprendizado-de-layouts.json";

#[derive(Serialize)]
struct OnDisk<'a> {
    versao: u32,
    pesos: &'a BTreeMap<String, u32>,
}

/// Só a versão: um arquivo mais novo pode ter os pesos em outro formato.
#[derive(Deserialize)]
struct Version {
    versao: u32,
}

/// Peso enorme num arquivo adulterado continua valendo como o teto do bônus (seção 6).
#[derive(Deserialize)]
struct Read {
    pesos: BTreeMap<String, u64>,
}

/// Um trabalho na fila de gravação (regra 19).
#[derive(Debug)]
pub enum LearnedJob {
    /// Grava estes pesos, com segurança (temporário e renomear).
    Save {
        path: PathBuf,
        weights: BTreeMap<String, u32>,
    },
    /// Marca na fila: avisa quando tudo o que veio antes terminou.
    Barrier(Sender<()>),
}

impl LearnedJob {
    /// Faz o trabalho. Quem chama está fora da linha da janela; `Err` diz por que não gravou, sem
    /// citar nenhum desenho (regra 17).
    pub fn run(self) -> Result<(), String> {
        match self {
            LearnedJob::Save { path, weights } => write(&path, &weights),
            LearnedJob::Barrier(done) => {
                // Quem esperava desistiu: a fila segue do mesmo jeito.
                let _ = done.send(());
                Ok(())
            }
        }
    }
}

/// O trabalhador da janela: faz os trabalhos em ordem, um de cada vez. Uma gravação que falha vira
/// uma linha no registro, sem aviso na janela (seção 6). Termina quando a fila é fechada.
pub fn start_learned_worker(log: Option<ErrorLog>) -> Sender<LearnedJob> {
    let (queue, jobs) = channel::<LearnedJob>();
    std::thread::spawn(move || {
        for job in jobs {
            if let (Err(reason), Some(log)) = (job.run(), &log) {
                log.note(&format!(
                    "não foi possível gravar o aprendizado de layouts: {reason}"
                ));
            }
        }
    });
    queue
}

/// O aprendizado e onde ele mora. Sem arquivo (`path` = `None`), como na IA e na linha de comando,
/// nada é contado.
#[derive(Debug, Default)]
pub struct LearnedStore {
    path: Option<PathBuf>,
    /// O arquivo é de uma versão mais nova: vale como vazio e nunca é sobrescrito (seção 6).
    frozen: bool,
    /// Não deu para ler: a janela só anota no registro (seção 6).
    unreadable: bool,
    queue: Option<Sender<LearnedJob>>,
    pub learned: Learned,
}

impl LearnedStore {
    /// Lê o arquivo. Sem arquivo ou ilegível, começa vazio.
    pub fn load(path: &Path) -> Self {
        let mut store = LearnedStore {
            path: Some(path.to_path_buf()),
            ..LearnedStore::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return store;
        };
        match serde_json::from_str::<Version>(&text) {
            Ok(file) if file.versao > VERSION => store.frozen = true,
            Ok(_) => match serde_json::from_str::<Read>(&text) {
                Ok(file) => store.learned = learned_from(file.pesos),
                Err(_) => store.unreadable = true,
            },
            Err(_) => store.unreadable = true,
        }
        store
    }

    /// Daqui em diante, os trabalhos vão para esta fila, em vez de rodar na hora.
    pub fn set_queue(&mut self, queue: Sender<LearnedJob>) {
        self.queue = Some(queue);
    }

    /// Esta janela aprende? (tem um arquivo para isso)
    pub(crate) fn can_learn(&self) -> bool {
        self.path.is_some()
    }

    /// Há algo para apagar? Um arquivo de versão mais nova também conta: ele continua no disco.
    pub(crate) fn has_anything(&self) -> bool {
        !self.learned.is_empty() || self.frozen
    }

    /// Não deu para ler o arquivo ao abrir?
    pub(crate) fn unreadable(&self) -> bool {
        self.unreadable
    }

    /// Entrega a gravação dos pesos de agora (regra 19). O editor termina sem esperar o disco.
    pub(crate) fn save(&self) {
        let Some(path) = self.path.as_ref().filter(|_| !self.frozen) else {
            return;
        };
        let job = LearnedJob::Save {
            path: path.clone(),
            weights: self.learned.weights().clone(),
        };
        self.deliver(job);
    }

    fn deliver(&self, job: LearnedJob) {
        let unsent = match &self.queue {
            Some(queue) => queue.send(job).err().map(|failed| failed.0),
            None => Some(job),
        };
        // Sem trabalhador, roda aqui: a falha seria só o aviso no registro.
        if let Some(job) = unsent {
            let _ = job.run();
        }
    }

    /// Espera a fila acabar (sair do Lamina e antes de apagar).
    pub(crate) fn flush(&self) {
        let (done, wait) = channel();
        self.deliver(LearnedJob::Barrier(done));
        let _ = wait.recv();
    }

    /// Apaga o arquivo e o que está na memória, depois do que estava na fila: uma gravação atrasada
    /// nunca recria o arquivo (regra 19). Se o arquivo ficar, nada muda.
    pub(crate) fn delete(&mut self) -> Result<(), String> {
        self.flush();
        if let Some(path) = &self.path {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        self.learned = Learned::default();
        self.frozen = false;
        Ok(())
    }
}

fn learned_from(weights: BTreeMap<String, u64>) -> Learned {
    Learned::from_weights(
        weights
            .into_iter()
            .map(|(key, weight)| (key, u32::try_from(weight).unwrap_or(u32::MAX))),
    )
}

fn write(path: &Path, weights: &BTreeMap<String, u32>) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let disk = OnDisk {
        versao: VERSION,
        pesos: weights,
    };
    let json = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    tmp.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
