//! Cenários da spec `aprovacao-cliente` (AC), a pasta e os textos: impressão protegida, substituir,
//! avisos, foto ausente, velocidade, mesmos bytes e resumo. O código da prova está em `ac_code.rs`.

use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{FrameContent, Photo, Rgb, TextBox};

use super::ac::{generate, plain, with_cover};
use super::ex::{Album, names, solid, spec};
use crate::{
    ApprovalOptions, ApprovalReport, DEFAULT_QUALITY, ExportOptions, ProjectFileError, WarningKind,
    approval_existing, approval_photos_found, approval_question, approval_summary,
    approval_warnings, watermark_problem,
};

fn overwrite() -> ApprovalOptions {
    ApprovalOptions {
        overwrite: true,
        ..plain()
    }
}

#[test]
fn ac_b13_caractere_que_a_fonte_nao_tem() {
    assert_eq!(watermark_problem("Estúdio Exemplo"), None);
    assert_eq!(
        watermark_problem("Estúdio 😀").as_deref(),
        Some("A marca-d'água tem um caractere que a fonte não tem: 😀.")
    );
    assert_eq!(
        watermark_problem("  ").as_deref(),
        Some("Escreva o texto da marca-d'água.")
    );
    assert_eq!(
        watermark_problem(&"a".repeat(41)).as_deref(),
        Some("A marca-d'água vai até 40 caracteres.")
    );
    assert_eq!(watermark_problem(&"a".repeat(40)), None);
}

#[test]
fn ac_b15_impressao_intacta_depois_da_aprovacao() {
    let dir = tempfile::tempdir().unwrap();
    let ours = ["lamina-01.jpg", "capa.jpg", "pagina-01.jpg", "notas.txt"];
    for name in ours {
        std::fs::write(dir.path().join(name), name).unwrap();
    }
    std::fs::write(dir.path().join("web-lamina-99.jpg"), b"x").unwrap();
    generate(&Album::new(spec(), 2), dir.path(), &overwrite()).unwrap();
    assert!(!dir.path().join("web-lamina-99.jpg").exists());
    for name in ours {
        let bytes = std::fs::read(dir.path().join(name)).unwrap();
        assert_eq!(bytes, name.as_bytes(), "{name}");
    }
}

#[test]
fn ac_b16_aprovacao_intacta_depois_da_impressao() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["web-lamina-01.jpg", "web-capa.jpg"] {
        std::fs::write(dir.path().join(name), name).unwrap();
    }
    let options = ExportOptions {
        overwrite: true,
        ..ExportOptions::default()
    };
    Album::new(spec(), 2).export(dir.path(), &options).unwrap();
    for name in ["web-lamina-01.jpg", "web-capa.jpg"] {
        let bytes = std::fs::read(dir.path().join(name)).unwrap();
        assert_eq!(bytes, name.as_bytes(), "{name}");
    }
}

#[test]
fn ac_b17_exportacao_de_impressao_nao_muda() {
    let album = Album::new(spec(), 1);
    let print = |album: &Album| {
        let dir = tempfile::tempdir().unwrap();
        let report = album.export(dir.path(), &ExportOptions::default()).unwrap();
        std::fs::read(&report.files[0]).unwrap()
    };
    let before = print(&album);
    let dir = tempfile::tempdir().unwrap();
    generate(&album, dir.path(), &plain()).unwrap();
    assert_eq!(print(&album), before);
    assert_eq!(DEFAULT_QUALITY, 95);
    assert_eq!(ExportOptions::default().quality, 95);
}

#[test]
fn ac_b18_nao_substitui_sem_perguntar() {
    let album = Album::new(spec(), 2);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("web-lamina-01.jpg"), b"antigo").unwrap();
    std::fs::write(dir.path().join("notas.txt"), b"x").unwrap();
    let found = approval_existing(&album.project, dir.path(), &plain()).unwrap();
    assert_eq!(found, ["web-lamina-01.jpg"]);
    let (title, _) = approval_question(found.len(), "aprovacao-cliente", false);
    assert_eq!(title, "Substituir o arquivo?");

    let refused = generate(&album, dir.path(), &plain());
    assert!(matches!(refused, Err(crate::ExportError::Exists { .. })));
    let kept = std::fs::read(dir.path().join("web-lamina-01.jpg")).unwrap();
    assert_eq!(kept, b"antigo");
    assert_eq!(names(dir.path()), ["notas.txt", "web-lamina-01.jpg"]);

    let (title, text) = approval_question(41, "aprovacao-cliente", false);
    assert_eq!(title, "Substituir os 41 arquivos?");
    assert_eq!(
        text,
        "A pasta aprovacao-cliente já tem 41 arquivos deste álbum. Os novos entram no lugar \
         deles, e os que sobrarem também saem. Seus outros arquivos continuam lá."
    );
    let (_, text) = approval_question(2, "aprovacao-cliente", true);
    assert!(
        text.ends_with("Os novos entram no lugar deles. As outras lâminas continuam lá."),
        "{text}"
    );
}

