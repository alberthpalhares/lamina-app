//! Cenários da spec `inicio-rapido` (IQ) que olham as telas (IQ-B6): o botão "Início rápido" na tela
//! inicial e o item "Início rápido" no menu Arquivo, os dois pelo comando `arquivo.inicio-rapido`.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn ui(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
    fs::read_to_string(path).unwrap()
}

#[test]
fn iq_b6_o_botao_comecar_e_o_menu_chamam_o_comando() {
    let welcome = ui("welcome.slint");
    let cards = ui("welcome_cards.slint");
    let app = ui("app.slint");
    // Na primeira vez, "Início rápido" fica no corpo, ao lado do "Novo álbum…", que continua principal
    // (spec DI, regra 2; spec IQ, regra 8).
    assert!(
        welcome.contains("text: \"Início rápido\""),
        "falta o botão no corpo da primeira vez"
    );
    assert!(welcome.contains("root.command(\"arquivo.inicio-rapido\")"));
    assert!(welcome.contains("text: \"Novo álbum…\"; primary: true"));
    // Com recentes, o botão fica na barra de cima.
    assert!(
        cards.contains("text: \"Início rápido\""),
        "falta o botão na barra de quem já usa"
    );
    assert!(cards.contains("root.command(\"arquivo.inicio-rapido\")"));
    // O menu Arquivo tem o item, logo depois do "Novo álbum…".
    assert!(
        app.contains("title: \"Início rápido\""),
        "falta o item no menu Arquivo"
    );
    assert!(app.contains("root.command(\"arquivo.inicio-rapido\")"));
}
