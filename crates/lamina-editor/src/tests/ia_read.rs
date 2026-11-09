//! Cenários da spec IA que leem o álbum: IA-B1 a IA-B3.

use serde_json::json;

use super::common::Base;
use super::ia_common::{Lab, unused};
use crate::agent::ToolError;
use crate::frame_dpi_text;
use crate::hand::frame_mm;

#[test]
fn ia_b1_abrir_e_resumir() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let reply = lab.ok("resumo_do_album", json!({}));
    let d = &reply.data;
    assert_eq!(d["laminas"], 6);
    assert_eq!(d["fotos"]["total"], 30);
    assert_eq!(d["fotos"]["sem_uso"], 2);
    // Com os modelos de fábrica na montagem (spec RM, beta 9.1): 3 avisos agora.
    assert_eq!(d["avisos"], 3, "{d}");
    assert_eq!(d["alteracoes_nao_salvas"], false);
    assert!(reply.text.ends_with("sem alterações."), "{}", reply.text);
    let warnings = lab.ok("pre_impressao", json!({}));
    // Com os modelos de fábrica na montagem (spec RM, beta 9.1), o álbum de 6 lâminas sai com
    // desenhos que dão 3 avisos de pré-impressão (a foto pequena continua entre eles).
    let texts: Vec<&str> = warnings.data["avisos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["aviso"].as_str().unwrap())
        .collect();
    assert!(
        texts.iter().any(|t| t.starts_with("Lâmina 2: a foto")),
        "{texts:?}"
    );
}

#[test]
fn ia_b2_ver_uma_lamina_com_previa() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let reply = lab.ok("ver_lamina", json!({ "lamina": 2 }));
    let project = lab.project();
    let trim = project.spec.trim_rect();
    let frames = reply.data["quadros"].as_array().unwrap();
    assert_eq!(frames.len(), project.spreads[1].frames.len());
    for (i, f) in frames.iter().enumerate() {
        assert_eq!(f["numero"], i + 1);
        let mm = frame_mm(project, 1, i).unwrap();
        let x = f["posicao_mm"]["x"].as_f64().unwrap();
        assert!(
            (x - (mm.x.get() - trim.x.get())).abs() <= 0.05,
            "quadro {i}"
        );
        let dpi = frame_dpi_text(project, 1, i).map(|d| d.0);
        assert_eq!(f["dpi_efetivo"].as_str().map(str::to_owned), dpi);
        assert!(f["foto"]["numero"].is_u64());
    }
    assert_eq!(reply.images.len(), 1);
    assert_eq!(reply.images[0].mime, "image/png");
    let png = image::load_from_memory(&reply.images[0].bytes).unwrap();
    assert_eq!(png.width().max(png.height()), 1024);
    let canvas = project.spec.canvas_rect();
    let expected = canvas.width.get() / canvas.height.get();
    let got = f64::from(png.width()) / f64::from(png.height());
    assert!((got - expected).abs() < 0.01, "{got} × {expected}");
}

#[test]
fn ia_b3_imagens_desligadas() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, false);
    let spread = lab.ok("ver_lamina", json!({ "lamina": 2 }));
    assert!(spread.images.is_empty());
    let layouts = lab.ok("sugerir_layouts", json!({ "lamina": 2, "imagens": true }));
    assert!(layouts.images.is_empty());
    assert!(!layouts.data["opcoes"].as_array().unwrap().is_empty());
    let first = unused(lab.project())[0].get();
    let err = lab
        .call("ver_fotos", json!({ "fotos": [first] }))
        .unwrap_err();
    assert!(matches!(err, ToolError::ImagesOff));
    assert_eq!(
        err.to_string(),
        "As imagens estão desligadas nas configurações do Lamina. Use listar_fotos para os dados \
         das fotos."
    );
    let summary = lab.ok("resumo_do_album", json!({}));
    assert_eq!(summary.data["imagens"], "desligadas");
}

/// As miniaturas vêm na ordem pedida, e a lista das fotos sem uso bate com o álbum (seção 3).
#[test]
fn ia_b3_com_imagens_as_miniaturas_vem_na_ordem() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let free = unused(lab.project());
    let numbers: Vec<u64> = free.iter().map(|id| id.get()).collect();
    let listed = lab.ok("listar_fotos", json!({ "filtro": "sem_uso" }));
    let listed: Vec<u64> = listed.data["fotos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["numero"].as_u64().unwrap())
        .collect();
    assert_eq!(listed, numbers);
    let thumbs = lab.ok("ver_fotos", json!({ "fotos": [numbers[1], numbers[0]] }));
    assert_eq!(thumbs.images.len(), 2);
    assert_eq!(thumbs.data["fotos"][0]["numero"], numbers[1]);
    let too_many: Vec<u64> = (1..=25).collect();
    let err = lab
        .call("ver_fotos", json!({ "fotos": too_many }))
        .unwrap_err();
    assert!(matches!(err, ToolError::TooManyPhotos(24)));
}
