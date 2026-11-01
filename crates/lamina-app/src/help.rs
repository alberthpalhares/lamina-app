//! A ajuda de sugestões e registros (spec SR): o formulário no navegador, a pasta de registros no
//! explorador e o registro de erros desde a abertura. As decisões ficam no `lamina-editor`.

use std::path::Path;

use lamina_editor::error_log::{LOG_FOLDER, install_panic_hook};
use lamina_editor::feedback::{
    browser_failed, channel_name, folder_failed, logs_failed, system_name,
};
use lamina_editor::updates::news::video_address;
use lamina_editor::{ErrorLog, config_dir, feedback_address};

use crate::files::show_error;
use crate::state::App;

/// O registro na pasta de configuração, com o gancho das falhas inesperadas (regras 5 e 8).
pub fn error_log() -> ErrorLog {
    let log = ErrorLog::new(config_dir().join(LOG_FOLDER));
    install_panic_hook(log.clone());
    log
}

/// A linha "início" de cada abertura, com a versão e o canal já lidos (regra 5).
pub fn start(app: &App) {
    if let Some(log) = app.editor.log() {
        app.updates.log_start(log, &system_name());
    }
}

fn channel(app: &App) -> &'static str {
    channel_name(app.updates.store.prefs.beta)
}

/// Ajuda → "Enviar sugestão ou problema…" (regras 3 e 4).
pub fn send_feedback(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let version = app.updates.current().to_string();
    let address = feedback_address(&version, &system_name(), channel(app));
    if !w.invoke_open_url(address.clone().into()) {
        show_error(app, &browser_failed(&address));
    }
}

/// A capa ou o "Assistir ao vídeo" de uma novidade (spec PV, regra 8): o editor confere o endereço
/// antes de ele chegar ao navegador; sem internet, o erro aparece lá (seção 6).
pub fn open_video(app: &App, address: &str) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let Some(address) = video_address(address) else {
        return;
    };
    if !w.invoke_open_url(address.into()) {
        show_error(app, &browser_failed(address));
    }
}

/// Ajuda → "Abrir pasta de registros" (regra 10).
pub fn open_logs(app: &App) {
    let Some(log) = app.editor.log() else {
        return;
    };
    let opened = log
        .ready_folder()
        .and_then(|folder| open_in_explorer(&folder));
    if let Err(problem) = opened {
        show_error(app, &logs_failed(log.folder(), &problem));
    }
}

/// "Abrir a pasta" do resumo da exportação (spec EI).
pub fn open_folder(app: &App, folder: &Path) {
    if let Err(problem) = open_in_explorer(folder) {
        show_error(app, &folder_failed(folder, &problem));
    }
}

/// O gerenciador de arquivos do sistema na pasta.
fn open_in_explorer(folder: &Path) -> std::io::Result<()> {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(folder)
        .spawn()
        .map(|_| ())
}
