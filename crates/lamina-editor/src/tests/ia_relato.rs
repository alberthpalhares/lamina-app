//! Cenários do relato do teste pelo MCP (2026-10-29; spec IA, IA-B25 a IA-B27): a primeira
//! lâmina num álbum em branco, o nome do álbum e o preenchimento nas sugestões de layout.

use serde_json::{Value, json};

use super::common::Base;
use super::ia_common::Lab;
use crate::agent::{ToolError, ToolReply};

/// Um álbum em branco (só as fotos) criado pela IA, como no relato: `montar: false`.
fn blank(base: &Base, extra: Value) -> Lab {
    let roots = vec![base.path("fotos"), base.path("ia")];
    let mut lab = Lab::new(base, roots, false);
    let mut args = json!({
        "pasta_fotos": base.path("fotos"),
        "salvar_em": base.path("ia").join("Teste Manual MCP.lamina"),
    });
    if let (Some(args), Some(extra)) = (args.as_object_mut(), extra.as_object()) {
        args.extend(extra.clone());
    }
    lab.ok("novo_album", args);
    assert!(lab.project().spreads.is_empty());
    lab
}

#[test]
fn ia_b25_montar_a_primeira_lamina_num_album_em_branco() {
    let base = Base::with_photos(12, (600, 400));
    for posicao in [None, Some(1)] {
        let mut lab = blank(&base, json!({}));
        let mut args = json!({ "fotos": [1, 2, 3] });
        if let Some(p) = posicao {
            args["posicao"] = json!(p);
        }
        let reply = lab.ok("montar_lamina", args);
        assert_eq!(lab.project().spreads.len(), 1);
        assert_eq!(reply.data["lamina"]["numero"], 1);
        assert_eq!(reply.data["lamina"]["quadros"].as_array().unwrap().len(), 3);
        assert_eq!(lab.labels(), ["IA: Montar lâmina · lâmina 1"]);
        std::fs::remove_file(base.path("ia").join("Teste Manual MCP.lamina")).unwrap();
    }
}

#[test]
fn ia_b25_nova_lamina_num_album_em_branco() {
    let base = Base::with_photos(12, (600, 400));
    let mut lab = blank(&base, json!({}));
    lab.ok("nova_lamina", json!({}));
    assert_eq!(lab.project().spreads.len(), 1);
    assert!(lab.project().spreads[0].frames.is_empty());
    // A segunda entra na frente, como já entrava num álbum com lâminas.
    lab.ok("montar_lamina", json!({ "fotos": [4, 5], "posicao": 1 }));
    assert_eq!(lab.project().spreads.len(), 2);
    assert_eq!(lab.project().spreads[0].frames.len(), 2);
    assert!(lab.project().spreads[1].frames.is_empty());
}

#[test]
fn ia_b26_nome_do_album_na_criacao() {
    let base = Base::with_photos(12, (600, 400));
    let lab = blank(&base, json!({ "nome": "  Casamento Ana e Leo " }));
    assert_eq!(lab.project().name, "Casamento Ana e Leo");
    // Sem nome, vale o nome do arquivo pedido, não o da pasta das fotos.
    let other = Base::with_photos(12, (600, 400));
    assert_eq!(blank(&other, json!({})).project().name, "Teste Manual MCP");
}

#[test]
fn ia_b26_renomear_o_album() {
    let base = Base::with_photos(12, (600, 400));
    let mut lab = blank(&base, json!({}));
    let reply = lab.ok("renomear_album", json!({ "nome": "Ensaio Gestante" }));
    assert_eq!(lab.project().name, "Ensaio Gestante");
    assert_eq!(reply.data["alteracoes_nao_salvas"], true);
    assert_eq!(lab.labels(), ["IA: Renomear álbum · álbum"]);
    let err = lab
        .call("renomear_album", json!({ "nome": "   " }))
        .unwrap_err();
    assert!(matches!(err, ToolError::Refused(_)));
    assert_eq!(err.to_string(), "O nome do álbum não pode ficar vazio.");
    let err = lab
        .call("renomear_album", json!({ "nome": "Ensaio Gestante" }))
        .unwrap_err();
    assert_eq!(err.to_string(), "O álbum já se chama Ensaio Gestante.");
    assert_eq!(lab.labels().len(), 1);
}

