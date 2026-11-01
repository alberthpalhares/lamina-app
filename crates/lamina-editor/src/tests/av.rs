//! Cenários da spec `aviso-de-atualizacao` (AV) que vivem no editor: a barra desde o achado, as
//! falhas no registro, a linha "início" e a simulação. O instalador é de mentira; o Velopack real só
//! roda num Lamina instalado, com um pacote publicado (seção 11).

use std::ffi::OsString;
use std::fs;

use crate::error_log::LOG_FILE;
use crate::updates::{
    CHECK_FAILED, Channel, Job, Outcome, Phase, Step, UpdateError, UpdatePrefs, Updater, Updates,
    UpdatesStore, Version, download_failed, run,
};
use crate::{Command, ErrorLog};

const DOWNLOADING: &str = "Lamina 0.2.0 encontrada. Baixando…";
const READY: &str = "Lamina 0.2.0 está pronta.";

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

/// O instalador de mentira: acha a 0.2.0, salvo se o teste mandar falhar.
#[derive(Default)]
struct Fake {
    offline: bool,
    not_installed: bool,
    broken_download: bool,
}

impl Updater for Fake {
    fn latest(&mut self, _: Channel) -> Result<Option<Version>, UpdateError> {
        if self.not_installed {
            return Err(UpdateError::NotInstalled);
        }
        if self.offline {
            return Err(UpdateError::Failed("sem rede".into()));
        }
        Ok(Some(v("0.2.0")))
    }
    fn download(&mut self, _: Channel, _: &Version) -> Result<(), UpdateError> {
        if self.broken_download {
            return Err(UpdateError::Failed("conexão caiu aos 40%".into()));
        }
        Ok(())
    }
    fn apply_and_restart(&mut self, _: &[OsString]) -> Result<(), UpdateError> {
        Ok(())
    }
    fn apply_on_exit(&mut self) -> Result<(), UpdateError> {
        Ok(())
    }
}

fn updates(automatic: bool, installed: bool) -> Updates {
    let prefs = UpdatePrefs {
        automatic,
        last_version: Some("0.1.0".into()),
        ..UpdatePrefs::default()
    };
    Updates::new(UpdatesStore::in_memory(prefs), v("0.1.0"), installed)
}

/// Roda a checagem pedida e devolve o passo com o download, sem rodá-lo.
fn found(u: &mut Updates, fake: &mut Fake, step: Step) -> Job {
    let job = step.job.unwrap();
    let outcome = run(fake, &job, &v("0.1.0"));
    u.finish(outcome).job.unwrap()
}

#[test]
fn av_b1_cada_abertura_checa_uma_vez() {
    for _ in 0..2 {
        let mut u = Updates::new(
            UpdatesStore::in_memory(UpdatePrefs::default()),
            v("0.1.0"),
            true,
        );
        assert_eq!(
            u.on_open().job,
            Some(Job::Check {
                manual: false,
                beta: false
            })
        );
        assert_eq!(u.on_open().job, None, "uma só por abertura");
    }
}

#[test]
fn av_b2_a_opcao_continua_no_menu_e_continua_valendo() {
    assert_eq!(
        Command::from_name("ajuda.atualizar-automaticamente"),
        Some(Command::ToggleAutoUpdates)
    );
    let mut u = updates(false, true);
    assert_eq!(u.on_open(), Step::default());
    assert_eq!(u.bar(), None);
    let mut fake = Fake::default();
    let step = u.check_now();
    found(&mut u, &mut fake, step);
    assert_eq!(u.bar().as_deref(), Some(DOWNLOADING));
    assert_eq!(u.toggle_automatic(), Ok(()));
    assert!(u.store.prefs.automatic, "a opção liga de novo");
}

#[test]
fn av_b3_a_barra_aparece_enquanto_baixa_sem_botoes() {
    for manual in [false, true] {
        let mut u = updates(true, true);
        let mut fake = Fake::default();
        let step = if manual { u.check_now() } else { u.on_open() };
        assert_eq!(*u.phase(), Phase::Checking);
        assert_eq!(u.bar(), None, "só checando, não há barra");
        found(&mut u, &mut fake, step);
        assert_eq!(u.bar().as_deref(), Some(DOWNLOADING));
        assert!(!u.bar_ready(), "sem botões enquanto baixa");
        assert!(!u.begin_restart(), "Reiniciar e atualizar não começa");
        u.dismiss();
        assert_eq!(*u.phase(), Phase::Downloading(v("0.2.0")));
        assert!(!u.applies_on_exit(), "nada baixado para entrar ao fechar");
    }
}

#[test]
fn av_b4_pronta_com_os_botoes() {
    let mut u = updates(true, true);
    let mut fake = Fake::default();
    let step = u.on_open();
    let download = found(&mut u, &mut fake, step);
    u.finish(run(&mut fake, &download, &v("0.1.0")));
    assert_eq!(u.bar().as_deref(), Some(READY));
    assert!(u.bar_ready());
}

