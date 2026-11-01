//! Limpeza do cache de compilação (ADR 0015): `cargo xtask limpar [--dias N] [--poupar <pasta>]
//! [--simular]` apaga o cache incremental das worktrees sem compilação recente. O cache é a maior
//! parte do `target` e o cargo não o apaga sozinho.
//!
//! Só `target/<perfil>/incremental` é apagado: os programas compilados, que são os que o dono
//! confere, ficam. A trava do próprio cargo é segurada durante a limpeza: quem está compilando
//! faz a pasta ser pulada, e quem começar depois espera.

use std::fs::{self, File, TryLockError};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

const PROFILES: [&str; 2] = ["debug", "release"];
const DEFAULT_DAYS: u64 = 2;
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// O que a limpeza faz: quanto tempo sem compilar basta e se só mede, sem apagar.
#[derive(Debug, Clone, Copy)]
struct Options {
    min_age: Duration,
    simulate: bool,
}

/// O resultado em uma pasta de perfil (`target/debug`, por exemplo).
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    NoCache,
    Busy,
    Recent,
    Cleaned(Tally),
}

/// Bytes liberados e arquivos que ficaram (um arquivo em uso não pode ser apagado no Windows).
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    bytes: u64,
    kept: u64,
}

pub fn run(args: &[String]) -> bool {
    let Some((options, spared)) = parse_args(args) else {
        eprintln!("uso: cargo xtask limpar [--dias N] [--poupar <pasta>]... [--simular]");
        return false;
    };
    let Some(worktrees) = list_worktrees() else {
        eprintln!("  ✘ limpar: não consegui listar as worktrees com o git");
        return false;
    };
    let spared: Vec<PathBuf> = spared.iter().map(|p| canonical(p)).collect();
    let mut total = 0;
    for worktree in worktrees.iter().filter(|w| !spared.contains(&canonical(w))) {
        total += clean_worktree(worktree, options);
    }
    let verb = if options.simulate {
        "liberaria"
    } else {
        "liberou"
    };
    println!("  limpar: {verb} {:.1} GB", total as f64 / 1e9);
    true
}

fn parse_args(args: &[String]) -> Option<(Options, Vec<PathBuf>)> {
    let mut days = DEFAULT_DAYS;
    let mut simulate = false;
    let mut spared = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--dias" => days = rest.next()?.parse().ok()?,
            "--poupar" => spared.push(PathBuf::from(rest.next()?)),
            "--simular" => simulate = true,
            _ => return None,
        }
    }
    let min_age = Duration::from_secs(days * SECONDS_PER_DAY);
    Some((Options { min_age, simulate }, spared))
}

