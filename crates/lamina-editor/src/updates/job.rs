//! O trabalho que sai da linha da janela (spec DS, seção 4; spec AV, regra 4): a interface do
//! instalador, o que pedir a ele e a resposta. Quem decide o que fazer com a resposta é
//! [`super::Updates::finish`].

use std::ffi::OsString;

use super::Version;

/// De onde vêm as versões: as estáveis ou as de teste (regra 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

/// Por que o instalador não respondeu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    /// O Lamina não está numa instalação do Velopack (`cargo run`, regra 8).
    NotInstalled,
    /// Rede, servidor ou pacote que não confere: a mensagem fica só no registro.
    Failed(String),
}

impl UpdateError {
    /// O detalhe técnico, para o registro (spec AV, regra 5); nunca vai para a janela.
    pub fn detail(&self) -> &str {
        match self {
            UpdateError::NotInstalled => "o Lamina não está instalado",
            UpdateError::Failed(detail) => detail,
        }
    }
}

/// O instalador: o Velopack no app, um de mentira nos testes.
pub trait Updater: Send {
    /// A versão mais nova publicada no canal, se houver alguma.
    fn latest(&mut self, channel: Channel) -> Result<Option<Version>, UpdateError>;
    /// Baixa e confere a versão achada por [`Updater::latest`] no canal.
    fn download(&mut self, channel: Channel, version: &Version) -> Result<(), UpdateError>;
    /// Aplica o que foi baixado e reabre o Lamina com `args`. No app, não volta.
    fn apply_and_restart(&mut self, args: &[OsString]) -> Result<(), UpdateError>;
    /// Aplica o que foi baixado quando o Lamina sair (regra 6), sem reabrir.
    fn apply_on_exit(&mut self) -> Result<(), UpdateError>;
}

/// O trabalho que o app roda fora da linha da janela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Procura versão nova; `manual` = pedido pelo menu (regra 3).
    Check {
        manual: bool,
        beta: bool,
    },
    Download {
        channel: Channel,
        version: Version,
    },
}

impl Job {
    /// A resposta de um trabalho que nem chegou ao fim (a linha separada quebrou): sem ela, a barra
    /// ficaria presa em "Baixando…" (spec AV, regra 4).
    pub fn failed(&self, detail: &str) -> Outcome {
        let error = UpdateError::Failed(detail.to_owned());
        match self {
            Job::Check { manual, .. } => Outcome::CheckFailed {
                manual: *manual,
                error,
            },
            Job::Download { version, .. } => Outcome::DownloadFailed {
                version: version.clone(),
                error,
            },
        }
    }
}

/// A resposta de um [`Job`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Found {
        manual: bool,
        channel: Channel,
        version: Version,
    },
    UpToDate {
        manual: bool,
    },
    CheckFailed {
        manual: bool,
        error: UpdateError,
    },
    Downloaded(Version),
    DownloadFailed {
        version: Version,
        error: UpdateError,
    },
}

/// Roda um trabalho no instalador (na linha separada do app).
pub fn run(updater: &mut dyn Updater, job: &Job, current: &Version) -> Outcome {
    match job {
        Job::Check { manual, beta } => {
            let manual = *manual;
            let mut channels = vec![Channel::Stable];
            if *beta {
                channels.push(Channel::Beta);
            }
            let mut best: Option<(Channel, Version)> = None;
            for channel in channels {
                match updater.latest(channel) {
                    Ok(Some(v)) if v > *current && best.as_ref().is_none_or(|(_, b)| v > *b) => {
                        best = Some((channel, v));
                    }
                    Ok(_) => {}
                    Err(error) => return Outcome::CheckFailed { manual, error },
                }
            }
            match best {
                Some((channel, version)) => Outcome::Found {
                    manual,
                    channel,
                    version,
                },
                None => Outcome::UpToDate { manual },
            }
        }
        Job::Download { channel, version } => match updater.download(*channel, version) {
            Ok(()) => Outcome::Downloaded(version.clone()),
            Err(error) => Outcome::DownloadFailed {
                version: version.clone(),
                error,
            },
        },
    }
}
