//! A lista da página de novidades (spec PV): filtros, busca, destaques, o endereço do vídeo e a capa.
//! Nada aqui olha a hora nem o sistema da máquina (regra 15).

use std::borrow::Cow;

use super::page::{date_text_of, heading};
use super::{Illustration, NewsEntry, NewsItem, NewsKind, NewsPage, NewsSection, item_label};
use crate::shortcuts::fold;

/// O único começo de endereço que vira botão de vídeo (regra 8). A página do site redireciona, e
/// assim o destino final muda sem versão nova.
const VIDEO_SITE: &str = "https://alberthpalhares.github.io/lamina-app/v/";

/// O maior tamanho da capa na memória: nítida em tela de alta densidade (regra 10).
const COVER_WIDTH: u32 = 640;
const COVER_HEIGHT: u32 = 360;

/// Quantos cartões cabem na faixa de destaques (regra 5).
const MAX_HIGHLIGHTS: usize = 3;

/// O filtro da página (regra 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsFilter {
    All,
    New,
    Improvements,
    Video,
}

impl NewsFilter {
    /// O nome na fila de filtros.
    pub fn label(self) -> &'static str {
        match self {
            NewsFilter::All => "Todas",
            NewsFilter::New => "Novos recursos",
            NewsFilter::Improvements => "Melhorias",
            NewsFilter::Video => "Com vídeo",
        }
    }
}

/// Todas as linhas da página, na ordem das versões.
pub fn entries(page: &NewsPage) -> Vec<NewsEntry> {
    shown(page, NewsFilter::All, "")
}

/// Os filtros que têm alguma novidade na página; "Todas" vem sempre (regra 6).
pub fn available_filters(page: &NewsPage) -> Vec<NewsFilter> {
    [
        NewsFilter::All,
        NewsFilter::New,
        NewsFilter::Improvements,
        NewsFilter::Video,
    ]
    .into_iter()
    .filter(|f| {
        *f == NewsFilter::All
            || page
                .sections
                .iter()
                .flat_map(|s| &s.items)
                .any(|i| in_filter(i, *f))
    })
    .collect()
}

/// As linhas que passam no filtro e na busca. O cabeçalho da versão vai na primeira que aparece
/// dela (regra 6), e cada linha guarda o índice do item na lista completa.
pub fn shown(page: &NewsPage, filter: NewsFilter, query: &str) -> Vec<NewsEntry> {
    let words = fold(query);
    let words: Vec<&str> = words.split_whitespace().collect();
    let mut out = Vec::new();
    let mut index = 0;
    for section in &page.sections {
        let mut first = true;
        for item in items(section).iter() {
            if in_filter(item, filter) && matches(item, section, &words) {
                out.push(entry(page, section, item, index, first));
                first = false;
            }
            index += 1;
        }
    }
    out
}

/// Até 3 itens fixados, só da versão mais nova da página, com o índice na lista completa (regra 5).
pub fn highlights(page: &NewsPage) -> Vec<(usize, NewsEntry)> {
    let Some(newest) = page.sections.first() else {
        return Vec::new();
    };
    newest
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.pinned)
        .take(MAX_HIGHLIGHTS)
        .map(|(i, item)| (i, entry(page, newest, item, i, i == 0)))
        .collect()
}

/// A faixa de destaques só aparece com "Todas" e sem busca (regra 5).
pub fn shows_highlights(filter: NewsFilter, query: &str) -> bool {
    filter == NewsFilter::All && query.trim().is_empty()
}

/// A frase da busca sem resultado (regra 7).
pub fn no_results(query: &str) -> String {
    format!("Nenhuma novidade com “{}”.", query.trim())
}

/// O endereço, só se for uma página de vídeo do site do Lamina (regra 8).
pub fn video_address(text: &str) -> Option<&str> {
    let name = text.strip_prefix(VIDEO_SITE)?.strip_suffix(".html")?;
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    (!name.is_empty() && name.chars().all(valid)).then_some(text)
}

/// A capa de uma novidade: o primeiro quadro em até 640×360, sem ampliar (regra 10). `None` se o
/// arquivo não é imagem.
pub fn cover(bytes: &[u8]) -> Option<Illustration> {
    let full = image::load_from_memory(bytes).ok()?;
    let fits = full.width() <= COVER_WIDTH && full.height() <= COVER_HEIGHT;
    let picture = if fits {
        full.into_rgba8()
    } else {
        full.thumbnail(COVER_WIDTH, COVER_HEIGHT).into_rgba8()
    };
    Some(Illustration {
        width: picture.width(),
        height: picture.height(),
        rgba: picture.into_raw(),
    })
}

fn in_filter(item: &NewsItem, filter: NewsFilter) -> bool {
    match filter {
        NewsFilter::All => true,
        NewsFilter::New => item.kind == Some(NewsKind::New),
        NewsFilter::Improvements => item.kind == Some(NewsKind::Improvement),
        NewsFilter::Video => item.video.as_deref().and_then(video_address).is_some(),
    }
}

/// Todas as palavras aparecem no título, no texto, no "Experimente" ou na versão (regra 7).
fn matches(item: &NewsItem, section: &NewsSection, words: &[&str]) -> bool {
    if words.is_empty() {
        return true;
    }
    let text = [
        item.title.as_deref().unwrap_or_default(),
        item.text.as_str(),
        item.try_it.as_deref().unwrap_or_default(),
        section.version.to_string().as_str(),
    ]
    .join(" ");
    let text = fold(&text);
    words.iter().all(|w| text.contains(w))
}

/// Os itens da seção; uma versão só com a frase de abertura vira uma linha com ela.
fn items(section: &NewsSection) -> Cow<'_, [NewsItem]> {
    if section.items.is_empty() {
        Cow::Owned(vec![NewsItem {
            text: section.intro.clone(),
            ..NewsItem::default()
        }])
    } else {
        Cow::Borrowed(&section.items)
    }
}

fn entry(
    page: &NewsPage,
    section: &NewsSection,
    item: &NewsItem,
    index: usize,
    first: bool,
) -> NewsEntry {
    let this_version = page.current.as_ref() == Some(&section.version);
    NewsEntry {
        index,
        heading: first.then(|| heading(section)),
        intro: if first && !section.items.is_empty() {
            section.intro.clone()
        } else {
            String::new()
        },
        version: if first {
            section.version.to_string()
        } else {
            String::new()
        },
        date: if first {
            date_text_of(section)
        } else {
            String::new()
        },
        current: this_version && first,
        // Na lista, sem o ponto final do título ("Atalhos do teclado").
        label: item_label(item).trim_end_matches(['.', ':']).to_owned(),
        item: item.clone(),
    }
}