/// As sobras também contam: de 100 para 99 lâminas, nenhum nome se repete (`001` e `01`).
#[test]
fn ac_b18_sobra_de_outra_numeracao_tambem_pergunta() {
    let album = Album::new(spec(), 2);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("web-lamina-001.jpg"), b"antigo").unwrap();
    let found = approval_existing(&album.project, dir.path(), &plain()).unwrap();
    assert_eq!(found, ["web-lamina-001.jpg"]);
}

fn dummies(folder: &std::path::Path, count: usize) {
    for n in 1..=count {
        let name = format!("web-lamina-{n:02}.jpg");
        std::fs::write(folder.join(name), b"antigo").unwrap();
    }
}

#[test]
fn ac_b19_sobras_saem_ao_substituir() {
    let dir = tempfile::tempdir().unwrap();
    dummies(dir.path(), 40);
    std::fs::write(dir.path().join("notas.txt"), b"x").unwrap();
    let report = generate(&Album::new(spec(), 39), dir.path(), &overwrite()).unwrap();
    assert!(!dir.path().join("web-lamina-40.jpg").exists());
    assert!(dir.path().join("notas.txt").exists());
    assert_eq!(report.removed, [dir.path().join("web-lamina-40.jpg")]);
    assert_eq!(report.files.len(), 39);
}

#[test]
fn ac_b20_com_intervalo_nada_e_apagado() {
    let dir = tempfile::tempdir().unwrap();
    dummies(dir.path(), 40);
    std::fs::write(dir.path().join("web-capa.jpg"), b"antigo").unwrap();
    let mut album = Album::new(spec(), 40);
    with_cover(&mut album);
    let options = ApprovalOptions {
        range: Some("5, 12".to_owned()),
        ..overwrite()
    };
    let report = generate(&album, dir.path(), &options).unwrap();
    assert_eq!(report.files.len(), 2);
    assert!(report.removed.is_empty());
    assert_eq!(names(dir.path()).len(), 41);
    let rewritten = ["web-lamina-05.jpg", "web-lamina-12.jpg"];
    for name in names(dir.path()) {
        let old = std::fs::read(dir.path().join(&name)).unwrap() == b"antigo";
        assert_eq!(old, !rewritten.contains(&name.as_str()), "{name}");
    }
}

#[test]
fn ac_b21_avisos_antes_de_gerar() {
    let mut album = Album::new(spec(), 1);
    let small = album.photo("pequena.jpg", solid(100, 100, [10, 20, 30]));
    album.photo_frame(0, small, [0.05, 0.1, 0.4, 0.8]);
    album.frame(0, FrameContent::Empty, [0.55, 0.1, 0.2, 0.2]);
    let text = FrameContent::Text(TextBox {
        text: "Olá".to_owned(),
        font: "Fonte Que Não Existe".to_owned(),
        ..TextBox::new(Rgb::BLACK)
    });
    album.frame(0, text, [0.55, 0.5, 0.3, 0.2]);
    let warnings = approval_warnings(&album.project, &plain()).unwrap();
    let kinds: Vec<_> = warnings.iter().map(|w| &w.kind).collect();
    assert_eq!(kinds.len(), 2, "{kinds:?}");
    assert!(kinds.iter().any(|k| matches!(k, WarningKind::EmptyFrame)));
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, WarningKind::UnknownFont { .. }))
    );
}

#[test]
fn ac_b22_foto_ausente() {
    let mut album = Album::new(spec(), 2);
    let gone = album.photo("sumiu.jpg", solid(100, 100, [1, 2, 3]));
    album.photo_frame(1, gone, [0.1, 0.1, 0.3, 0.3]);
    let dir = tempfile::tempdir().unwrap();
    let error = approval_photos_found(&album.project, &plain()).unwrap_err();
    assert!(matches!(
        &error,
        ProjectFileError::MissingPhoto { name, .. } if name == "sumiu.jpg"
    ));
    assert_eq!(
        error.to_string(),
        "A foto sumiu.jpg não foi encontrada em sumiu.jpg."
    );
    assert!(names(dir.path()).is_empty(), "nada é gravado");

    // Só o que sai conta: a lâmina 1 não usa a foto, e uma foto fora do álbum não para nada.
    let first = ApprovalOptions {
        range: Some("1".to_owned()),
        ..plain()
    };
    approval_photos_found(&album.project, &first).unwrap();
    album.project.spreads[1].frames.clear();
    approval_photos_found(&album.project, &plain()).unwrap();
}

