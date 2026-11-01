//! Cenários da spec `distribuicao` (DS) que vivem no editor: quando checar, a barra, salvar antes
//! de reiniciar, os canais, as preferências e a associação do `.lamina`. O instalador é de
//! mentira: guarda o que pediram e responde o que o teste mandar.

use std::ffi::OsString;
use std::path::Path;

use super::tl_common::{editor_at, three_spreads};
use crate::updates::association::{windows_entries, windows_keys_to_remove};
use crate::updates::{
    CHECK_FAILED, Channel, Job, NOT_INSTALLED, Outcome, Phase, Step, UPDATES_UNREADABLE,
    UpdateError, UpdatePrefs, Updater, Updates, UpdatesStore, Version, restart_args, run,
};
use crate::{Command, Leave};

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

/// O instalador de mentira.
#[derive(Default)]
struct Fake {
    stable: Option<Version>,
    beta: Option<Version>,
    offline: bool,
    asked: Vec<Channel>,
    downloaded: Vec<Version>,
    restarted: Vec<Vec<OsString>>,
    on_exit: usize,
}

impl Updater for Fake {
    fn latest(&mut self, channel: Channel) -> Result<Option<Version>, UpdateError> {
        self.asked.push(channel);
        if self.offline {
            return Err(UpdateError::Failed("sem rede".into()));
        }
        Ok(match channel {
            Channel::Stable => self.stable.clone(),
            Channel::Beta => self.beta.clone(),
        })
    }
    fn download(&mut self, _: Channel, version: &Version) -> Result<(), UpdateError> {
        self.downloaded.push(version.clone());
        Ok(())
    }
    fn apply_and_restart(&mut self, args: &[OsString]) -> Result<(), UpdateError> {
        self.restarted.push(args.to_vec());
        Ok(())
    }
    fn apply_on_exit(&mut self) -> Result<(), UpdateError> {
        self.on_exit += 1;
        Ok(())
    }
}

fn updates(prefs: UpdatePrefs, current: &str, installed: bool) -> Updates {
    Updates::new(UpdatesStore::in_memory(prefs), v(current), installed)
}

fn installed(current: &str) -> Updates {
    let prefs = UpdatePrefs {
        last_version: Some(current.to_owned()),
        ..UpdatePrefs::default()
    };
    updates(prefs, current, true)
}

/// Roda os trabalhos até parar, como a linha separada do app; devolve os avisos.
fn drive(updates: &mut Updates, fake: &mut Fake, mut step: Step) -> Vec<String> {
    let mut notices: Vec<String> = step.notice.take().into_iter().collect();
    while let Some(job) = step.job.take() {
        let outcome = run(fake, &job, &updates.current().clone());
        step = updates.finish(outcome);
        notices.extend(step.notice.take());
    }
    notices
}

/// Com a 0.2.0 baixada e a barra à mostra.
fn ready() -> (Updates, Fake) {
    let mut u = installed("0.1.0");
    let mut fake = Fake {
        stable: Some(v("0.2.0")),
        ..Fake::default()
    };
    let step = u.on_open();
    drive(&mut u, &mut fake, step);
    assert_eq!(u.bar().as_deref(), Some("Lamina 0.2.0 está pronta."));
    (u, fake)
}

#[test]
fn ds_b1_checa_uma_vez_ao_abrir() {
    let mut u = installed("0.1.0");
    let mut fake = Fake::default();
    let step = u.on_open();
    assert_eq!(
        step.job,
        Some(Job::Check {
            manual: false,
            beta: false
        })
    );
    drive(&mut u, &mut fake, step);
    assert_eq!(fake.asked, [Channel::Stable]);
    assert_eq!(
        u.on_open().job,
        None,
        "nenhuma outra checagem nesta abertura"
    );
}

