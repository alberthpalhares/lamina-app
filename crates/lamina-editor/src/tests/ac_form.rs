//! Cenários das specs AC e AH no editor, a caixa: os padrões, o que cada campo reclama, as
//! opções que eles dão e os textos.

use std::path::Path;

use super::ac::album;
use crate::{
    APPROVAL_FILE, ApprovalForm, ApprovalStore, approval_finished, approval_progress,
    approval_replace,
};

/// O JPG e o HTML vêm ligados, com o nome do álbum.
#[test]
fn a_caixa_abre_com_os_padroes_da_spec() {
    let store = ApprovalStore::default();
    let form = ApprovalForm::new(&store.data, &album("Festa"));
    assert!(form.jpg && form.html && form.all_spreads && form.number);
    assert!(!form.watermark_on);
    assert_eq!(form.client_name, "Festa");
    assert_eq!(form.whatsapp, "+55 ", "o código do país já vem preenchido");
    assert!(form.problems(&album("Festa")).is_clear());
    assert_eq!(form.options(false).html.unwrap().whatsapp, None);
    assert_eq!(form.whatsapp_shown(), "");
    assert_eq!(
        form.data().whatsapp,
        "",
        "só o código não é um número guardado"
    );
}

#[test]
fn ah_b16_o_numero_guardado_abre_como_foi_digitado() {
    let mut store = ApprovalStore::default();
    store.data.whatsapp = "+55 11 99999-9999".to_owned();
    let form = ApprovalForm::new(&store.data, &album("Festa"));
    assert_eq!(form.whatsapp, "+55 11 99999-9999");
    assert_eq!(form.data().whatsapp, "+55 11 99999-9999");
    assert_eq!(form.whatsapp_shown(), "+55 11 99999-9999");
}

#[test]
fn as_opcoes_saem_limpas_do_formulario() {
    let project = album("Festa");
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, &project);
    form.whatsapp = "(11) 99999-9999".to_owned();
    form.email = " estudio@exemplo.com ".to_owned();
    form.client_name = " Ana e Pedro ".to_owned();
    form.all_spreads = false;
    form.range = " 1 ".to_owned();
    form.watermark_on = true;
    form.watermark_text = " Estúdio ".to_owned();
    let options = form.options(true);
    assert!(options.overwrite && options.jpg);
    assert_eq!(options.range.as_deref(), Some("1"));
    assert_eq!(options.watermark.as_deref(), Some("Estúdio"));
    let html = options.html.unwrap();
    assert_eq!(html.name, "Ana e Pedro");
    assert_eq!(html.whatsapp.as_deref(), Some("5511999999999"));
    assert_eq!(html.email.as_deref(), Some("estudio@exemplo.com"));

    form.html = false;
    form.jpg = true;
    form.watermark_on = false;
    assert_eq!(form.options(false).html, None);
    assert_eq!(form.options(false).watermark, None, "desligada");
    form.jpg = false;
    form.html = true;
    assert_eq!(form.options(false).range, None, "o intervalo é só do JPG");
}

#[test]
fn cada_campo_diz_o_proprio_problema() {
    let project = album("Festa");
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, &project);
    form.watermark_on = true;
    form.client_name = " ".to_owned();
    form.whatsapp = "123".to_owned();
    form.email = "sem-arroba".to_owned();
    form.all_spreads = false;
    form.range = "9".to_owned();
    let problems = form.problems(&project);
    assert!(!problems.is_clear());
    assert_eq!(
        problems.watermark.as_deref(),
        Some("Escreva o texto da marca-d'água.")
    );
    assert_eq!(
        problems.name.as_deref(),
        Some("O nome que o cliente vê vai de 1 a 60 caracteres.")
    );
    assert_eq!(
        problems.whatsapp.as_deref(),
        Some("Confira o WhatsApp: use o código do país, o DDD e o número.")
    );
    assert_eq!(problems.email.as_deref(), Some("Confira o e-mail."));
    assert_eq!(
        problems.range.as_deref(),
        Some("O intervalo pede a lâmina 9, mas o álbum tem 1.")
    );
    assert_eq!(problems.outputs, None);

    // Campo de saída desligada não reclama: é o que a caixa não mostra.
    form.html = false;
    let problems = form.problems(&project);
    assert_eq!(
        (problems.name, problems.whatsapp, problems.email),
        (None, None, None)
    );
    form.jpg = false;
    assert_eq!(
        form.problems(&project).outputs.as_deref(),
        Some("Escolha pelo menos um: HTML ou JPG.")
    );
    form.html = true;
    assert_eq!(
        form.problems(&project).range,
        None,
        "o intervalo é só do JPG"
    );
}

#[test]
fn andamento_conta_os_arquivos() {
    assert_eq!(approval_progress(3, 41), "Gerando 3 de 41…");
}

#[test]
fn ah_b12_a_caixa_mostra_como_o_numero_ficou() {
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, &album("Festa"));
    assert_eq!(form.whatsapp_shown(), "", "vazio");
    form.whatsapp = "(11) 99999-9999".to_owned();
    assert_eq!(form.whatsapp_shown(), "+55 11 99999-9999");
    form.whatsapp = "123".to_owned();
    assert_eq!(form.whatsapp_shown(), "", "com erro, o campo já avisa");
}

#[test]
fn ac_b18_a_pergunta_usa_o_nome_da_pasta_e_o_intervalo_so_vale_no_jpg() {
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, &album("Festa"));
    let folder = Path::new("fotos").join("aprovacao-cliente");
    let (title, text) = approval_replace(41, &folder, &form);
    assert_eq!(title, "Substituir os 41 arquivos?");
    assert!(
        text.starts_with("A pasta aprovacao-cliente já tem 41 arquivos"),
        "{text}"
    );
    assert!(text.contains("os que sobrarem também saem"), "{text}");
    form.all_spreads = false;
    let (_, text) = approval_replace(2, &folder, &form);
    assert!(text.contains("As outras lâminas continuam lá"), "{text}");
    form.jpg = false;
    let (_, text) = approval_replace(2, &folder, &form);
    assert!(
        text.contains("os que sobrarem também saem"),
        "o HTML sai inteiro: {text}"
    );
}

fn report() -> lamina_export::ApprovalReport {
    lamina_export::ApprovalReport {
        files: vec![std::path::PathBuf::from("a.jpg"); 41],
        bytes: 21_000_000,
        code: "3F2A9C".to_owned(),
        removed: Vec::new(),
        html_bytes: None,
    }
}

#[test]
fn ac_b30_o_fim_guarda_os_dados_e_devolve_o_resumo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(APPROVAL_FILE);
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, &album("Festa"));
    form.watermark_on = true;
    form.watermark_text = "Estúdio".to_owned();
    let text = approval_finished(ApprovalStore::load(&file), form.data(), &report());
    assert!(text.contains("prova 3F2A9C"), "{text}");
    assert_eq!(ApprovalStore::load(&file).data.watermark_text, "Estúdio");
}

/// Não conseguir guardar não desfaz os arquivos: o resumo sai e avisa.
#[test]
fn ac_b30_sem_conseguir_guardar_o_resumo_avisa() {
    let dir = tempfile::tempdir().unwrap();
    // Uma pasta no lugar do arquivo: gravar falha.
    let file = dir.path().join(APPROVAL_FILE);
    std::fs::create_dir_all(&file).unwrap();
    let store = ApprovalStore::load(&file);
    let text = approval_finished(store, crate::PhotographerData::default(), &report());
    assert!(text.contains("prova 3F2A9C"), "{text}");
    assert!(
        text.contains("Não foi possível gravar os dados da aprovação"),
        "{text}"
    );
}