#[test]
fn ac_b26_gerar_rapido() {
    use lamina_core::testing::{HEAVY_TRIES, budget};
    let copy = Arc::new(solid(1600, 1067, [90, 120, 150]));
    let mut album = Album::new(spec(), 10);
    for s in 0..10 {
        for n in 0..6u32 {
            let photo = album.photo(&format!("f{s}_{n}.jpg"), (*copy).clone());
            let (col, row) = (f64::from(n % 2), f64::from(n / 2));
            album.photo_frame(s, photo, [0.03 + col * 0.5, 0.04 + row * 0.32, 0.44, 0.29]);
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let mut elapsed = std::time::Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let start = std::time::Instant::now();
        generate(&album, dir.path(), &overwrite()).unwrap();
        elapsed = elapsed.min(start.elapsed());
        if elapsed < budget(5000) {
            break;
        }
    }
    assert!(elapsed < budget(5000), "gerar levou {elapsed:?}");
}

#[test]
fn ac_b27_mesmos_bytes() {
    let mut album = Album::new(spec(), 2);
    let photo = album.photo("f.jpg", super::ex::noise(600, 400));
    album.photo_frame(0, photo, [0.1, 0.1, 0.5, 0.6]);
    with_cover(&mut album);
    let options = ApprovalOptions {
        watermark: Some("Estúdio Exemplo".to_owned()),
        ..ApprovalOptions::default()
    };
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let first = generate(&album, a.path(), &options).unwrap();
    let second = generate(&album, b.path(), &options).unwrap();
    assert_eq!(first.files.len(), 3);
    for (x, y) in first.files.iter().zip(&second.files) {
        assert_eq!(std::fs::read(x).unwrap(), std::fs::read(y).unwrap());
    }
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.code, second.code);
}

#[test]
fn ac_b33_resumo() {
    let report = ApprovalReport {
        files: vec![std::path::PathBuf::from("a.jpg"); 41],
        bytes: 21_000_000,
        code: "3F2A9C".to_owned(),
        removed: Vec::new(),
        html_bytes: None,
    };
    let text = approval_summary(&report);
    assert!(text.contains("41 arquivos"), "{text}");
    assert!(text.contains("21 MB"), "{text}");
    assert!(text.contains("prova 3F2A9C"), "{text}");
    let one = ApprovalReport {
        files: vec![std::path::PathBuf::from("a.jpg")],
        bytes: 500_000,
        ..report
    };
    let text = approval_summary(&one);
    assert!(text.contains("1 arquivo,"), "{text}");
    assert!(text.contains("0,5 MB"), "{text}");
}

/// Quem chama com uma fonte vazia não grava meio arquivo: o erro diz o que ficou.
#[test]
fn foto_ilegivel_no_meio_para_e_diz_o_que_ficou() {
    let mut album = Album::new(spec(), 2);
    let photo = album.photo("f.jpg", solid(100, 100, [1, 2, 3]));
    album.photo_frame(1, photo, [0.1, 0.1, 0.3, 0.3]);
    let dir = tempfile::tempdir().unwrap();
    let source = |_: &Photo, _: (u32, u32)| Err::<Arc<RgbaImage>, String>("estragada".to_owned());
    let result = crate::export_approval(&album.project, dir.path(), &source, &plain(), &|_, _| {});
    let Err(crate::ExportError::Stopped { done, .. }) = result else {
        panic!("esperava Stopped: {result:?}");
    };
    assert_eq!(done, [dir.path().join("web-lamina-01.jpg")]);
    assert_eq!(names(dir.path()), ["web-lamina-01.jpg"]);
}

#[test]
fn album_vazio_nao_tem_o_que_gerar() {
    let album = Album::new(spec(), 0);
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        generate(&album, dir.path(), &plain()),
        Err(crate::ExportError::NoSpreads)
    ));
    let mut with = Album::new(spec(), 0);
    with_cover(&mut with);
    generate(&with, dir.path(), &plain()).unwrap();
    assert_eq!(names(dir.path()), ["web-capa.jpg"]);
}