#[test]
fn ds_b2_checagem_desligada() {
    let prefs = UpdatePrefs {
        automatic: false,
        last_version: Some("0.1.0".into()),
        ..UpdatePrefs::default()
    };
    let mut u = updates(prefs, "0.1.0", true);
    assert_eq!(u.on_open(), Step::default());
    let mut fake = Fake::default();
    let step = u.check_now();
    assert!(step.job.is_some(), "o menu ainda checa");
    let notices = drive(&mut u, &mut fake, step);
    assert_eq!(fake.asked, [Channel::Stable]);
    assert_eq!(notices, ["Você está com a versão mais nova (0.1.0)."]);
}

#[test]
fn ds_b3_sem_rede_sem_aviso() {
    let mut u = installed("0.1.0");
    let mut fake = Fake {
        offline: true,
        ..Fake::default()
    };
    let step = u.on_open();
    assert!(drive(&mut u, &mut fake, step).is_empty());
    assert_eq!(u.bar(), None);
    assert_eq!(*u.phase(), Phase::Idle);
    let step = u.check_now();
    assert_eq!(drive(&mut u, &mut fake, step), [CHECK_FAILED]);
}

#[test]
fn ds_b4_versao_nova_vira_barra_e_ganha_os_botoes_depois_de_baixada() {
    let mut u = installed("0.1.0");
    let mut fake = Fake {
        stable: Some(v("0.2.0")),
        ..Fake::default()
    };
    let job = u.on_open().job.unwrap();
    let step = u.finish(run(&mut fake, &job, &v("0.1.0")));
    assert_eq!(*u.phase(), Phase::Downloading(v("0.2.0")));
    assert_eq!(
        u.bar().as_deref(),
        Some("Lamina 0.2.0 encontrada. Baixando…")
    );
    assert!(!u.bar_ready(), "sem botões enquanto baixa");
    let download = step.job.unwrap();
    assert_eq!(
        download,
        Job::Download {
            channel: Channel::Stable,
            version: v("0.2.0")
        }
    );
    u.finish(run(&mut fake, &download, &v("0.1.0")));
    assert_eq!(fake.downloaded, [v("0.2.0")]);
    assert_eq!(u.bar().as_deref(), Some("Lamina 0.2.0 está pronta."));
    assert!(u.bar_ready());
}

#[test]
fn ds_b5_ja_esta_na_mais_nova() {
    let mut u = installed("0.1.0");
    let mut fake = Fake {
        stable: Some(v("0.1.0")),
        ..Fake::default()
    };
    let step = u.check_now();
    assert_eq!(
        drive(&mut u, &mut fake, step),
        ["Você está com a versão mais nova (0.1.0)."]
    );
    assert!(fake.downloaded.is_empty());
}

#[test]
fn ds_b6_pede_para_salvar_antes() {
    let (mut u, mut fake) = ready();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Casamento.lamina");
    let mut editor = editor_at(three_spreads(), 0);
    editor.session_mut().unwrap().save_as(&file).unwrap();
    assert!(editor.run(Command::NextLayout));
    assert!(
        editor.needs_confirmation(),
        "alterações não salvas: pergunta"
    );

    // Cancelar: não chama o instalador e a barra volta.
    assert!(u.begin_restart());
    assert!(!editor.leave(Leave::Cancel, None).unwrap());
    u.cancel_restart();
    assert!(fake.restarted.is_empty());
    assert!(u.bar().is_some());

    // Salvar: grava e reinicia com o caminho do álbum.
    assert!(u.begin_restart());
    assert!(editor.leave(Leave::Save, None).unwrap());
    assert!(!editor.needs_confirmation());
    fake.apply_and_restart(&restart_args(&editor)).unwrap();
    assert_eq!(fake.restarted, [vec![file.into_os_string()]]);
}

#[test]
fn ds_b7_reabre_o_mesmo_album() {
    let (mut u, mut fake) = ready();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Casamento.lamina");
    let mut editor = editor_at(three_spreads(), 0);
    editor.session_mut().unwrap().save_as(&file).unwrap();
    assert!(!editor.needs_confirmation());
    assert!(u.begin_restart());
    assert!(!u.begin_restart(), "o segundo clique é ignorado");
    fake.apply_and_restart(&restart_args(&editor)).unwrap();
    assert_eq!(fake.restarted, [vec![file.into_os_string()]]);
    // Um álbum que nunca foi salvo não reabre.
    let unsaved = editor_at(three_spreads(), 0);
    assert!(restart_args(&unsaved).is_empty());
}

