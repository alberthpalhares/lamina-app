//! Cenários da spec `o-que-ha-de-novo` (NV) da página: a ilustração e o comando de cada novidade
//! (NV-B12) e a lista das versões (NV-B13, reescrito pela spec PV).

use std::path::Path;

use crate::Command;
use crate::updates::Version;
use crate::updates::news::{entries, from_help, illustration, parse};

const REAL: &str = include_str!("../../../lamina-app/dist/novidades.md");

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

#[test]
fn nv_b12_ilustracao_e_comando_de_cada_novidade() {
    let text = "## [0.1.0-beta.5] - 2026-10-24\n\n\
                - **Atalhos.** A lista. Experimente: aperte F1. <!-- imagem: atalhos.png; comando:\n  \
                ajuda.atalhos -->\n\
                - **Outra.** Sem comentário.\n";
    let sections = parse(text);
    let items = &sections[0].items;
    assert_eq!(items[0].image.as_deref(), Some("atalhos.png"));
    assert_eq!(items[0].command.as_deref(), Some("ajuda.atalhos"));
    assert_eq!(items[0].text, "A lista.");
    assert_eq!(items[0].try_it.as_deref(), Some("aperte F1."));
    assert_eq!(
        (items[1].image.as_ref(), items[1].command.as_ref()),
        (None, None)
    );
    // No arquivo de verdade, toda imagem existe e todo comando existe.
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lamina-app/dist/novidades");
    // O "Não lançado" conta como uma versão: as ilustrações dele também precisam existir.
    let sections = parse(&REAL.replace("## [Não lançado]", "## [9.9.9]"));
    let all: Vec<_> = sections.iter().flat_map(|s| &s.items).collect();
    assert!(all.iter().any(|i| i.image.is_some()));
    for item in &all {
        if let Some(image) = &item.image {
            let bytes = std::fs::read(folder.join(image)).unwrap();
            assert!(illustration(&bytes).is_some(), "{image} não é imagem");
        }
        if let Some(command) = &item.command {
            assert!(
                Command::from_name(command).is_some(),
                "{command} não existe"
            );
        }
    }
    assert!(illustration(b"nada").is_none());
}

#[test]
fn nv_b13_a_lista_da_pagina_traz_o_cabecalho_de_cada_versao() {
    let text = "## [0.1.0-beta.5] - 2026-10-24\n\nAbertura.\n\n\
                - **Atalhos.** A lista.\n\
                - Um texto sem título que continua por várias palavras.\n\n\
                ## [0.1.0-beta.4] - 2026-10-20\n\n- **Antes.** Um item.\n";
    let page = from_help(&parse(text), &v("0.1.0-beta.5"));
    let list = entries(&page);
    assert_eq!(list.len(), 3);
    assert_eq!(
        list[0].heading.as_deref(),
        Some("Versão 0.1.0-beta.5 · 24/10/2026")
    );
    assert_eq!(list[0].intro, "Abertura.");
    assert!(list[0].current, "Esta versão");
    assert_eq!(list[1].heading, None);
    assert_eq!(list[1].label, "Um texto sem título que…");
    assert_eq!(
        list[2].heading.as_deref(),
        Some("Versão 0.1.0-beta.4 · 20/10/2026")
    );
    assert!(!list[2].current);
    assert_eq!(list[0].label, "Atalhos", "sem o ponto final");
}
