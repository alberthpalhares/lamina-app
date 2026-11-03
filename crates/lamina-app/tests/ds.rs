//! Cenários das specs `distribuicao` (DS) e `primeiro-contato` (PC) que precisam do executável: os
//! ganchos da instalação e a verificação escondida do teste de fumaça, também com o álbum de exemplo.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use lamina_editor::{Cache, NewAlbum, Session, SpecFields, create_album};

const APP: &str = env!("CARGO_BIN_EXE_lamina-app");

/// Roda o app e espera no máximo `limit`; devolve o código de saída e a mensagem de erro.
fn run(args: &[&str], limit: Duration) -> (Option<i32>, String) {
    run_env(args, &[], limit)
}

/// Como [`run`], com variáveis de ambiente a mais.
fn run_env(args: &[&str], env: &[(&str, &str)], limit: Duration) -> (Option<i32>, String) {
    let mut child = Command::new(APP)
        .args(args)
        .envs(env.iter().copied())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let mut err = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                std::io::Read::read_to_string(&mut pipe, &mut err).unwrap();
            }
            return (status.code(), err);
        }
        if start.elapsed() > limit {
            child.kill().unwrap();
            panic!("{args:?} passou de {limit:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Um álbum pequeno de verdade, com quatro fotos.
fn album(dir: &Path) -> PathBuf {
    let photos = dir.join("fotos");
    std::fs::create_dir_all(&photos).unwrap();
    for i in 0..4u8 {
        let img = image::RgbImage::from_fn(300, 200, |x, _| {
            image::Rgb([i * 40, u8::try_from(x % 255).unwrap(), 90])
        });
        img.save(photos.join(format!("f{i}.jpg"))).unwrap();
    }
    let fields = SpecFields {
        width_cm: 10.0,
        height_cm: 10.0,
        dpi: 100,
        ..SpecFields::default()
    };
    let new = NewAlbum {
        folder: photos,
        name: Some("fumaca".into()),
        spec: fields.to_spec().unwrap(),
        spreads: Some(2),
        reuse: lamina_layout::Reuse::Medium,
        preferences: lamina_layout::Preferences::default(),
        learned: lamina_layout::Learned::default(),
        models: Vec::new(),
        limits: lamina_core::PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
        style: Default::default(),
        files: Vec::new(),
        build: true,
    };
    let (built, _) = create_album(&new, &Cache::new(dir.join("cache")), &|_| {}).unwrap();
    Session::new_unsaved(built.project)
        .save_as(&dir.join("fumaca.lamina"))
        .unwrap()
}

/// Há tela para abrir a janela? (o CI dos PRs roda o Linux sem tela)
fn has_display() -> bool {
    cfg!(any(windows, target_os = "macos"))
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[test]
fn ds_b14_os_ganchos_nao_abrem_janela() {
    for hook in [
        "--veloapp-install",
        "--veloapp-updated",
        "--veloapp-uninstall",
    ] {
        let (code, _) = run(&[hook, "0.2.0"], Duration::from_secs(30));
        assert_eq!(code, Some(0), "{hook}");
    }
}

#[test]
fn ds_b15_verificacao_escondida() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("nao-e-album.lamina");
    std::fs::write(&bad, "isto não é um álbum").unwrap();
    let (code, err) = run(
        &["--verificar", bad.to_str().unwrap()],
        Duration::from_secs(30),
    );
    assert_eq!(code, Some(1));
    // No Windows o app é de janela: a mensagem só chega com a saída redirecionada, como aqui.
    assert!(err.contains("--verificar:"), "{err:?}");
    if has_display() {
        let good = album(dir.path());
        let (code, err) = run(
            &["--verificar", good.to_str().unwrap()],
            Duration::from_secs(60),
        );
        assert_eq!(code, Some(0), "{err}");
    }
}

/// PC-B8 (spec `primeiro-contato`): `--verificar exemplo` abre o álbum de exemplo que veio com o
/// programa; no `cargo test`, o do repositório.
#[test]
fn pc_b8_verificar_o_exemplo() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("sem-exemplo");
    let (code, err) = run_env(
        &["--verificar", "exemplo"],
        &[("LAMINA_EXEMPLO", empty.to_str().unwrap())],
        Duration::from_secs(30),
    );
    assert_eq!(code, Some(1));
    assert!(
        err.contains("O álbum de exemplo não veio com esta instalação."),
        "{err:?}"
    );
    if has_display() {
        let (code, err) = run(&["--verificar", "exemplo"], Duration::from_secs(60));
        assert_eq!(code, Some(0), "{err}");
    }
}