#[test]
fn ds_b8_agora_nao_e_a_atualizacao_ao_fechar() {
    let (mut u, mut fake) = ready();
    u.dismiss();
    assert_eq!(u.bar(), None, "a barra some nesta abertura");
    assert!(u.applies_on_exit());
    fake.apply_on_exit().unwrap();
    assert_eq!(fake.on_exit, 1);
    // Sem versão baixada, fechar não pede nada.
    assert!(!installed("0.1.0").applies_on_exit());
}

/// A versão nova é lembrada uma vez, na abertura, e o aviso "Lamina atualizado para X." deu lugar à
/// janela de novidades (spec NV, NV-B3 e NV-B9).
#[test]
fn ds_b9_versao_nova_lembrada_uma_vez() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("atualizacoes.json");
    std::fs::write(&path, r#"{"automatico": false, "ultima_versao": "0.1.0"}"#).unwrap();
    let mut u = Updates::new(UpdatesStore::load(&path), v("0.2.0"), true);
    assert_eq!(u.on_start().news_since, Some(v("0.1.0")));
    assert_eq!(u.on_open().notice, None, "sem o aviso antigo");
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("\"ultima_versao\": \"0.2.0\""), "{saved}");
    let mut again = Updates::new(UpdatesStore::load(&path), v("0.2.0"), true);
    assert_eq!(again.on_start(), Step::default());
    // A primeira abertura (sem arquivo) não mostra nada.
    let mut first = updates(UpdatePrefs::default(), "0.1.0", true);
    assert_eq!(first.on_start().news_since, None);
    assert_eq!(first.on_open().notice, None);
}

#[test]
fn ds_b10_canal_beta() {
    let beta_on = UpdatePrefs {
        beta: true,
        last_version: Some("0.1.0".into()),
        ..UpdatePrefs::default()
    };
    let mut u = updates(beta_on, "0.1.0", true);
    let mut fake = Fake {
        stable: Some(v("0.2.0")),
        beta: Some(v("0.3.0-beta.1")),
        ..Fake::default()
    };
    let step = u.on_open();
    drive(&mut u, &mut fake, step);
    assert_eq!(fake.asked, [Channel::Stable, Channel::Beta]);
    assert_eq!(fake.downloaded, [v("0.3.0-beta.1")]);
    // A estável passou da beta: é ela que vem, mesmo com o beta ligado.
    let job = Job::Check {
        manual: true,
        beta: true,
    };
    let mut newer = Fake {
        stable: Some(v("0.3.0")),
        beta: Some(v("0.3.0-beta.2")),
        ..Fake::default()
    };
    let Outcome::Found {
        channel, version, ..
    } = run(&mut newer, &job, &v("0.2.0"))
    else {
        panic!("devia achar a 0.3.0");
    };
    assert_eq!((channel, version), (Channel::Stable, v("0.3.0")));
    // Beta desligado com a beta instalada: a estável menor não é oferecida.
    let mut off = updates(UpdatePrefs::default(), "0.3.0-beta.1", true);
    let mut fake = Fake {
        stable: Some(v("0.2.0")),
        beta: Some(v("0.3.0-beta.1")),
        ..Fake::default()
    };
    let step = off.check_now();
    let notices = drive(&mut off, &mut fake, step);
    assert!(fake.downloaded.is_empty());
    assert_eq!(fake.asked, [Channel::Stable], "sem o beta, só o estável");
    assert_eq!(
        notices,
        ["Você está com a versão mais nova (0.3.0-beta.1)."]
    );
}

