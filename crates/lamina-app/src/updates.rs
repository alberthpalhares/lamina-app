//! Atualização automática (spec DS; ADR 0009): o [`Updater`] do editor com o Velopack, os
//! trabalhos numa linha separada, a barra e os itens do menu Ajuda. As decisões são do
//! `lamina_editor::updates`; aqui só a tradução.

use std::cell::RefCell;
use std::ffi::OsString;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lamina_editor::updates::{
    Channel, Job, Outcome, Step, UpdateError, Updater, Version, restart_args, run,
};
use lamina_editor::{HumanMessage, UPDATES_FILE, Updates, UpdatesStore, config_dir, guarded};
use slint::Timer;
use velopack::sources::{GithubSource, NoneSource};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions};

use crate::state::{App, Msg};

/// Onde as versões são publicadas (regra 11).
const REPO: &str = "https://github.com/alberthpalhares/lamina-app";

/// Espera depois de a janela aparecer, para a checagem não disputar com a abertura (regra 2).
pub const CHECK_DELAY: Duration = Duration::from_secs(4);

/// `LAMINA_ATUALIZACAO=<versão>`: simula a barra no Lamina compilado (spec AV, seção 3).
const SIMULATE: &str = "LAMINA_ATUALIZACAO";
const SIMULATED_DOWNLOAD: Duration = Duration::from_secs(5);

