//! Cenários AL-B9 (o que depende da gravação) e AL-B19 a AL-B27 da spec
//! `specs/aprendizado-de-layouts.md`: o arquivo, o apagar e a gravação fora da linha da janela.

use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_layout::learned::MAX_DESIGNS;
use lamina_layout::{Learned, MAX_LEARNED_BONUS};

use super::al_contagem::{choose, learning_at};
use super::tl_common::{H, V, built, spec};
use crate::backup::{BACKUP_FOLDER, Backups};
use crate::learned_store::{LearnedStore, start_learned_worker};
use crate::{Command, Editor, ErrorLog, LEARNED_FILE, Part};

fn album() -> lamina_core::Project {
    let photos: Vec<(u32, u32)> = (0..16).map(|i| if i % 4 == 3 { V } else { H }).collect();
    built(spec(30.0, 30.0), &photos, 4)
}

/// Editor que aprende em `file`, com o álbum aberto na lâmina 3.
fn editor_with(file: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.set_learned_store(LearnedStore::load(file));
    editor.adopt(album());
    assert!(editor.go_to_spread(2));
    editor
}

fn write_weights(file: &Path, weights: &[(String, u32)]) {
    let body: Vec<String> = weights
        .iter()
        .map(|(key, weight)| format!("{key:?}: {weight}"))
        .collect();
    let text = format!("{{\"versao\":1,\"pesos\":{{{}}}}}", body.join(","));
    std::fs::write(file, text).unwrap();
}

#[test]
fn al_b19_o_decaimento_e_feito_ao_gravar_sem_relogio() {
    let dir = tempfile::tempdir().unwrap();
    let run = |name: &str| {
        let file = dir.path().join(name);
        write_weights(&file, &[("antigo".to_owned(), 1000)]);
        let mut editor = editor_with(&file);
        let key = choose(&mut editor);
        assert!(editor.go_to_spread(3));
        assert_eq!(editor.learned().weight("antigo"), 997);
        assert_eq!(editor.learned().weight(&key), 1000);
        std::fs::read(&file).unwrap()
    };
    let first = run("a.json");
    assert_eq!(first, run("b.json"), "a mesma sequência, o mesmo arquivo");

    // Abrir e fechar o programa não muda nada.
    let file = dir.path().join("a.json");
    let mut editor = editor_with(&file);
    editor.remember_current();
    editor.flush_learned();
    assert_eq!(std::fs::read(&file).unwrap(), first);
}

#[test]
fn al_b9_o_arquivo_nao_tem_relogio() {
    let sources = [
        include_str!("../learned_store.rs"),
        include_str!("../editor/learning.rs"),
    ];
    for source in sources {
        for clock in ["SystemTime", "chrono", "Instant"] {
            assert!(!source.contains(clock), "`{clock}` no aprendizado");
        }
    }
}

#[test]
fn al_b20_o_arquivo_tem_limite() {
    let many = |n: usize, weight: u32| -> Vec<(String, u32)> {
        (0..n).map(|i| (format!("d{i:03}"), weight)).collect()
    };
    let mut learned = Learned::from_weights(many(MAX_DESIGNS - 1, 5000));
    learned = Learned::from_weights(
        learned
            .weights()
            .iter()
            .map(|(k, w)| (k.clone(), *w))
            .chain([("leve".to_owned(), 100)]),
    );
    learned.record("novo");
    assert_eq!(learned.weight("leve"), 0, "o peso baixo sai");
    assert_eq!(learned.weights().len(), MAX_DESIGNS);
    assert_eq!(learned.weight("novo"), 1000);

    // Tudo pesado: sai o primeiro na ordem da chave, e o novo fica.
    let mut full = Learned::from_weights(many(MAX_DESIGNS, 5000));
    full.record("novo");
    assert_eq!(full.weights().len(), MAX_DESIGNS);
    assert_eq!(full.weight("d000"), 0);
    assert_eq!(full.weight("novo"), 1000);
}

