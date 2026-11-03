//! Os campos do HTML na caixa (spec AH, regras 12 e 13, e seção 6): o que o fotógrafo digita vira o
//! que entra no arquivo, ou a mensagem do campo.

use lamina_core::Project;

const CHECK_WHATSAPP: &str = "Confira o WhatsApp: use o código do país, o DDD e o número.";
const CHECK_EMAIL: &str = "Confira o e-mail.";
const NAME_MAX: usize = 60;
/// O código do Brasil, que a caixa acrescenta quando falta.
const BRAZIL: &str = "55";
/// Como o campo abre, sem número guardado: sem o código do país, o WhatsApp não abre a conversa.
pub const WHATSAPP_START: &str = "+55 ";

/// Só os dígitos, com o país; `Ok(None)` para o campo vazio, que não é erro.
pub fn clean_whatsapp(typed: &str) -> Result<Option<String>, &'static str> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Ok(None);
    }
    let digits: String = typed.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_start_matches('0');
    // Só o código do país, como o campo abre, é o mesmo que vazio.
    if typed.starts_with('+') && digits == BRAZIL {
        return Ok(None);
    }
    let clean = match (typed.starts_with('+'), digits.len()) {
        (true, 8..=15) | (false, 12..=15) => digits.to_owned(),
        (false, 10 | 11) => format!("{BRAZIL}{digits}"),
        _ => return Err(CHECK_WHATSAPP),
    };
    Ok(Some(clean))
}

/// Como a caixa mostra o número limpo: "+55 11 99999-9999" no Brasil; nos outros, `+` e os dígitos.
pub fn whatsapp_display(clean: &str) -> String {
    let brazilian = clean
        .strip_prefix(BRAZIL)
        .filter(|rest| matches!(rest.len(), 10 | 11));
    match brazilian {
        Some(rest) => {
            let (area, number) = rest.split_at(2);
            let (first, last) = number.split_at(number.len() - 4);
            format!("+{BRAZIL} {area} {first}-{last}")
        }
        None => format!("+{clean}"),
    }
}

/// Um `@`, algo dos dois lados e nenhum espaço; o campo vazio não é erro.
pub fn email_problem(typed: &str) -> Option<&'static str> {
    let typed = typed.trim();
    if typed.is_empty() {
        return None;
    }
    let good = !typed.contains(char::is_whitespace)
        && typed
            .split_once('@')
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && !b.contains('@'));
    (!good).then_some(CHECK_EMAIL)
}

/// O nome que o cliente vê vai de 1 a 60 caracteres.
pub fn client_name_problem(name: &str) -> Option<&'static str> {
    let count = name.trim().chars().count();
    (count == 0 || count > NAME_MAX).then_some("O nome que o cliente vê vai de 1 a 60 caracteres.")
}

/// O que impede de gerar com estas saídas ligadas (seção 6).
pub fn outputs_problem(project: &Project, jpg: bool, html: bool) -> Option<&'static str> {
    if !jpg && !html {
        return Some("Escolha pelo menos um: HTML ou JPG.");
    }
    (html && project.spreads.is_empty()).then_some("O HTML precisa de pelo menos uma lâmina.")
}