/// O canal do Velopack: um por sistema, porque a mesma release do GitHub leva os três.
fn channel_name(channel: Channel) -> String {
    let os = if cfg!(windows) {
        "win"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    match channel {
        Channel::Stable => os.to_owned(),
        Channel::Beta => format!("{os}-beta"),
    }
}

/// A versão instalada, ou `None` fora de uma instalação do Velopack (`cargo run`).
pub fn installed_version() -> Option<Version> {
    let manager = UpdateManager::new(NoneSource {}, None, None).ok()?;
    Version::parse(&manager.get_current_version_as_string())
}

/// A versão que roda e se veio de um instalador (regra 8).
pub fn state() -> Updates {
    let store = UpdatesStore::load(&config_dir().join(UPDATES_FILE));
    match installed_version() {
        Some(version) => Updates::new(store, version, true),
        None => Updates::new(
            store,
            Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_default(),
            false,
        ),
    }
}

fn manager(channel: Channel) -> Result<UpdateManager, UpdateError> {
    let source = GithubSource::new(REPO, None, channel == Channel::Beta);
    let options = UpdateOptions {
        AllowVersionDowngrade: false,
        ExplicitChannel: Some(channel_name(channel)),
        MaximumDeltasBeforeFallback: 10,
    };
    UpdateManager::new(source, Some(options), None).map_err(failure)
}

fn failure(e: velopack::Error) -> UpdateError {
    match e {
        velopack::Error::NotInstalled(_) => UpdateError::NotInstalled,
        other => UpdateError::Failed(other.to_string()),
    }
}

/// O Velopack atrás da interface do editor. Guarda o que achou e o que baixou entre um
/// trabalho e outro.
#[derive(Debug, Default)]
pub struct Velopack {
    found: Vec<(Channel, UpdateInfo)>,
    ready: Option<(Channel, UpdateInfo)>,
}

impl Velopack {
    fn ready(&self) -> Result<(UpdateManager, &UpdateInfo), UpdateError> {
        let (channel, info) = self
            .ready
            .as_ref()
            .ok_or_else(|| UpdateError::Failed("nada baixado".into()))?;
        Ok((manager(*channel)?, info))
    }
}

impl Updater for Velopack {
    fn latest(&mut self, channel: Channel) -> Result<Option<Version>, UpdateError> {
        self.found.retain(|(c, _)| *c != channel);
        match manager(channel)?.check_for_updates().map_err(failure)? {
            UpdateCheck::UpdateAvailable(info) => {
                let version = Version::parse(&info.TargetFullRelease.Version);
                self.found.push((channel, *info));
                Ok(version)
            }
            UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => Ok(None),
        }
    }

    fn download(&mut self, channel: Channel, version: &Version) -> Result<(), UpdateError> {
        let index = self
            .found
            .iter()
            .position(|(c, info)| {
                *c == channel
                    && Version::parse(&info.TargetFullRelease.Version).as_ref() == Some(version)
            })
            .ok_or_else(|| UpdateError::Failed("versão não achada".into()))?;
        let (_, info) = self.found.remove(index);
        // O Velopack confere o SHA do pacote baixado e descarta o que não confere (seção 6).
        manager(channel)?
            .download_updates(&info, None)
            .map_err(failure)?;
        self.ready = Some((channel, info));
        Ok(())
    }

    fn apply_and_restart(&mut self, args: &[OsString]) -> Result<(), UpdateError> {
        let (manager, info) = self.ready()?;
        // Não volta: o Velopack fecha o Lamina, aplica e reabre com `args`.
        manager
            .apply_updates_and_restart_with_args(info, args)
            .map_err(failure)
    }

    fn apply_on_exit(&mut self) -> Result<(), UpdateError> {
        let (manager, info) = self.ready()?;
        manager
            .wait_exit_then_apply_updates(info, true, false, Vec::<OsString>::new())
            .map_err(failure)
    }
}

pub type Shared = Arc<Mutex<Velopack>>;

/// Roda o trabalho numa linha separada; a resposta volta pelo canal da janela.
fn spawn(app: &App, job: Job) {
    let (updater, tx) = (app.updater.clone(), app.tx.clone());
    let current = app.updates.current().clone();
    std::thread::spawn(move || {
        // Toda resposta volta, mesmo de uma falha inesperada: sem ela, a barra ficaria presa em
        // "Baixando…" (spec AV, regra 4). A janela pode ter fechado no meio: aí não há quem receba.
        let outcome = match updater.lock() {
            Ok(mut updater) => guarded(|| run(&mut *updater, &job, &current))
                .unwrap_or_else(|problem| job.failed(&problem.details)),
            Err(_) => job.failed("instalador ocupado"),
        };
        let _ = tx.send(Msg::Update(outcome));
    });
}

/// Aplica um passo do editor: o aviso na barra de avisos, a falha no registro e o próximo trabalho.
pub fn apply(app: &mut App, step: Step) {
    if let Some(notice) = step.notice {
        app.notice = notice;
    }
    if let (Some(line), Some(log)) = (step.log, app.editor.log()) {
        log.note(&line);
    }
    if let Some(job) = step.job {
        spawn(app, job);
    }
    show(app);
}

/// A barra de atualização e as marcas do menu Ajuda.
pub fn show(app: &App) {
    if let Some(w) = app.window.upgrade() {
        w.set_update_bar(app.updates.bar().unwrap_or_default().into());
        w.set_update_ready(app.updates.bar_ready());
        w.set_auto_updates(app.updates.store.prefs.automatic);
        w.set_beta_updates(app.updates.store.prefs.beta);
        w.set_notice(app.notice.clone().into());
        w.set_version(app.updates.current().to_string().into());
    }
}

/// A resposta de um trabalho da linha separada.
pub fn message(app: &mut App, outcome: Outcome) {
    let step = app.updates.finish(outcome);
    apply(app, step);
}

/// Ao abrir a janela (depois da espera): avisos e a checagem automática.
pub fn on_open(app: &mut App) {
    let step = app.updates.on_open();
    apply(app, step);
}

/// Os três itens do menu Ajuda.
pub fn check_now(app: &mut App) {
    let step = app.updates.check_now();
    apply(app, step);
}

pub fn toggle_automatic(app: &mut App) {
    if let Err(e) = app.updates.toggle_automatic() {
        app.notice = e;
    }
    show(app);
}

pub fn toggle_beta(app: &mut App) {
    if let Err(e) = app.updates.toggle_beta() {
        app.notice = e;
    }
    show(app);
}

/// `LAMINA_ATUALIZACAO=<versão>` (spec AV, seção 3): a barra de "baixando" e, um tempo depois, a de
/// "pronta", sem rede, para conferir no Lamina compilado.
pub fn simulate(shared: &Rc<RefCell<App>>) {
    let Ok(wanted) = std::env::var(SIMULATE) else {
        return;
    };
    let mut app = shared.borrow_mut();
    let Some(version) = app.updates.simulate_update(&wanted) else {
        return;
    };
    show(&app);
    let shared = shared.clone();
    Timer::single_shot(SIMULATED_DOWNLOAD, move || {
        message(&mut shared.borrow_mut(), Outcome::Downloaded(version));
    });
}

/// "Agora não" (regra 4).
pub fn dismiss(app: &mut App) {
    app.updates.dismiss();
    show(app);
}

/// "Reiniciar e atualizar", depois da pergunta de salvar (regra 5). Só volta se falhar.
pub fn restart(app: &mut App) {
    let args = restart_args(&app.editor);
    let result = match app.updater.lock() {
        Ok(mut updater) => updater.apply_and_restart(&args),
        Err(_) => Err(UpdateError::Failed("instalador ocupado".into())),
    };
    if let Err(error) = result {
        app.updates.cancel_restart();
        let message = HumanMessage::new(
            "Não foi possível atualizar agora.",
            "A versão nova entra quando o Lamina for fechado.",
            error.detail(),
        );
        crate::files::show_error(app, &message);
    }
    show(app);
}

/// O reinício não seguiu (Cancelar ou erro ao salvar): a barra volta.
pub fn restart_cancelled(app: &mut App) {
    app.updates.cancel_restart();
    show(app);
}

/// Ao fechar com uma versão baixada: ela entra depois que o Lamina sair (regra 6).
pub fn on_exit(app: &App) {
    if !app.updates.applies_on_exit() {
        return;
    }
    if let Ok(mut updater) = app.updater.lock() {
        // Fechando: se não der, o Velopack aplica na próxima abertura (o pacote já está baixado).
        let _ = updater.apply_on_exit();
    }
}