#[test]
fn al_b21_arquivo_ilegivel_ou_adulterado() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("quebrado.json");
    std::fs::write(&file, "isto não é json {").unwrap();
    let logs = dir.path().join("logs");
    let mut editor = Editor::new();
    editor.set_log(ErrorLog::new(logs.clone()));
    editor.set_learned_store(LearnedStore::load(&file));
    assert!(editor.learned().is_empty());
    assert_eq!(editor.take_notice(), None, "sem aviso na janela");
    let log = std::fs::read_to_string(logs.join(crate::error_log::LOG_FILE)).unwrap();
    assert!(log.contains("aprendizado de layouts"), "{log}");

    // A próxima escolha contada grava por cima.
    editor.adopt(album());
    let key = choose(&mut editor);
    assert!(editor.go_to_spread(3));
    let again = LearnedStore::load(&file);
    assert_eq!(again.learned.weight(&key), 1000);

    let odd = dir.path().join("adulterado.json");
    let text = r#"{"versao":1,"pesos":{"chave desconhecida":99999999999999}}"#;
    std::fs::write(&odd, text).unwrap();
    let store = LearnedStore::load(&odd);
    let bonus = store.learned.bonus("chave desconhecida");
    assert!(bonus > 0.0 && bonus < MAX_LEARNED_BONUS, "{bonus}");
}

#[test]
fn al_b22_versao_mais_nova_nao_e_sobrescrita() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("novo.json");
    let text = r#"{"versao":2,"pesos":[{"outro":"formato"}]}"#;
    std::fs::write(&file, text).unwrap();
    let mut editor = editor_with(&file);
    assert!(editor.learned().is_empty());
    let key = choose(&mut editor);
    assert!(editor.go_to_spread(3));
    assert_eq!(editor.learned().weight(&key), 1000, "vale na memória");
    editor.flush_learned();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
}

#[test]
fn al_b23_nao_conseguir_gravar_nao_atrapalha() {
    let dir = tempfile::tempdir().unwrap();
    // A pasta do arquivo é, na verdade, um arquivo.
    let blocker = dir.path().join("arquivo");
    std::fs::write(&blocker, "").unwrap();
    let mut store = LearnedStore::load(&blocker.join("aprendizado.json"));
    let (queue, jobs) = channel();
    store.set_queue(queue);
    let mut editor = Editor::new();
    editor.set_learned_store(store);
    editor.adopt(album());
    let key = choose(&mut editor);
    assert!(editor.go_to_spread(3));
    let job = jobs.try_recv().unwrap();
    let Err(reason) = job.run() else {
        panic!("a gravação devia falhar");
    };
    assert!(!reason.contains('|'), "sem chave de desenho: {reason}");
    assert_eq!(editor.take_notice(), None, "sem aviso na janela");
    assert_eq!(editor.learned().weight(&key), 1000, "vale até fechar");
    assert!(editor.go_to_spread(2), "a navegação segue");
}

#[test]
fn al_b24_apagar() {
    let (mut editor, dir) = learning_at(album(), 2);
    let file = dir.path().join("worker.json");
    let mut store = LearnedStore::load(&file);
    store.set_queue(start_learned_worker(None));
    editor.set_learned_store(store);

    assert!(editor.available(Command::DeleteLearned));
    assert!(editor.nothing_learned());
    let plain = editor.panel(Part::Spread).options;

    let key = choose(&mut editor);
    assert!(editor.go_to_spread(3));
    assert_eq!(editor.learned().weight(&key), 1000);
    assert!(!editor.nothing_learned());
    // "Cancelar" é a janela não rodar o comando: nada muda.
    assert_eq!(editor.learned().weight(&key), 1000);

    assert!(editor.mark_option(1, true));
    assert_eq!(editor.preferences().favorites.len(), 1);

    // Muitas gravações na fila e uma visita aberta: nada recria o arquivo.
    assert!(editor.go_to_spread(2));
    for _ in 0..5 {
        choose(&mut editor);
        assert!(editor.go_to_spread(3));
        assert!(editor.go_to_spread(2));
    }
    let open_key = choose(&mut editor);
    let album_before = editor.session().unwrap().album_shared();
    assert!(editor.run(Command::DeleteLearned));

    assert_eq!(
        editor.take_notice().as_deref(),
        Some("Aprendizado de layouts apagado.")
    );
    assert!(editor.learned().is_empty() && editor.nothing_learned());
    editor.flush_learned();
    assert!(!file.exists(), "o arquivo saiu e não voltou");
    assert_eq!(editor.preferences().favorites.len(), 1, "favoritos ficam");
    assert_eq!(*editor.session().unwrap().album(), *album_before);
    assert!(editor.go_to_spread(3));
    assert_eq!(editor.learned().weight(&open_key), 0, "a visita foi junto");
    assert!(editor.learned().is_empty());
    assert!(editor.go_to_spread(2));
    assert!(editor.run(Command::Undo));
    assert!(editor.learned().is_empty(), "o Ctrl+Z não traz de volta");

    // Sem o favorito, que vem primeiro, a ordem é a original.
    assert!(editor.mark_option(0, true));
    assert_eq!(editor.panel(Part::Spread).options, plain);
}

