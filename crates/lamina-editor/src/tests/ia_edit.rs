//! Cenários da spec IA que editam o álbum: IA-B4 a IA-B10.

use lamina_core::{FrameContent, Project};
use serde_json::{Value, json};

use super::common::Base;
use super::ia_common::{Lab, unused};
use crate::agent::ToolError;
use crate::{Editor, Part};

#[test]
fn ia_b4_sugerir_e_aplicar_um_layout() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, file) = Lab::opened(&base, true);
    let list = lab.ok("sugerir_layouts", json!({ "lamina": 3 }));
    assert!(list.data["opcoes"].as_array().unwrap().len() >= 2);
    lab.ok("aplicar_layout", json!({ "lamina": 3, "opcao": 2 }));

    let mut window = Editor::new();
    window.open(&file).unwrap();
    window.go_to_spread(2);
    assert!(window.apply_option(Part::Spread, 1));
    let expected = &window.session().unwrap().project().spreads[2];
    assert_eq!(&lab.project().spreads[2], expected);
    assert_eq!(lab.labels(), ["IA: Trocar modelo · lâmina 3"]);
}

#[test]
fn ia_b5_lista_vencida() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let err = lab
        .call("aplicar_layout", json!({ "lamina": 3, "opcao": 1 }))
        .unwrap_err();
    assert!(matches!(err, ToolError::NoList(3)));
    lab.ok("sugerir_layouts", json!({ "lamina": 3 }));
    let free = unused(lab.project())[0].get();
    lab.ok(
        "colocar_foto",
        json!({ "lamina": 3, "quadro": 1, "foto": free }),
    );
    let before = lab.project().clone();
    let err = lab
        .call("aplicar_layout", json!({ "lamina": 3, "opcao": 1 }))
        .unwrap_err();
    assert!(matches!(err, ToolError::StaleList(3)));
    assert_eq!(
        err.to_string(),
        "A lâmina 3 mudou depois da lista de layouts. Peça sugerir_layouts de novo."
    );
    assert_eq!(lab.project(), &before);
}

fn kind(frame: &Value) -> &str {
    frame["tipo"].as_str().unwrap()
}

/// Os três passos de IA-B6 na lâmina 2, com um quadro de texto no fim; devolve o projeto depois
/// de cada passo.
fn three_steps(lab: &mut Lab) -> Vec<Project> {
    lab.ok(
        "executar_comando",
        json!({ "comando": "lamina.novo-texto", "lamina": 2 }),
    );
    let free = unused(lab.project())[0].get();
    let mut states = Vec::new();
    let placed = lab.ok(
        "colocar_foto",
        json!({ "lamina": 2, "quadro": 1, "foto": free }),
    );
    assert_eq!(placed.data["lamina"]["quadros"][0]["foto"]["numero"], free);
    states.push(lab.project().clone());
    let swapped = lab.ok(
        "trocar_fotos",
        json!({ "lamina": 2, "quadro_a": 1, "quadro_b": 2 }),
    );
    assert_eq!(swapped.data["lamina"]["quadros"][1]["foto"]["numero"], free);
    states.push(lab.project().clone());
    let before = lab.project().spreads[1].frames.len();
    let cleared = lab.ok("tirar_foto", json!({ "lamina": 2, "quadro": 3 }));
    let frames = cleared.data["lamina"]["quadros"].as_array().unwrap();
    assert_eq!(frames.len(), before - 1, "a foto sai com o quadro");
    assert!(frames.iter().all(|f| kind(f) != "vazio"));
    states.push(lab.project().clone());
    states
}

#[test]
fn ia_b6_colocar_trocar_e_tirar_fotos() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    three_steps(&mut lab);
    let labels = lab.labels();
    assert_eq!(
        labels[1..],
        [
            "IA: Colocar foto · lâmina 2",
            "IA: Trocar fotos · lâmina 2",
            "IA: Tirar foto · lâmina 2"
        ]
    );
    // O texto continua na lâmina depois de tirar a foto (o layout novo muda a ordem dos quadros).
    let text = 1 + lab.project().spreads[1]
        .frames
        .iter()
        .position(|f| matches!(f.content, FrameContent::Text(_)))
        .unwrap();
    let free = unused(lab.project())[0].get();
    let err = lab
        .call(
            "colocar_foto",
            json!({ "lamina": 2, "quadro": text, "foto": free }),
        )
        .unwrap_err();
    assert!(matches!(err, ToolError::TextFrame { spread: 2, .. }));
    assert_eq!(
        err.to_string(),
        format!("O quadro {text} da lâmina 2 é de texto. Escolha um quadro de foto.")
    );
}