#[test]
fn ds_b11_preferencias_ilegiveis() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("atualizacoes.json");
    std::fs::write(&path, "{ isto não é json").unwrap();
    let mut u = Updates::new(UpdatesStore::load(&path), v("0.2.0"), true);
    assert!(u.store.prefs.automatic && !u.store.prefs.beta, "o padrão");
    let step = u.on_open();
    assert_eq!(step.notice.as_deref(), Some(UPDATES_UNREADABLE));
    assert!(step.job.is_some(), "vale o padrão: checa");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "{ isto não é json",
        "não reescreve ao abrir"
    );
    u.toggle_beta().unwrap();
    let saved: UpdatePrefs =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(saved.beta && saved.automatic);
}

#[test]
fn ds_b12_nao_instalado() {
    let mut u = updates(UpdatePrefs::default(), "0.1.0", false);
    assert_eq!(u.on_open().job, None);
    assert_eq!(
        u.check_now(),
        Step {
            notice: Some(NOT_INSTALLED.to_owned()),
            ..Step::default()
        }
    );
}

#[test]
fn ds_b13_associacao_do_lamina() {
    let exe = Path::new(r"C:\Users\ana\AppData\Local\Lamina\current\lamina-app.exe");
    let entries = windows_entries(exe);
    let get = |key: &str| {
        entries
            .iter()
            .find(|e| e.key == key && e.name.is_none())
            .map(|e| e.data.as_str())
    };
    assert_eq!(get(r"HKCU\Software\Classes\.lamina"), Some("Lamina.Album"));
    assert_eq!(
        get(r"HKCU\Software\Classes\Lamina.Album\DefaultIcon"),
        Some(r#""C:\Users\ana\AppData\Local\Lamina\current\lamina-app.exe",0"#)
    );
    assert_eq!(
        get(r"HKCU\Software\Classes\Lamina.Album\shell\open\command"),
        Some(r#""C:\Users\ana\AppData\Local\Lamina\current\lamina-app.exe" "%1""#)
    );
    // Desinstalar apaga as mesmas chaves e nada fora delas.
    let removed = windows_keys_to_remove();
    assert_eq!(
        removed,
        [
            r"HKCU\Software\Classes\.lamina",
            r"HKCU\Software\Classes\Lamina.Album"
        ]
    );
    for entry in &entries {
        assert!(
            removed.iter().any(|k| entry.key.starts_with(k.as_str())),
            "{} fica de fora da desinstalação",
            entry.key
        );
    }
    assert!(
        entries
            .iter()
            .all(|e| e.key.starts_with(r"HKCU\Software\Classes\"))
    );
}

/// Auditoria da Fase 4, achado 8: uma versão de teste instalada recebe as próximas versões de
/// teste sem o fotógrafo ir ao menu; a escolha dele no menu vale sempre.
#[test]
fn versao_de_teste_instalada_liga_as_versoes_de_teste() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("atualizacoes.json");
    std::fs::write(
        &path,
        r#"{"automatico": false, "ultima_versao": "0.1.0-beta.3"}"#,
    )
    .unwrap();
    let mut beta = Updates::new(UpdatesStore::load(&path), v("0.1.0-beta.4"), true);
    assert_eq!(beta.on_start().news_since, Some(v("0.1.0-beta.3")));
    assert_eq!(
        beta.on_open().notice,
        None,
        "a janela de novidades substitui o aviso (NV-B9)"
    );
    assert!(UpdatesStore::load(&path).prefs.beta);
    // Desligado no menu: fica desligado nas próximas aberturas.
    beta.toggle_beta().unwrap();
    let mut again = Updates::new(UpdatesStore::load(&path), v("0.1.0-beta.4"), true);
    again.on_open();
    assert!(!UpdatesStore::load(&path).prefs.beta);
    // Uma versão estável e o Lamina compilado pela equipe não ligam nada.
    let mut stable = updates(UpdatePrefs::default(), "0.2.0", true);
    stable.on_open();
    assert!(!stable.store.prefs.beta);
    let mut dev = updates(UpdatePrefs::default(), "0.1.0-beta.4", false);
    dev.on_open();
    assert!(!dev.store.prefs.beta);
}
