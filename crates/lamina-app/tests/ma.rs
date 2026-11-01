//! Cenários da spec `abrir-no-mac` (MA) que precisam do executável ou dos arquivos do pacote: o
//! `Info.plist`, a verificação escondida `--verificar-abertura` e o `instalar.sh` no Mac.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use lamina_editor::{Cache, NewAlbum, Session, SpecFields, create_album};

const APP: &str = env!("CARGO_BIN_EXE_lamina-app");
const DIR: &str = env!("CARGO_MANIFEST_DIR");
const UTI: &str = "io.github.alberthpalhares.lamina.album";

/// Roda o app com a configuração numa pasta própria e espera no máximo `limit`.
fn run(args: &[&str], home: &Path, limit: Duration) -> (Option<i32>, String) {
    let mut child = Command::new(APP)
        .args(args)
        .env("LOCALAPPDATA", home.join("local"))
        .env("APPDATA", home.join("roaming"))
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_CONFIG_HOME", home.join("config"))
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

/// Há tela para abrir a janela? (o CI dos PRs roda o Linux sem tela)
fn has_display() -> bool {
    cfg!(any(windows, target_os = "macos"))
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
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
        name: Some("duplo-clique".into()),
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
        .save_as(&dir.join("duplo-clique.lamina"))
        .unwrap()
}

/// O `Info.plist` como a release o entrega ao `vpk` (a mesma troca do `sed` do `release.yml`).
fn plist(version: &str) -> String {
    let short = version.split('-').next().unwrap();
    std::fs::read_to_string(format!("{DIR}/dist/Info.plist"))
        .unwrap()
        .replace("@VERSAO@", version)
        .replace("@VERSAO_CURTA@", short)
}

/// O valor `<string>` logo depois da chave `key`, a partir de `from`.
fn value_after(text: &str, from: usize, key: &str) -> String {
    let tag = format!("<key>{key}</key>");
    let at = text[from..]
        .find(&tag)
        .unwrap_or_else(|| panic!("sem {key}"))
        + from
        + tag.len();
    let rest = &text[at..];
    let start = rest.find("<string>").unwrap() + "<string>".len();
    let end = rest.find("</string>").unwrap();
    rest[start..end].to_owned()
}

#[test]
fn ma_b7_o_pacote_declara_o_lamina() {
    let text = plist("0.1.0-beta.5");
    assert!(!text.contains('@'), "toda marca trocada");
    let get = |key: &str| value_after(&text, 0, key);
    assert_eq!(get("CFBundleExecutable"), "lamina-app");
    assert_eq!(get("CFBundleIconFile"), "lamina.icns");
    assert_eq!(
        get("CFBundleIdentifier"),
        "io.github.alberthpalhares.lamina"
    );
    assert_eq!(get("CFBundleVersion"), "0.1.0-beta.5");
    assert_eq!(get("CFBundleShortVersionString"), "0.1.0");
    assert_eq!(get("CFBundlePackageType"), "APPL");
    assert_eq!(get("NSPrincipalClass"), "NSApplication");
    let docs = text.find("<key>CFBundleDocumentTypes</key>").unwrap();
    assert_eq!(value_after(&text, docs, "CFBundleTypeRole"), "Editor");
    assert_eq!(value_after(&text, docs, "LSHandlerRank"), "Owner");
    assert_eq!(value_after(&text, docs, "LSItemContentTypes"), UTI);
    let types = text.find("<key>UTExportedTypeDeclarations</key>").unwrap();
    assert_eq!(value_after(&text, types, "UTTypeIdentifier"), UTI);
    assert_eq!(value_after(&text, types, "UTTypeConformsTo"), "public.data");
    assert_eq!(
        value_after(&text, types, "public.filename-extension"),
        "lamina"
    );
    // Um comentário XML com dois hífens seguidos deixa o arquivo inválido para o Mac.
    let inside = text
        .split("<!--")
        .nth(1)
        .unwrap()
        .split("-->")
        .next()
        .unwrap();
    assert!(!inside.contains("--"), "comentário inválido");
    // A release e o CI do Mac entregam este arquivo ao vpk e ajustam o .pkg.
    for workflow in ["release.yml", "ci.yml"] {
        let yml =
            std::fs::read_to_string(format!("{DIR}/../../.github/workflows/{workflow}")).unwrap();
        assert!(yml.contains("--plist Info.plist"), "{workflow}");
        assert!(yml.contains("dist/ajustar-pkg.sh"), "{workflow}");
        // O vpk recusa --bundleId com --plist: o identificador vem do Info.plist.
        assert!(!yml.contains("--bundleId"), "{workflow}");
    }
}

#[test]
fn ma_b8_verificacao_escondida_da_abertura() {
    let dir = tempfile::tempdir().unwrap();
    let (code, err) = run(
        &["--verificar-abertura"],
        dir.path(),
        Duration::from_secs(30),
    );
    assert_eq!(code, Some(1));
    assert!(err.contains("falta o arquivo do relatório"), "{err:?}");
    if !has_display() {
        return;
    }
    let report = dir.path().join("relatorio.txt");
    let good = album(dir.path());
    let (code, err) = run(
        &[
            "--verificar-abertura",
            report.to_str().unwrap(),
            good.to_str().unwrap(),
        ],
        dir.path(),
        Duration::from_secs(60),
    );
    let text = std::fs::read_to_string(&report).unwrap_or_default();
    assert_eq!(code, Some(0), "{err}{text}");
    assert!(text.starts_with("aberto: "), "{text}");
    assert!(text.contains("duplo-clique.lamina (2 lâminas)"), "{text}");
    // Fora do Mac, sem o álbum no argumento, nada chega: erro depois do tempo limite.
    if cfg!(not(target_os = "macos")) {
        let nothing = dir.path().join("nada.txt");
        let (code, _) = run(
            &["--verificar-abertura", nothing.to_str().unwrap()],
            dir.path(),
            Duration::from_secs(60),
        );
        assert_eq!(code, Some(1));
        let text = std::fs::read_to_string(&nothing).unwrap();
        assert_eq!(text, "erro: nenhum arquivo chegou\n");
    }
}

/// MA-B6 pela janela de verdade: um álbum com alterações, o recebido chega, a pergunta "Salvar
/// as alterações?" aparece (`files::request`), a resposta "Não salvar" segue pelo mesmo caminho do
/// botão (`files::answer` → `opened::proceed`) e o recebido abre. O job "Abrir no Mac" repete pelo
/// `open` do Mac.
#[test]
fn ma_b6_a_janela_pergunta_antes_de_abrir() {
    if !has_display() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("relatorio.txt");
    let good = album(dir.path());
    let (code, err) = run(
        &[
            "--verificar-abertura",
            report.to_str().unwrap(),
            "--com-alteracoes",
            good.to_str().unwrap(),
        ],
        dir.path(),
        Duration::from_secs(60),
    );
    let text = std::fs::read_to_string(&report).unwrap_or_default();
    assert_eq!(code, Some(0), "{err}{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0], "pergunta: Salvar as alterações? → Não salvar");
    assert!(
        lines[1].starts_with("aberto: ") && lines[1].ends_with("duplo-clique.lamina (2 lâminas)"),
        "{text}"
    );
}

#[test]
fn ma_b9_arquivo_que_nao_e_album() {
    if !has_display() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("IMG_0001.jpg");
    image::RgbImage::new(4, 4).save(&photo).unwrap();
    let report = dir.path().join("relatorio.txt");
    let (code, err) = run(
        &[
            "--verificar-abertura",
            report.to_str().unwrap(),
            photo.to_str().unwrap(),
        ],
        dir.path(),
        Duration::from_secs(60),
    );
    assert_eq!(code, Some(1), "{err}");
    let text = std::fs::read_to_string(&report).unwrap();
    assert_eq!(text, "erro: O Lamina só abre álbuns (arquivos .lamina).\n");
}

/// MA-B10: no Mac, o `instalar.sh` põe o app na pasta Aplicativos do sistema (aqui uma pasta de
/// teste, `LAMINA_APLICATIVOS`) e apaga o antigo da pasta pessoal; sem permissão de gravar lá,
/// instala na pasta pessoal. O job "Abrir no Mac" do CI repete com o `/Applications` de verdade.
#[cfg(target_os = "macos")]
#[test]
fn ma_b10_instalar_pelo_terminal_no_mac() {
    let (release, home) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let mount = release.path().join("monta/Lamina.app/Contents/MacOS");
    std::fs::create_dir_all(&mount).unwrap();
    std::fs::write(mount.join("lamina-app"), b"#!/bin/sh\necho lamina\n").unwrap();
    let zip = release.path().join("Lamina-osx-Portable.zip");
    let ok = Command::new("ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(release.path().join("monta/Lamina.app"))
        .arg(&zip)
        .status()
        .unwrap();
    assert!(ok.success());
    let sum = Command::new("shasum")
        .args(["-a", "256"])
        .arg(&zip)
        .output()
        .unwrap();
    let sum = String::from_utf8(sum.stdout).unwrap();
    let sum = sum.split_whitespace().next().unwrap();
    std::fs::write(
        release.path().join("SHA256SUMS"),
        format!("{sum}  Lamina-osx-Portable.zip\n"),
    )
    .unwrap();
    let install = |apps: &Path| {
        Command::new("sh")
            .arg(format!("{DIR}/dist/instalar.sh"))
            .env("LAMINA_FONTE", release.path())
            .env("LAMINA_APLICATIVOS", apps)
            .env("HOME", home.path())
            .output()
            .unwrap()
    };
    let personal = home.path().join("Applications/Lamina.app");
    std::fs::create_dir_all(personal.join("Contents")).unwrap();
    let system = home.path().join("sistema");
    std::fs::create_dir_all(&system).unwrap();
    let out = install(&system);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        system.join("Lamina.app/Contents/MacOS/lamina-app").exists(),
        "{text}"
    );
    assert!(!personal.exists(), "o antigo da pasta pessoal saiu");
    assert!(text.contains("Pronto!"), "{text}");
    // Sem permissão na pasta do sistema: a pasta pessoal, com aviso.
    let locked = home.path().join("trancada");
    std::fs::create_dir_all(&locked).unwrap();
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    }
    let out = install(&locked);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{text}");
    assert!(
        personal.join("Contents/MacOS/lamina-app").exists(),
        "{text}"
    );
    assert!(
        text.contains("pasta Aplicativos da sua pasta pessoal"),
        "{text}"
    );
}
