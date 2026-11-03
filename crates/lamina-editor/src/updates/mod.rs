//! Atualização automática (spec DS; ADR 0009): quando checar, o que mostrar e o que pedir ao
//! instalador. A rede e o instalador ficam atrás de [`Updater`], que o `lamina-app` implementa
//! com o Velopack; aqui tudo roda sem rede, com um `Updater` de mentira nos testes.
//!
//! O ciclo: ao abrir, [`Updates::on_open`] pede uma checagem ([`Job::Check`]); o app a roda
//! numa linha separada com [`run`] e devolve o [`Outcome`] em [`Updates::finish`], que pede o
//! download ([`Job::Download`]). A barra ([`Updates::bar`]) aparece desde a versão achada e ganha
//! os botões com tudo baixado (spec AV, regra 3).

pub mod association;
mod job;
pub mod news;
mod store;
mod version;

use std::ffi::OsString;

pub use job::{Channel, Job, Outcome, UpdateError, Updater, run};
pub use store::{UPDATES_FILE, UPDATES_UNREADABLE, UpdatePrefs, UpdatesStore};
pub use version::Version;

use crate::feedback::channel_name;
use crate::{Editor, ErrorLog};

/// Os avisos da spec (seção 4).
pub const UP_TO_DATE: &str = "Você está com a versão mais nova";
pub const CHECK_FAILED: &str =
    "Não foi possível verificar agora. Confira a internet e tente de novo.";
pub const NOT_INSTALLED: &str = "As atualizações automáticas funcionam no Lamina instalado.";

/// O aviso de quando a barra some porque o download falhou (spec AV, regra 4).
pub fn download_failed(version: &Version) -> String {
    format!(
        "Não foi possível baixar o Lamina {version} agora. Tente de novo em Ajuda → Verificar \
         atualizações agora."
    )
}

/// Em que ponto está a atualização desta abertura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    /// Achada e baixando: a barra aparece, sem botões (spec AV, regra 3).
    Downloading(Version),
    /// Baixada e conferida: a barra ganha os botões.
    Ready(Version),
    /// Baixada, mas "Agora não": entra ao fechar.
    Dismissed(Version),
    /// "Reiniciar e atualizar" já pedido.
    Applying(Version),
}

/// O que o app faz depois de uma resposta: um aviso, o próximo trabalho, na abertura a versão
/// anterior do fotógrafo quando há novidades a mostrar (spec NV), e a linha do registro de uma
/// falha (spec AV, regra 5).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Step {
    pub notice: Option<String>,
    pub job: Option<Job>,
    pub news_since: Option<Version>,
    pub log: Option<String>,
}

/// A atualização da abertura atual e as preferências.
#[derive(Debug)]
pub struct Updates {
    pub store: UpdatesStore,
    current: Version,
    installed: bool,
    phase: Phase,
    /// A checagem automática já rodou nesta abertura (regra 2).
    checked: bool,
}

impl Updates {
    /// `current` é a versão que está rodando; `installed`, se veio de um instalador.
    pub fn new(store: UpdatesStore, current: Version, installed: bool) -> Self {
        Updates {
            store,
            current,
            installed,
            phase: Phase::Idle,
            checked: false,
        }
    }