#[test]
fn al_b24_sempre_ativo_e_sem_nada_nao_pergunta() {
    let editor = Editor::new();
    assert!(editor.available(Command::DeleteLearned), "sem álbum");
    assert!(editor.nothing_learned());
    assert_eq!(crate::NOTHING_LEARNED, "Nada aprendido ainda.");
    assert_eq!(Command::DeleteLearned.name(), "layout.apagar-aprendizado");
    assert_eq!(
        Command::DeleteLearned.label(),
        "Apagar o aprendizado de layouts…"
    );
    let (mut on_cover, _dir) = learning_at(album(), 0);
    let measures = lamina_core::CoverMeasures::defaults(&on_cover.session().unwrap().album().spec);
    assert!(on_cover.create_cover(measures, None, &[]));
    assert!(on_cover.available(Command::DeleteLearned), "com a capa");
}

#[test]
fn al_b24_se_o_arquivo_nao_sai_vem_o_aviso_e_nada_muda() {
    let dir = tempfile::tempdir().unwrap();
    // Uma pasta no lugar do arquivo: `remove_file` recusa.
    let file = dir.path().join("pasta.json");
    std::fs::create_dir(&file).unwrap();
    let mut editor = Editor::new();
    editor.set_learned_store(LearnedStore::load(&file));
    editor.set_learned(Learned::from_weights([("desenho".to_owned(), 3000)]));
    assert!(editor.run(Command::DeleteLearned));
    let notice = editor.take_notice().unwrap();
    assert!(notice.starts_with("Não foi possível apagar"), "{notice}");
    assert_eq!(editor.learned().weight("desenho"), 3000);
}

#[test]
fn al_b5_rejeitar_apaga_o_peso_tambem_do_arquivo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("pesos.json");
    let mut editor = editor_with(&file);
    let key = choose(&mut editor);
    assert!(editor.run(Command::NextSpread));
    assert_eq!(LearnedStore::load(&file).learned.weight(&key), 1000);
    assert!(editor.run(Command::PreviousSpread));
    assert!(editor.run(Command::Reject));
    assert_eq!(LearnedStore::load(&file).learned.weight(&key), 0);
}

