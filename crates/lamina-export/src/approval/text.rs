//! As frases da aprovação (spec AC, seção 6 e regras 14 e 21): o campo da marca-d'água, a pergunta de
//! substituir e o resumo do fim.

use lamina_core::{Rgb, TextBox};
use lamina_render::check_text;

use super::ApprovalReport;

const MARK_MAX: usize = 40;
/// Acima disto, o e-mail pode recusar o HTML: o Gmail para em 25 MB (spec AH, regra 15).
const EMAIL_WARNING_BYTES: u64 = 20_000_000;

/// O que há de errado com o texto da marca-d'água; `None` quando serve.
pub fn watermark_problem(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return Some("Escreva o texto da marca-d'água.".to_owned());
    }
    if text.chars().count() > MARK_MAX {
        return Some(format!("A marca-d'água vai até {MARK_MAX} caracteres."));
    }
    let probe = TextBox {
        text: text.to_owned(),
        ..TextBox::new(Rgb::BLACK)
    };
    let missing = check_text(&probe, (1000.0, 100.0)).missing;
    missing
        .first()
        .map(|c| format!("A marca-d'água tem um caractere que a fonte não tem: {c}."))
}

/// O título e o texto da pergunta de substituir (regra 14), para `count` arquivos em `folder`, com
/// ou sem intervalo de lâminas.
pub fn approval_question(count: usize, folder: &str, with_range: bool) -> (String, String) {
    let one = count == 1;
    let title = if one {
        "Substituir o arquivo?".to_owned()
    } else {
        format!("Substituir os {count} arquivos?")
    };
    let has = if one {
        format!("A pasta {folder} já tem 1 arquivo deste álbum.")
    } else {
        format!("A pasta {folder} já tem {count} arquivos deste álbum.")
    };
    let enters = if one {
        "O novo entra no lugar dele"
    } else {
        "Os novos entram no lugar deles"
    };
    let rest = if with_range {
        ". As outras lâminas continuam lá."
    } else {
        ", e os que sobrarem também saem. Seus outros arquivos continuam lá."
    };
    (title, format!("{has} {enters}{rest}"))
}

/// "41 arquivos, 21 MB, prova 3F2A9C."
pub fn approval_summary(report: &ApprovalReport) -> String {
    let count = report.files.len();
    let files = if count == 1 {
        "1 arquivo".to_owned()
    } else {
        format!("{count} arquivos")
    };
    let mut text = format!(
        "{files}, {}, prova {}.",
        megabytes(report.bytes),
        report.code
    );
    if let Some(html) = report.html_bytes {
        if html > EMAIL_WARNING_BYTES {
            text.push_str(
                "\nO e-mail pode recusar este arquivo. Mande pelo WhatsApp, como documento.",
            );
        }
        text.push_str(
            "\nPor e-mail, o cliente precisa baixar o arquivo para folhear. Prefira o WhatsApp, ou mande os JPG por e-mail.",
        );
        text.push_str(
            "\nNo iPhone, o arquivo pode abrir como lista, sem folhear. Para esses clientes, mande também os JPG.",
        );
    }
    text
}

/// Com uma casa abaixo de 10 MB, sem casas daí para cima.
fn megabytes(bytes: u64) -> String {
    let megabytes = bytes as f64 / 1_000_000.0;
    if megabytes < 10.0 {
        format!("{megabytes:.1} MB").replace('.', ",")
    } else {
        format!("{megabytes:.0} MB")
    }
}