    pub fn current(&self) -> &Version {
        &self.current
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Na abertura, antes de a janela aparecer (spec NV, regra 3): o Lamina instalado lembra a
    /// versão que roda e devolve a anterior em `news_since` quando ela é menor. Grava antes de a
    /// janela de novidades aparecer, então ela abre uma vez só. O Lamina compilado pela equipe não
    /// grava nada, para não esconder as novidades da versão instalada na mesma máquina.
    pub fn on_start(&mut self) -> Step {
        let current = self.current.to_string();
        let last = self.store.prefs.last_version.clone();
        if !self.installed || last.as_deref() == Some(current.as_str()) {
            return Step::default();
        }
        let news_since = last
            .as_deref()
            .and_then(Version::parse)
            .filter(|l| *l < self.current);
        Step {
            notice: self.store.remember_version(&current).err(),
            news_since,
            ..Step::default()
        }
    }

    /// `LAMINA_NOVIDADES=<versão>` (spec NV, seção 3): a janela como se o Lamina viesse de
    /// `previous`, sem gravar nada, para a equipe conferir no Lamina compilado.
    pub fn simulate_start(&self, previous: &str) -> Step {
        Step {
            news_since: Version::parse(previous),
            ..Step::default()
        }
    }

    /// Ao abrir a janela: o aviso de preferências ilegíveis (DS-B11), as versões de teste numa
    /// versão de teste instalada e a checagem automática, uma vez só (DS-B1, B2 e B12). A versão
    /// nova é lembrada antes, em [`Updates::on_start`] (spec NV, regra 7).
    pub fn on_open(&mut self) -> Step {
        let mut notices = Vec::new();
        if let Some(warning) = self.store.warning.take() {
            notices.push(warning);
        }
        // Uma versão de teste instalada recebe as próximas versões de teste, até o fotógrafo
        // escolher no menu (auditoria da Fase 4, achado 8).
        let prefs = &self.store.prefs;
        if self.installed
            && self.current.is_prerelease()
            && !prefs.beta
            && !prefs.beta_chosen
            && let Err(e) = self.store.beta_by_default()
        {
            notices.push(e);
        }
        let job = if self.installed && self.store.prefs.automatic && !self.checked {
            self.checked = true;
            self.start_check(false)
        } else {
            None
        };
        Step {
            notice: (!notices.is_empty()).then(|| notices.join(" ")),
            job,
            ..Step::default()
        }
    }

    /// "Verificar atualizações agora" (regra 2): checa mesmo com a preferência desligada.
    pub fn check_now(&mut self) -> Step {
        if !self.installed {
            return Step {
                notice: Some(NOT_INSTALLED.to_owned()),
                ..Step::default()
            };
        }
        if let Phase::Dismissed(v) = &self.phase {
            // Já baixada: a barra volta.
            self.phase = Phase::Ready(v.clone());
            return Step::default();
        }
        Step {
            job: self.start_check(true),
            ..Step::default()
        }
    }

    fn start_check(&mut self, manual: bool) -> Option<Job> {
        if self.phase != Phase::Idle {
            return None;
        }
        self.phase = Phase::Checking;
        Some(Job::Check {
            manual,
            beta: self.store.prefs.beta,
        })
    }

    /// A resposta de um trabalho (regras 3 e 4).
    pub fn finish(&mut self, outcome: Outcome) -> Step {
        match outcome {
            Outcome::Found {
                channel, version, ..
            } => {
                self.phase = Phase::Downloading(version.clone());
                Step {
                    job: Some(Job::Download { channel, version }),
                    ..Step::default()
                }
            }
            Outcome::UpToDate { manual } => {
                self.phase = Phase::Idle;
                Step {
                    notice: manual.then(|| format!("{UP_TO_DATE} ({}).", self.current)),
                    ..Step::default()
                }
            }
            Outcome::CheckFailed { manual, error } => {
                self.phase = Phase::Idle;
                let notice = match error {
                    _ if !manual => None,
                    UpdateError::NotInstalled => Some(NOT_INSTALLED.to_owned()),
                    UpdateError::Failed(_) => Some(CHECK_FAILED.to_owned()),
                };
                // Não instalado não é falha: é o Lamina compilado (AV-B6).
                let log = matches!(error, UpdateError::Failed(_))
                    .then(|| format!("atualização: a checagem falhou: {}", error.detail()));
                Step {
                    notice,
                    log,
                    ..Step::default()
                }
            }
            Outcome::Downloaded(version) => {
                self.phase = Phase::Ready(version);
                Step::default()
            }
            Outcome::DownloadFailed { version, error } => {
                self.phase = Phase::Idle;
                Step {
                    notice: Some(download_failed(&version)),
                    log: Some(format!(
                        "atualização: o download da {version} falhou: {}",
                        error.detail()
                    )),
                    ..Step::default()
                }
            }
        }
    }

    /// O texto da barra: desde a versão achada, baixando ou pronta (spec AV, regra 3).
    pub fn bar(&self) -> Option<String> {
        match &self.phase {
            Phase::Downloading(v) => Some(format!("Lamina {v} encontrada. Baixando…")),
            Phase::Ready(v) => Some(format!("Lamina {v} está pronta.")),
            _ => None,
        }
    }

    /// A barra já tem os botões: a versão está toda baixada (AV-B4).
    pub fn bar_ready(&self) -> bool {
        matches!(self.phase, Phase::Ready(_))
    }

    /// A linha "início" do registro, com qual Lamina abriu (spec SR, regra 5; AV-B7).
    pub fn log_start(&self, log: &ErrorLog, system: &str) {
        let origin = if self.installed {
            "instalado"
        } else {
            "compilado"
        };
        let channel = channel_name(self.store.prefs.beta);
        log.start(
            &self.current.to_string(),
            system,
            &format!("{channel} · {origin}"),
        );
    }

    /// `LAMINA_ATUALIZACAO=<versão>` (spec AV, seção 3): a barra de "baixando" no Lamina compilado,
    /// sem rede. Devolve a versão para o app avisar depois, com [`Outcome::Downloaded`], que ela
    /// ficou pronta. O instalado ignora, e uma versão inválida não mostra nada.
    pub fn simulate_update(&mut self, version: &str) -> Option<Version> {
        let version = Version::parse(version).filter(|_| !self.installed)?;
        self.phase = Phase::Downloading(version.clone());
        Some(version)
    }

    /// "Agora não": a barra some nesta abertura (regra 4).
    pub fn dismiss(&mut self) {
        if let Phase::Ready(v) = &self.phase {
            self.phase = Phase::Dismissed(v.clone());
        }
    }

    /// Começa o "Reiniciar e atualizar". `false` se não há versão pronta ou já começou (o
    /// segundo clique é ignorado).
    pub fn begin_restart(&mut self) -> bool {
        match &self.phase {
            Phase::Ready(v) | Phase::Dismissed(v) => {
                self.phase = Phase::Applying(v.clone());
                true
            }
            _ => false,
        }
    }

    /// Volta à barra quando o reinício não seguiu (Cancelar, erro ao salvar).
    pub fn cancel_restart(&mut self) {
        if let Phase::Applying(v) = &self.phase {
            self.phase = Phase::Ready(v.clone());
        }
    }

    /// Ao fechar com uma versão baixada e não aplicada, ela entra depois que o Lamina sair
    /// (regra 6, DS-B8).
    pub fn applies_on_exit(&self) -> bool {
        matches!(
            self.phase,
            Phase::Ready(_) | Phase::Dismissed(_) | Phase::Applying(_)
        )
    }

    /// Liga ou desliga a checagem ao abrir (menu Ajuda).
    pub fn toggle_automatic(&mut self) -> Result<(), String> {
        let on = !self.store.prefs.automatic;
        self.store.set_automatic(on)
    }

    /// Liga ou desliga as versões de teste (menu Ajuda).
    pub fn toggle_beta(&mut self) -> Result<(), String> {
        let on = !self.store.prefs.beta;
        self.store.set_beta(on)
    }
}

/// Os argumentos para reabrir depois de atualizar: o álbum aberto, se tem arquivo (regra 5).
pub fn restart_args(editor: &Editor) -> Vec<OsString> {
    editor
        .session()
        .and_then(|s| s.file())
        .map(|f| vec![f.as_os_str().to_owned()])
        .unwrap_or_default()
}