#[test]
fn ia_b7_montar_uma_lamina_com_fotos_escolhidas() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let free: Vec<u64> = unused(lab.project()).iter().map(|id| id.get()).collect();
    let reply = lab.ok("montar_lamina", json!({ "fotos": free, "posicao": 4 }));
    assert_eq!(lab.project().spreads.len(), 7);
    let numbers: Vec<u64> = reply.data["lamina"]["quadros"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["foto"]["numero"].as_u64().unwrap())
        .collect();
    assert_eq!(reply.data["lamina"]["numero"], 4);
    assert_eq!(numbers, free);
    assert!(unused(lab.project()).is_empty());
    assert_eq!(lab.labels(), ["IA: Montar lâmina · lâmina 4"]);
}

#[test]
fn ia_b8_nova_mover_e_apagar_lamina() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let original = lab.project().spreads.clone();
    lab.ok("nova_lamina", json!({}));
    assert_eq!(lab.project().spreads.len(), 7);
    lab.ok("mover_lamina", json!({ "lamina": 7, "para": 1 }));
    assert!(lab.project().spreads[0].frames.is_empty());
    lab.ok("apagar_lamina", json!({ "lamina": 1 }));
    assert_eq!(lab.project().spreads, original);
    assert_eq!(lab.labels().len(), 3);
    assert!(lab.labels().iter().all(|l| l.starts_with("IA: ")));
}

#[test]
fn ia_b9_desfazer_e_refazer() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let states = three_steps(&mut lab);
    let undone = lab.ok("desfazer", json!({ "passos": 2 }));
    assert_eq!(lab.project(), &states[0]);
    assert_eq!(undone.data["passos"].as_array().unwrap().len(), 2);
    lab.ok("refazer", json!({}));
    assert_eq!(lab.project(), &states[1]);
    let history = lab.ok("historico", json!({}));
    let steps = history.data["passos"].as_array().unwrap();
    assert_eq!(steps.len(), 4);
    assert!(
        steps
            .iter()
            .all(|s| s["rotulo"].as_str().unwrap().starts_with("IA: "))
    );
    assert_eq!(history.data["atual"], 3);
}

#[test]
fn ia_b10_executar_um_comando_pelo_nome() {
    let base = Base::with_photos(30, (600, 400));
    let (mut by_name, _) = Lab::opened(&base, true);
    by_name.ok(
        "executar_comando",
        json!({ "comando": "layout.espelhar", "lamina": 2 }),
    );
    let (mut by_tool, _) = Lab::opened(&base, true);
    by_tool.ok("espelhar", json!({ "lamina": 2 }));
    assert_eq!(by_name.project().spreads[1], by_tool.project().spreads[1]);
    assert_eq!(by_name.labels(), ["IA: Espelhar lâmina · lâmina 2"]);

    let err = by_name
        .call("executar_comando", json!({ "comando": "arquivo.exportar" }))
        .unwrap_err();
    assert!(matches!(
        err,
        ToolError::DialogCommand {
            tool: "exportar",
            ..
        }
    ));
    assert_eq!(
        err.to_string(),
        "O comando arquivo.exportar abre uma janela. Use a ferramenta exportar no lugar."
    );
    let err = by_name
        .call("executar_comando", json!({ "comando": "xyz" }))
        .unwrap_err();
    assert!(matches!(err, ToolError::UnknownCommand(_)));
    assert_eq!(
        err.to_string(),
        "Não existe o comando xyz. A lista está em lamina://comandos."
    );
}

/// Spec IA, regra 3: um comando que muda as opções sem mudar a lâmina (alternar as reordenadas,
/// favoritar) também vence a lista; aplicar a opção 1 depois dele aplicaria outra opção.
#[test]
fn comando_que_muda_as_opcoes_vence_a_lista() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    for command in ["layout.favoritar", "layout.reordenadas"] {
        lab.ok("sugerir_layouts", json!({ "lamina": 3 }));
        lab.ok(
            "executar_comando",
            json!({ "comando": command, "lamina": 3 }),
        );
        let before = lab.project().clone();
        let err = lab
            .call("aplicar_layout", json!({ "lamina": 3, "opcao": 1 }))
            .unwrap_err();
        assert!(matches!(err, ToolError::NoList(3)), "{command}: {err}");
        assert_eq!(lab.project(), &before);
    }
}
