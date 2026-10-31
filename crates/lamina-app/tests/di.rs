//! As telas da spec `design-tela-inicial` (DI): o que só dá para conferir lendo os `.slint`. O que a
//! tela mostra sai do `lamina-editor` e é testado lá (`tests/di.rs`).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ui(file: &str) -> String {
    fs::read_to_string(manifest().join("ui").join(file)).unwrap()
}

#[test]
fn di_b1_as_telas_usam_os_nomes_curtos_dos_botoes() {
    let screens = [ui("welcome.slint"), ui("welcome_cards.slint")].concat();
    for name in ["Novo álbum…", "Abrir álbum…", "Álbum de exemplo"] {
        assert!(screens.contains(name), "falta {name}");
    }
    for old in [
        "Novo álbum com minhas fotos",
        "Abrir um álbum",
        "Experimentar com o álbum de exemplo",
    ] {
        assert!(!screens.contains(old), "ainda diz {old}");
    }
    // Na primeira vez, o principal é o novo álbum, e o exemplo vem ao lado, sem destaque.
    let welcome = ui("welcome.slint");
    assert!(welcome.contains("text: \"Novo álbum…\"; primary: true"));
    assert!(welcome.contains("DialogButton { text: \"Álbum de exemplo\"; clicked"));
}

#[test]
fn di_b12_a_tela_inicial_entra_nas_travas() {
    let exceptions = fs::read_to_string(manifest().join("../../xtask/src/icones.rs")).unwrap();
    assert!(!exceptions.contains("\"WelcomeScreen\""));
    for file in ["welcome.slint", "welcome_cards.slint"] {
        assert!(!ui(file).contains("Path {"), "{file} desenha ícone à mão");
    }
}