#[test]
fn av_b5_download_que_falha_explica_por_que_a_barra_sumiu() {
    let notice = "Não foi possível baixar o Lamina 0.2.0 agora. Tente de novo em Ajuda → \
                  Verificar atualizações agora.";
    assert_eq!(download_failed(&v("0.2.0")), notice);
    for manual in [false, true] {
        let mut u = updates(true, true);
        let mut fake = Fake {
            broken_download: true,
            ..Fake::default()
        };
        let step = if manual { u.check_now() } else { u.on_open() };
        let download = found(&mut u, &mut fake, step);
        let step = u.finish(run(&mut fake, &download, &v("0.1.0")));
        assert_eq!(u.bar(), None);
        assert_eq!(*u.phase(), Phase::Idle);
        assert_eq!(step.notice.as_deref(), Some(notice));
        let line = step.log.unwrap();
        assert!(line.contains("0.2.0") && line.contains("conexão caiu aos 40%"));
    }
}

#[test]
fn av_b5_linha_separada_que_quebra_nao_deixa_a_barra_presa() {
    let mut u = updates(true, true);
    let mut fake = Fake::default();
    let step = u.on_open();
    let download = found(&mut u, &mut fake, step);
    let step = u.finish(download.failed("o trabalho quebrou"));
    assert_eq!(u.bar(), None);
    assert_eq!(step.notice, Some(download_failed(&v("0.2.0"))));
    assert!(step.log.unwrap().contains("o trabalho quebrou"));
    // A checagem que quebra, sem barra à mostra: só o registro, e o menu pode tentar de novo.
    let mut u = updates(true, true);
    let job = u.on_open().job.unwrap();
    let step = u.finish(job.failed("o trabalho quebrou"));
    assert_eq!(*u.phase(), Phase::Idle);
    assert_eq!(step.notice, None);
    assert!(step.log.unwrap().contains("o trabalho quebrou"));
}

#[test]
fn av_b6_falha_de_rede_vai_para_o_registro() {
    let mut fake = Fake {
        offline: true,
        ..Fake::default()
    };
    let mut u = updates(true, true);
    let job = u.on_open().job.unwrap();
    let step = u.finish(run(&mut fake, &job, &v("0.1.0")));
    assert_eq!(step.notice, None, "a checagem automática não incomoda");
    assert_eq!(u.bar(), None);
    assert!(step.log.unwrap().contains("sem rede"));
    // Pedida pelo menu: o aviso humano, e o erro técnico só no registro.
    let job = u.check_now().job.unwrap();
    let step = u.finish(run(&mut fake, &job, &v("0.1.0")));
    assert_eq!(step.notice.as_deref(), Some(CHECK_FAILED));
    assert!(step.log.unwrap().contains("sem rede"));
}

#[test]
fn av_b6_lamina_nao_instalado_nao_gera_linha() {
    let mut fake = Fake {
        not_installed: true,
        ..Fake::default()
    };
    let mut u = updates(true, true);
    let job = u.on_open().job.unwrap();
    let step = u.finish(run(&mut fake, &job, &v("0.1.0")));
    assert_eq!(step, Step::default());
}

#[test]
fn av_b7_a_linha_inicio_diz_qual_lamina_abriu() {
    for (installed, origin) in [(true, "instalado"), (false, "compilado")] {
        let dir = tempfile::tempdir().unwrap();
        let log = ErrorLog::with_home(dir.path().to_path_buf(), None);
        updates(true, installed).log_start(&log, "windows-x86_64");
        let text = fs::read_to_string(dir.path().join(LOG_FILE)).unwrap();
        let expected =
            format!("início · Lamina 0.1.0 · windows-x86_64 · canal estavel · {origin}\n");
        assert!(text.ends_with(&expected), "{text}");
    }
}

#[test]
fn av_b8_a_simulacao_mostra_os_dois_estados_sem_o_updater() {
    let mut u = updates(true, false);
    let version = u.simulate_update("0.2.0").unwrap();
    assert_eq!(u.bar().as_deref(), Some(DOWNLOADING));
    assert!(!u.bar_ready());
    assert_eq!(u.finish(Outcome::Downloaded(version)), Step::default());
    assert_eq!(u.bar().as_deref(), Some(READY));
    assert!(u.bar_ready());
}

#[test]
fn av_b8_o_instalado_ignora_a_simulacao_e_versao_invalida_nao_mostra_nada() {
    let mut u = updates(true, true);
    assert_eq!(u.simulate_update("0.2.0"), None);
    assert_eq!(u.bar(), None);
    let mut u = updates(true, false);
    assert_eq!(u.simulate_update("não é versão"), None);
    assert_eq!(u.bar(), None);
}