#[test]
fn ia_b26_nome_invalido_e_recusado() {
    let base = Base::with_photos(12, (600, 400));
    let mut lab = blank(&base, json!({}));
    let err = lab
        .call("renomear_album", json!({ "nome": "Ana: Leo" }))
        .unwrap_err();
    assert!(matches!(
        err,
        ToolError::Core(lamina_core::CoreError::InvalidName(_))
    ));
    assert_eq!(lab.project().name, "Teste Manual MCP");

    let target = base.path("ia").join("Outro.lamina");
    let err = lab
        .call(
            "novo_album",
            json!({
                "pasta_fotos": base.path("fotos"),
                "salvar_em": target,
                "nome": "Ana: Leo",
            }),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        ToolError::Core(lamina_core::CoreError::InvalidName(_))
    ));
}

fn options(reply: &ToolReply) -> Vec<Value> {
    reply.data["opcoes"].as_array().unwrap().clone()
}

#[test]
fn ia_b27_sugestoes_mostram_area_menor_foto_e_corte() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, false);
    let list = lab.ok("sugerir_layouts", json!({ "lamina": 3 }));
    for o in options(&list) {
        for field in ["area_ocupada", "menor_foto", "corte_maximo"] {
            let v = o[field].as_f64().unwrap();
            assert!((0.0..=100.0).contains(&v), "{field} = {v}");
        }
    }
}

#[test]
fn ia_b27_ordem_por_preenchimento_e_tolerancia_de_corte() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, false);
    let args = json!({ "lamina": 3, "ordem": "preenchimento", "quantas": 24 });
    let list = options(&lab.ok("sugerir_layouts", args));
    let areas: Vec<f64> = list
        .iter()
        .map(|o| o["area_ocupada"].as_f64().unwrap())
        .collect();
    assert!(areas.windows(2).all(|w| w[0] >= w[1]), "{areas:?}");
    let args = json!({ "lamina": 3, "corte_maximo": 0, "quantas": 24 });
    let uncut = options(&lab.ok("sugerir_layouts", args));
    assert!(
        uncut
            .iter()
            .all(|o| o["corte_maximo"].as_f64() == Some(0.0))
    );

    // aplicar_layout usa a opção da lista que a IA viu, na ordem pedida.
    let args = json!({ "lamina": 3, "ordem": "preenchimento" });
    let first = options(&lab.ok("sugerir_layouts", args))[0].clone();
    let applied = lab.ok("aplicar_layout", json!({ "lamina": 3, "opcao": 1 }));
    let again = options(&lab.ok(
        "sugerir_layouts",
        json!({ "lamina": 3, "ordem": "preenchimento" }),
    ));
    let used = again.iter().find(|o| o["em_uso"] == true).unwrap();
    assert_eq!(used["area_ocupada"], first["area_ocupada"]);
    assert_eq!(applied.data["lamina"]["numero"], 3);
}

#[test]
fn ia_b28_novo_album_diz_por_que_nao_achou_fotos() {
    let base = Base::with_photos(0, (10, 10));
    let folder = base.path("fotos");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("leia-me.txt"), "contrato").unwrap();
    let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], false);
    let call = |lab: &mut Lab| {
        let target = base.path("ia").join("a.lamina");
        lab.call(
            "novo_album",
            json!({ "pasta_fotos": folder, "salvar_em": target }),
        )
        .unwrap_err()
        .to_string()
    };
    assert!(call(&mut lab).ends_with(": 1 arquivo, mas nenhum em JPEG, PNG, TIFF ou HEIC."));
    // Fotos que existem, mas não podem ser lidas, não são "nenhuma foto encontrada".
    std::fs::write(folder.join("f01.jpg"), b"nao e um jpeg").unwrap();
    std::fs::write(folder.join("f02.jpg"), b"nao e um jpeg").unwrap();
    let text = call(&mut lab);
    assert!(text.starts_with("Nenhuma das 2 fotos de "), "{text}");
    assert!(text.contains("Primeiro motivo: f01.jpg: "), "{text}");
}