#[test]
fn al_b25_nada_sai_do_computador() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LEARNED_FILE);
    let logs = dir.path().join("logs");
    let mut editor = Editor::new();
    editor.set_log(ErrorLog::new(logs.clone()));
    editor.set_learned_store(LearnedStore::load(&file));
    editor.set_backups(Backups::new(dir.path().join(BACKUP_FOLDER)));
    let mut project = album();
    project.name = "Casamento da Ana".to_owned();
    editor.adopt(project);
    let album_file = dir.path().join("Casamento.lamina");
    let key = choose(&mut editor);
    assert!(editor.go_to_spread(3));
    editor.save_to(Some(&album_file)).unwrap();
    choose(&mut editor);
    let backup = editor.backup_step().unwrap();
    backup.write().unwrap();
    assert!(editor.go_to_spread(0));
    editor.flush_learned();

    let text = std::fs::read_to_string(&file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = json.as_object().unwrap();
    assert_eq!(object.len(), 2, "só versao e pesos");
    assert_eq!(object["versao"], 1);
    assert!(object["pesos"][&key].is_u64());
    for private in ["Casamento", "Ana", ".jpg", "\\\\", "/", "2026", "modelo:"] {
        assert!(!text.contains(private), "`{private}` no arquivo: {text}");
    }

    let mut others: Vec<PathBuf> = vec![album_file];
    others.extend(files_in(&dir.path().join(BACKUP_FOLDER)));
    others.extend(files_in(&logs));
    assert!(others.len() >= 2, "{others:?}");
    for other in others {
        let content = String::from_utf8_lossy(&std::fs::read(&other).unwrap()).into_owned();
        for learned in [key.as_str(), "pesos", LEARNED_FILE] {
            let around = content.find(learned).map(|at| {
                let start = at.saturating_sub(80);
                content.get(start..at + 80).unwrap_or_default().to_owned()
            });
            assert_eq!(around, None, "`{learned}` em {other:?}");
        }
    }
}

fn files_in(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .flat_map(|e| {
            let path = e.path();
            if path.is_dir() {
                files_in(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

#[test]
fn al_b26_a_ia_nao_le_o_arquivo() {
    // A IA abre um `Editor::new()`: sem arquivo, sem fila, sem peso. Quem carrega o arquivo é só o
    // `main` da janela; a parte da linha de comando está em `lamina-cli/tests/al.rs`.
    let editor = Editor::new();
    assert!(editor.learned().is_empty());
    let agent = [
        include_str!("../agent/mod.rs"),
        include_str!("../agent/layout.rs"),
        include_str!("../agent/album.rs"),
    ];
    for source in agent {
        assert!(!source.contains("LearnedStore") && !source.contains("LEARNED_FILE"));
        assert!(!source.contains("choose_option"), "a IA não conta");
        assert!(!source.contains("set_learned"), "a IA não aprende");
    }
}

/// Uma lâmina pronta para contar: escolhe uma opção nova e mede a saída, até caber em `goal`.
fn fastest_leave(editor: &mut Editor, goal: Duration) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        assert!(editor.go_to_spread(2));
        choose(editor);
        let start = Instant::now();
        assert!(editor.go_to_spread(3));
        best = best.min(start.elapsed());
        if best < goal {
            break;
        }
    }
    best
}

#[test]
fn al_b27_rapido_mudar_de_lamina_nao_espera_o_disco() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("cheio.json");
    let full: Vec<(String, u32)> = (0..MAX_DESIGNS)
        .map(|i| (format!("HHHH|paginas:E=L{i};D=L{i}|m0c"), 5000))
        .collect();
    write_weights(&file, &full);
    let before = std::fs::read(&file).unwrap();
    let mut store = LearnedStore::load(&file);
    let (queue, jobs) = channel();
    store.set_queue(queue);
    let mut editor = Editor::new();
    editor.set_learned_store(store);
    editor.adopt(album());

    let got = fastest_leave(&mut editor, budget(10));
    assert!(got < budget(10), "mudar de lâmina: {got:?}");
    assert_eq!(std::fs::read(&file).unwrap(), before, "só muda quando roda");

    // Os trabalhos em ordem deixam o arquivo do último.
    let mut all: Vec<_> = jobs.try_iter().collect();
    assert!(!all.is_empty());
    let second = choose_and_leave(&mut editor);
    all.extend(jobs.try_iter());
    for job in all {
        job.run().unwrap();
    }
    let saved = LearnedStore::load(&file);
    assert_eq!(saved.learned.weight(&second), 1000);
    assert_eq!(saved.learned, *editor.learned());
}

fn choose_and_leave(editor: &mut Editor) -> String {
    assert!(editor.go_to_spread(0));
    let key = choose(editor);
    assert!(editor.go_to_spread(1));
    key
}
