//! O que a página "O que há de novo" abre (specs NV e PV) e o aviso da release quando falta o
//! "Experimente" (NV, regra 9).

use super::{NewsItem, NewsSection, date_text, since, until, without_try};
use crate::updates::Version;

/// Pela Ajuda, sem nenhuma seção até a versão que roda.
pub const NO_NEWS: &str = "Ainda não há notas desta versão.";

/// A página: o título, o subtítulo, as versões e, pela Ajuda, a versão marcada "Esta versão".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsPage {
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<NewsSection>,
    pub current: Option<Version>,
    /// "Ainda não há notas desta versão." quando não há seção; vazio quando há.
    pub empty: String,
}

/// Depois de atualizar (regras 3 e 4): as versões que o fotógrafo ainda não viu. `None` quando
/// não há nenhuma, e aí a janela não abre.
pub fn after_update(
    sections: &[NewsSection],
    previous: &Version,
    current: &Version,
) -> Option<NewsPage> {
    let shown = since(sections, previous, current);
    (!shown.is_empty()).then(|| NewsPage {
        title: format!("Lamina atualizado para {current}"),
        subtitle: format!(
            "Veja o que mudou desde a {previous}, a versão que você tinha, e experimente:"
        ),
        sections: shown,
        current: None,
        empty: String::new(),
    })
}

/// Ajuda → "O que há de novo…" (regra 5): todas as versões até a que roda, a atual marcada.
pub fn from_help(sections: &[NewsSection], current: &Version) -> NewsPage {
    let shown = until(sections, current);
    NewsPage {
        title: "O que há de novo no Lamina".to_owned(),
        subtitle: String::new(),
        empty: if shown.is_empty() {
            NO_NEWS.to_owned()
        } else {
            String::new()
        },
        sections: shown,
        current: Some(current.clone()),
    }
}

/// O cabeçalho de uma versão: "Versão 0.1.0-beta.5 · 24/10/2026".
pub fn heading(section: &NewsSection) -> String {
    match &section.date {
        Some(date) => format!("Versão {} · {}", section.version, date_text(date)),
        None => format!("Versão {}", section.version),
    }
}

/// A data da seção como a página mostra: "24/10/2026"; vazia quando a seção não tem.
pub(super) fn date_text_of(section: &NewsSection) -> String {
    section.date.as_deref().map(date_text).unwrap_or_default()
}

/// O aviso da release (regra 9): "1 item da 0.1.0-beta.5 sem “Experimente:”: «Programa
/// menor.»". Item sem título aparece pelas primeiras palavras. `None` quando todos têm.
pub fn missing_try(section: &NewsSection) -> Option<String> {
    let missing = without_try(section);
    if missing.is_empty() {
        return None;
    }
    let names: Vec<String> = missing
        .iter()
        .map(|i| format!("«{}»", item_label(i)))
        .collect();
    let count = match missing.len() {
        1 => "1 item".to_owned(),
        n => format!("{n} itens"),
    };
    Some(format!(
        "{count} da {} sem “Experimente:”: {}",
        section.version,
        names.join(", ")
    ))
}

/// O nome de uma novidade na lista e nos avisos: o título ou, sem título, as primeiras palavras.
pub fn item_label(item: &NewsItem) -> String {
    item.title
        .clone()
        .unwrap_or_else(|| first_words(&item.text))
}

/// Uma linha da página (spec PV, seção 4): a novidade e, na primeira que aparece de cada versão, a
/// data, o número da versão e a frase de abertura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsEntry {
    /// A posição dela na lista completa da página: não muda com o filtro nem com a busca.
    pub index: usize,
    pub heading: Option<String>,
    pub intro: String,
    /// O número da versão e a data, só na linha que leva o cabeçalho.
    pub version: String,
    pub date: String,
    /// A versão que roda, marcada "Esta versão" (só pela Ajuda).
    pub current: bool,
    pub label: String,
    pub item: NewsItem,
}

/// A ilustração decodificada, em RGBA, para a janela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Illustration {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decodifica uma ilustração embutida (PNG ou JPEG). `None` se o arquivo não é imagem.
pub fn illustration(bytes: &[u8]) -> Option<Illustration> {
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    Some(Illustration {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

/// As cinco primeiras palavras, com reticências quando o texto continua.
fn first_words(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= 5 {
        return words.join(" ");
    }
    format!("{}…", words[..5].join(" "))
}
