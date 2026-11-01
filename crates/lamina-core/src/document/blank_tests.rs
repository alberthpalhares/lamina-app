//! As páginas em branco no arquivo e nas contas (spec PB, PB-B1 e PB-B2).

use serde_json::Value;

use crate::{AlbumEdge, AlbumSpec, BlankPages, Dpi, Mm, PageSide, Project};

fn album(spreads: usize) -> Project {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    for _ in 0..spreads {
        project.add_spread();
    }
    project
}

#[test]
fn pb_b1_formato_8_regrava_igual_e_so_grava_a_marca_quando_existe() {
    // O arquivo como a versão 7 gravava.
    let without = album(2);
    let mut v7: Value = serde_json::from_str(&without.to_json().unwrap()).unwrap();
    v7["schema_version"] = serde_json::json!(7);
    let opened = Project::from_json(&v7.to_string()).unwrap();
    let saved = opened.to_json().unwrap();
    assert!(saved.contains("\"schema_version\": 8"), "{saved}");
    assert!(!saved.contains("em_branco"), "{saved}");
    assert_eq!(saved, without.to_json().unwrap());

    // Só a lâmina 1 tem a marca, e só o lado marcado vai para o arquivo.
    let mut with = album(2);
    with.spreads[0].blank.right = true;
    let saved = with.to_json().unwrap();
    let parsed: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        parsed["spreads"][0]["em_branco"],
        serde_json::json!({ "direita": true })
    );
    assert!(parsed["spreads"][1].get("em_branco").is_none());
    let reopened = Project::from_json(&saved).unwrap();
    assert_eq!(reopened, with);
    assert_eq!(reopened.to_json().unwrap(), saved);
}

#[test]
fn pb_b2_pagina_em_branco_conta_e_nao_recebe_conteudo() {
    let mut project = album(10);
    project.spec = project
        .spec
        .with_edges(AlbumEdge::SinglePage, AlbumEdge::FullSpread);
    project.spreads[3].blank.left = true;
    let edges = project.spec.edges();

    let first = (edges.active_pages(0, 10), project.content_pages(0));
    assert!(!first.0.left && first.0.right);
    assert!(!first.1.left && first.1.right);

    let fourth = (edges.active_pages(3, 10), project.content_pages(3));
    assert!(
        fourth.0.left && fourth.0.right,
        "as duas continuam impressas"
    );
    assert!(
        !fourth.1.left && fourth.1.right,
        "só a direita tem conteúdo"
    );

    assert_eq!(edges.page_count(10), 19);
}

#[test]
fn marca_le_e_grava_o_lado_certo() {
    let mut blank = BlankPages::default();
    assert!(blank.is_none());
    blank.set(PageSide::Left, true);
    assert!(blank.has(PageSide::Left) && !blank.has(PageSide::Right));
    assert!(!blank.is_none());
    blank.set(PageSide::Left, false);
    assert!(blank.is_none());
}
