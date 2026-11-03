//! Cenários da spec `aprovacao-cliente-html` (AH), os campos e a pasta: nome do arquivo, WhatsApp,
//! e-mail, formulário, resumo, substituir e velocidade.

use std::path::PathBuf;

use super::ac::{generate, plain};
use super::ah::html_only;
use super::ex::{Album, names, solid, spec};
use crate::{
    ApprovalOptions, ApprovalReport, ExportOptions, approval_summary, clean_whatsapp,
    client_name_problem, email_problem, html_file_name, outputs_problem, whatsapp_display,
};

#[test]
fn ah_b11_nome_de_arquivo() {
    for (name, file) in [
        ("Ana e Pedro", "Ana e Pedro-aprovacao.html"),
        ("Ana/Pedro: 2026", "Ana Pedro 2026-aprovacao.html"),
        ("///", "album-aprovacao.html"),
        ("CON", "album-aprovacao.html"),
        ("com1", "album-aprovacao.html"),
        ("Festa.", "Festa-aprovacao.html"),
    ] {
        assert_eq!(html_file_name(name), file, "{name}");
    }
}

#[test]
fn ah_b12_numero_do_whatsapp() {
    for (typed, clean) in [
        ("(11) 99999-9999", "5511999999999"),
        ("011 99999-9999", "5511999999999"),
        ("+55 11 3333-4444", "551133334444"),
        ("+1 415 555 0100", "14155550100"),
    ] {
        assert_eq!(clean_whatsapp(typed), Ok(Some(clean.to_owned())), "{typed}");
    }
    assert_eq!(
        clean_whatsapp("123"),
        Err("Confira o WhatsApp: use o código do país, o DDD e o número.")
    );
    assert_eq!(clean_whatsapp("  "), Ok(None));
    // O campo já abre com o código do país: só ele é o mesmo que vazio.
    assert_eq!(clean_whatsapp("+55 "), Ok(None));
    assert_eq!(
        clean_whatsapp("+55 11 99999-9999"),
        Ok(Some("5511999999999".to_owned()))
    );
    assert_eq!(whatsapp_display("5511999999999"), "+55 11 99999-9999");
    assert_eq!(whatsapp_display("551133334444"), "+55 11 3333-4444");
    assert_eq!(whatsapp_display("14155550100"), "+14155550100");
}

#[test]
fn ah_b13_email() {
    assert_eq!(email_problem("a@b.com"), None);
    assert_eq!(email_problem(""), None);
    assert_eq!(email_problem("a b@c.com"), Some("Confira o e-mail."));
    assert_eq!(email_problem("sem-arroba"), Some("Confira o e-mail."));
    assert_eq!(email_problem("a@b@c.com"), Some("Confira o e-mail."));
    assert_eq!(email_problem("@b.com"), Some("Confira o e-mail."));
}

#[test]
fn ah_b14_nenhuma_saida_ligada() {
    let album = Album::new(spec(), 1);
    assert_eq!(
        outputs_problem(&album.project, false, false),
        Some("Escolha pelo menos um: HTML ou JPG.")
    );
    assert_eq!(outputs_problem(&album.project, true, true), None);
    let empty = Album::new(spec(), 0);
    assert_eq!(
        outputs_problem(&empty.project, false, true),
        Some("O HTML precisa de pelo menos uma lâmina.")
    );
    assert_eq!(outputs_problem(&empty.project, true, false), None);
    let message = Some("O nome que o cliente vê vai de 1 a 60 caracteres.");
    assert_eq!(client_name_problem(" "), message);
    assert_eq!(client_name_problem(&"a".repeat(61)), message);
    assert_eq!(client_name_problem(&"a".repeat(60)), None);
}

#[test]
fn ah_b7_o_aviso_de_capa_so_vale_para_o_html_sem_capa() {
    let mut album = Album::new(spec(), 1);
    let notice = crate::html_cover_notice;
    assert!(notice(&album.project, &html_only("Ana")).is_some());
    assert_eq!(notice(&album.project, &plain()), None, "só o JPG");
    super::ac::with_cover(&mut album);
    assert_eq!(notice(&album.project, &html_only("Ana")), None, "com capa");
}

#[test]
fn ah_b15_resumo() {
    let report = |bytes: u64| ApprovalReport {
        files: vec![PathBuf::from("Ana-aprovacao.html")],
        bytes,
        code: "3F2A9C".to_owned(),
        removed: Vec::new(),
        html_bytes: Some(bytes),
    };
    let weight = "O e-mail pode recusar este arquivo. Mande pelo WhatsApp, como documento.";
    let mail = "Por e-mail, o cliente precisa baixar o arquivo para folhear. Prefira o WhatsApp, ou mande os JPG por e-mail.";
    let iphone = "No iPhone, o arquivo pode abrir como lista, sem folhear. Para esses clientes, mande também os JPG.";
    let heavy = approval_summary(&report(21_000_000));
    assert!(heavy.contains(weight), "{heavy}");
    assert!(heavy.contains(mail), "{heavy}");
    assert!(heavy.contains(iphone), "{heavy}");
    let light = approval_summary(&report(19_000_000));
    assert!(!light.contains(weight), "{light}");
    assert!(light.contains(mail), "{light}");
    assert!(light.contains(iphone), "{light}");
}