fn list_worktrees() -> Option<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| parse_worktrees(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_worktrees(porcelain: &str) -> Vec<PathBuf> {
    porcelain
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .collect()
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Limpa os perfis de uma worktree, conta o que fez na tela e devolve os bytes liberados.
fn clean_worktree(worktree: &Path, options: Options) -> u64 {
    let mut bytes = 0;
    for profile in PROFILES {
        let outcome = clean_profile(&worktree.join("target").join(profile), options);
        let what = match &outcome {
            Outcome::NoCache => continue,
            Outcome::Busy => "em compilação, pulada".to_string(),
            Outcome::Recent => "compilada há pouco, pulada".to_string(),
            Outcome::Cleaned(t) => {
                bytes += t.bytes;
                format!(
                    "{:.1} GB, {} arquivo(s) em uso",
                    t.bytes as f64 / 1e9,
                    t.kept
                )
            }
        };
        println!("  {} ({profile}): {what}", worktree.display());
    }
    bytes
}

/// A trava é o mesmo arquivo que o cargo usa e fica presa até o fim desta função.
fn clean_profile(profile: &Path, options: Options) -> Outcome {
    let cache = profile.join("incremental");
    if !cache.is_dir() {
        return Outcome::NoCache;
    }
    let Some(_lock) = take_lock(profile) else {
        return Outcome::Busy;
    };
    if !idle(profile, options.min_age) {
        return Outcome::Recent;
    }
    let mut tally = Tally::default();
    sweep(&cache, options.simulate, &mut tally);
    Outcome::Cleaned(tally)
}

fn take_lock(profile: &Path) -> Option<File> {
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(profile.join(".cargo-lock"))
        .ok()?;
    match file.try_lock() {
        Ok(()) => Some(file),
        Err(TryLockError::WouldBlock | TryLockError::Error(_)) => None,
    }
}

/// A data do `.cargo-lock` não serve: o cargo trava o arquivo sem escrever nele. As marcas de
/// compilação (`.fingerprint`) e as pastas do cache mudam a cada compilação.
fn idle(profile: &Path, min_age: Duration) -> bool {
    if min_age.is_zero() {
        return true;
    }
    let newest = ["incremental", ".fingerprint"]
        .iter()
        .filter_map(|name| newest_entry(&profile.join(name)))
        .max();
    newest
        .and_then(|time| time.elapsed().ok())
        .is_some_and(|age| age >= min_age)
}

fn newest_entry(folder: &Path) -> Option<SystemTime> {
    fs::read_dir(folder)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.metadata().ok()?.modified().ok())
        .max()
}

/// Percorre a pasta somando o que apaga; o que não sair (arquivo em uso) fica para a próxima vez.
fn sweep(folder: &Path, simulate: bool, tally: &mut Tally) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let (path, Ok(meta)) = (entry.path(), entry.metadata()) else {
            continue;
        };
        if meta.is_dir() {
            sweep(&path, simulate, tally);
        } else if simulate || fs::remove_file(&path).is_ok() {
            tally.bytes += meta.len();
        } else {
            tally.kept += 1;
        }
    }
    if !simulate {
        let _ = fs::remove_dir(folder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: Options = Options {
        min_age: Duration::ZERO,
        simulate: false,
    };

    /// Um `target/debug` com cache incremental, marca de compilação e o programa.
    fn profile_folder(root: &Path) -> PathBuf {
        let profile = root.join("target").join("debug");
        fs::create_dir_all(profile.join("incremental").join("crate-1")).unwrap();
        fs::create_dir_all(profile.join(".fingerprint").join("crate-1")).unwrap();
        fs::write(
            profile.join("incremental").join("crate-1").join("a.o"),
            [0; 10],
        )
        .unwrap();
        fs::write(profile.join("lamina-app.exe"), [0; 5]).unwrap();
        profile
    }

    #[test]
    fn limpar_apaga_so_o_cache_e_o_programa_fica() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile_folder(dir.path());
        let outcome = clean_profile(&profile, NOW);
        assert_eq!(outcome, Outcome::Cleaned(Tally { bytes: 10, kept: 0 }));
        assert!(!profile.join("incremental").exists());
        assert!(profile.join("lamina-app.exe").exists());
        assert!(profile.join(".fingerprint").exists());
    }

    #[test]
    fn limpar_pula_a_pasta_com_a_trava_ocupada() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile_folder(dir.path());
        let compiling = File::create(profile.join(".cargo-lock")).unwrap();
        compiling.lock().unwrap();
        assert_eq!(clean_profile(&profile, NOW), Outcome::Busy);
        assert!(
            profile
                .join("incremental")
                .join("crate-1")
                .join("a.o")
                .exists()
        );
    }

    #[test]
    fn limpar_pula_a_pasta_compilada_ha_pouco() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile_folder(dir.path());
        let options = Options {
            min_age: Duration::from_secs(SECONDS_PER_DAY),
            simulate: false,
        };
        assert_eq!(clean_profile(&profile, options), Outcome::Recent);
        assert!(profile.join("incremental").exists());
    }

    #[test]
    fn limpar_simulado_mede_sem_apagar() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile_folder(dir.path());
        let options = Options {
            simulate: true,
            ..NOW
        };
        let outcome = clean_profile(&profile, options);
        assert_eq!(outcome, Outcome::Cleaned(Tally { bytes: 10, kept: 0 }));
        assert!(
            profile
                .join("incremental")
                .join("crate-1")
                .join("a.o")
                .exists()
        );
    }

    #[test]
    fn limpar_sem_cache_nao_cria_a_trava() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(clean_profile(dir.path(), NOW), Outcome::NoCache);
        assert!(!dir.path().join(".cargo-lock").exists());
    }

    #[test]
    fn limpar_le_as_worktrees_do_git() {
        let porcelain =
            "worktree D:/a\nHEAD 123\nbranch refs/heads/x\n\nworktree D:/b c\nHEAD 456\n";
        let expected = [PathBuf::from("D:/a"), PathBuf::from("D:/b c")];
        assert_eq!(parse_worktrees(porcelain), expected);
    }

    #[test]
    fn limpar_entende_os_argumentos_e_recusa_o_que_nao_conhece() {
        let args = ["--dias", "3", "--poupar", "D:/a", "--simular"].map(String::from);
        let (options, spared) = parse_args(&args).unwrap();
        assert_eq!(options.min_age, Duration::from_secs(3 * SECONDS_PER_DAY));
        assert!(options.simulate);
        assert_eq!(spared, [PathBuf::from("D:/a")]);
        assert!(parse_args(&["--apagar-tudo".to_string()]).is_none());
    }
}
