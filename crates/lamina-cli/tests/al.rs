//! AL-B26 da spec `specs/aprendizado-de-layouts.md`: a linha de comando não lê o arquivo do
//! aprendizado. Com peso máximo nele, `lamina build` e `lamina auto` dão o mesmo resultado, e o
//! arquivo não muda.

// Arquivo só de testes (`tests/`): as funções auxiliares também podem usar `unwrap` (AGENTS.md,
// regra 3, vale fora de testes).
#![allow(clippy::unwrap_used)]

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Base, text};
use lamina_editor::suggestions::{Part, lists, selection};
use lamina_editor::{LEARNED_FILE, Session};
use lamina_layout::design_key;

/// A pasta de configuração que o `lamina` vai usar com `HOME`, `APPDATA` e `XDG_CONFIG_HOME`
/// apontando para `root` (a mesma conta de `config_dir`).
fn config_in(root: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        root.join("Library")
            .join("Application Support")
            .join("Lamina")
    } else {
        root.join("Lamina")
    }
}

/// Roda o `lamina` com a configuração em `root`.
fn lamina(base: &Base, root: &Path, args: &[&str]) {
    let out = Command::new(env!("CARGO_BIN_EXE_lamina"))
        .current_dir(base.path(""))
        .args(args)
        .env("HOME", root)
        .env("APPDATA", root)
        .env("XDG_CONFIG_HOME", root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
}

/// `lamina build` em lâminas pequenas, gravando em `saida`.
fn build(base: &Base, root: &Path, saida: &str) -> Vec<u8> {
    let cache = base.path("cache");
    let args = [
        "build",
        "fotos",
        "--saida",
        saida,
        "--cache",
        cache.to_str().unwrap(),
        "--largura-cm",
        "5",
        "--altura-cm",
        "5",
        "--dpi",
        "72",
    ];
    lamina(base, root, &args);
    std::fs::read(base.path(saida).join("fotos.lamina")).unwrap()
}

/// `lamina auto` nas mesmas fotos, gravando o desenho em `svg`.
fn auto(base: &Base, root: &Path, svg: &str) -> Vec<u8> {
    let cache = base.path("cache");
    let args = [
        "auto",
        "fotos",
        "--laminas",
        "3",
        "--svg",
        svg,
        "--cache",
        cache.to_str().unwrap(),
    ];
    lamina(base, root, &args);
    std::fs::read(base.path(svg)).unwrap()
}

/// Peso máximo em todas as opções de todas as lâminas, menos a que o álbum usa: quem lesse o
/// arquivo montaria outro álbum.
fn every_other_design(album: &Path) -> String {
    let session = Session::open(album).unwrap();
    let project = session.album();
    let mut weights = Vec::new();
    for index in 0..project.spreads.len() {
        let sel = selection(project, index, Part::Spread).unwrap();
        let options = lists(project, &sel).unwrap().in_order;
        let keys = options.iter().map(|c| design_key(c, &sel.aspects));
        weights.extend(keys.skip(1).map(|key| format!("{key:?}:4294967295")));
    }
    assert!(!weights.is_empty());
    format!("{{\"versao\":1,\"pesos\":{{{}}}}}", weights.join(","))
}

#[test]
fn al_b26_a_linha_de_comando_nao_le_o_arquivo() {
    let base = Base::with_photos(8, (300, 200));
    let (empty, taught) = (base.path("sem"), base.path("com"));
    std::fs::create_dir_all(&empty).unwrap();

    let without = build(&base, &empty, "saida-sem");
    let drawn = auto(&base, &empty, "sem.svg");

    let folder = config_in(&taught);
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join(LEARNED_FILE);
    let body = every_other_design(&base.path("saida-sem/fotos.lamina"));
    std::fs::write(&file, &body).unwrap();

    assert!(
        build(&base, &taught, "saida-com") == without,
        "o arquivo mudou o `lamina build`"
    );
    assert!(
        auto(&base, &taught, "com.svg") == drawn,
        "o arquivo mudou o `lamina auto`"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), body);

    // Nenhum fonte da linha de comando, do servidor da IA ou da IA cita o arquivo.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for dir in [
        "lamina-cli/src",
        "lamina-mcp/src",
        "lamina-editor/src/agent",
    ] {
        for source in sources(&root.join(dir)) {
            let code = std::fs::read_to_string(&source).unwrap();
            assert!(
                !code.contains("LearnedStore") && !code.contains("LEARNED_FILE"),
                "{source:?} cita o arquivo do aprendizado"
            );
        }
    }
}

fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
    found
}