#[test]
fn ah_b17_substituir_com_o_html() {
    let album = Album::new(spec(), 1);
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    for name in ["web-lamina-40.jpg", "Antigo-aprovacao.html", "notas.txt"] {
        std::fs::write(folder.join(name), name).unwrap();
    }
    let html = ApprovalOptions {
        overwrite: true,
        ..html_only("Ana")
    };
    let report = generate(&album, folder, &html).unwrap();
    assert_eq!(report.removed, [folder.join("Antigo-aprovacao.html")]);
    assert_eq!(
        names(folder),
        ["Ana-aprovacao.html", "notas.txt", "web-lamina-40.jpg"]
    );
    let jpg = ApprovalOptions {
        overwrite: true,
        ..plain()
    };
    let report = generate(&album, folder, &jpg).unwrap();
    assert_eq!(report.removed, [folder.join("web-lamina-40.jpg")]);
    assert_eq!(
        names(folder),
        ["Ana-aprovacao.html", "notas.txt", "web-lamina-01.jpg"]
    );
}

/// As sobras e o HTML que já existe também entram na pergunta de substituir.
#[test]
fn ah_b17_nao_substitui_o_html_sem_perguntar() {
    let album = Album::new(spec(), 1);
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    std::fs::write(folder.join("Antigo-aprovacao.html"), b"x").unwrap();
    let found = crate::approval_existing(&album.project, folder, &html_only("Ana")).unwrap();
    assert_eq!(found, ["Antigo-aprovacao.html"]);
    assert!(matches!(
        generate(&album, folder, &html_only("Ana")),
        Err(crate::ExportError::Exists { .. })
    ));
    let only_jpg = crate::approval_existing(&album.project, folder, &plain()).unwrap();
    assert!(only_jpg.is_empty(), "só com o JPG, o HTML fica");
}

#[test]
fn ah_b18_aprovacao_e_impressao_na_mesma_pasta() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Festa-aprovacao.html");
    std::fs::write(&file, b"album").unwrap();
    let options = ExportOptions {
        overwrite: true,
        ..ExportOptions::default()
    };
    Album::new(spec(), 1).export(dir.path(), &options).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"album");
}

/// O intervalo vale só para o JPG: o HTML sai com o álbum inteiro.
#[test]
fn com_intervalo_o_html_continua_inteiro() {
    let album = Album::new(spec(), 3);
    let dir = tempfile::tempdir().unwrap();
    let options = ApprovalOptions {
        range: Some("2".to_owned()),
        ..html_only("Ana")
    };
    let options = ApprovalOptions {
        jpg: true,
        ..options
    };
    let report = generate(&album, dir.path(), &options).unwrap();
    assert_eq!(
        names(dir.path()),
        ["Ana-aprovacao.html", "web-lamina-02.jpg"]
    );
    let html = std::fs::read_to_string(report.files.last().unwrap()).unwrap();
    assert!(html.contains("id=\"lamina-1\""));
    assert!(html.contains("id=\"lamina-3\""));
}

#[test]
fn ah_b19_gerar_jpg_e_html_rapido() {
    use lamina_core::testing::{HEAVY_TRIES, budget};
    let copy = solid(1600, 1067, [90, 120, 150]);
    let mut album = Album::new(spec(), 10);
    for s in 0..10 {
        for n in 0..6u32 {
            let photo = album.photo(&format!("f{s}_{n}.jpg"), copy.clone());
            let (col, row) = (f64::from(n % 2), f64::from(n / 2));
            album.photo_frame(s, photo, [0.03 + col * 0.5, 0.04 + row * 0.32, 0.44, 0.29]);
        }
    }
    let options = ApprovalOptions {
        jpg: true,
        overwrite: true,
        ..html_only("Ana")
    };
    let dir = tempfile::tempdir().unwrap();
    let mut elapsed = std::time::Duration::MAX;
    for _ in 0..HEAVY_TRIES {
        let start = std::time::Instant::now();
        generate(&album, dir.path(), &options).unwrap();
        elapsed = elapsed.min(start.elapsed());
        if elapsed < budget(6000) {
            break;
        }
    }
    assert!(elapsed < budget(6000), "gerar levou {elapsed:?}");
}
