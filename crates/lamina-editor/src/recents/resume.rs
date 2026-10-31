//! O álbum em andamento e a grade da tela inicial (spec DI): quem vem grande, com o botão de
//! continuar, quem fica nos cartões embaixo e as frases de cada um. Nada aqui lê arquivos.

use super::Recent;
use super::screen::{Card, search};

/// A miniatura grande, só do álbum em andamento, no lado maior (regra 14).
pub const RESUME_SIDE: u32 = 1024;
/// A coluna mais estreita da grade e o vão entre elas (regra 3).
const MIN_COLUMN: f32 = 192.0;
const GAP: f32 = 16.0;
/// O conteúdo para em 1456 px; o limite só protege a conta de uma largura absurda.
const MAX_COLUMNS: u16 = 32;

/// O álbum em andamento. `index` é a posição na lista, como no cartão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resume {
    pub index: usize,
    /// "Você parou na lâmina 3 de 6"; vazia no álbum em branco.
    pub sentence: String,
    /// "Continuar na lâmina 3" ou, no álbum em branco, "Abrir".
    pub button: String,
    /// "6 lâminas · 16 fotos · aberto hoje, 16:44".
    pub meta: String,
}

/// O aberto por último cujo arquivo existe (regra 11). Com a busca preenchida, nenhum (regra 15).
pub fn resume(list: &[Recent], cards: &[Card], query: &str) -> Option<Resume> {
    if !query.trim().is_empty() {
        return None;
    }
    let card = cards
        .iter()
        .rev()
        .filter(|c| !c.missing)
        .max_by_key(|c| list.get(c.index).map_or(i64::MIN, |r| r.opened_at))?;
    let recent = list.get(card.index)?;
    let (sentence, button) = phrases(recent);
    Some(Resume {
        index: card.index,
        sentence,
        button,
        meta: format!("{} · {}", card.counts, opened_text(&card.when)),
    })
}

/// A frase de onde parou e o botão (regra 13). A lâmina guardada vale até a última que o álbum tem.
fn phrases(recent: &Recent) -> (String, String) {
    if recent.spreads == 0 {
        return (String::new(), "Abrir".to_owned());
    }
    let spread = (recent.current + 1).min(recent.spreads);
    (
        format!("Você parou na lâmina {spread} de {}", recent.spreads),
        format!("Continuar na lâmina {spread}"),
    )
}

/// "aberto hoje, 14:32", "aberto ontem" ou "aberto em 12 de out.".
fn opened_text(when: &str) -> String {
    if when.starts_with(|c: char| c.is_ascii_digit()) {
        format!("aberto em {when}")
    } else {
        format!("aberto {when}")
    }
}

/// Os cartões da grade: os que a busca acha, sem o álbum em andamento, na ordem da lista (os
/// fixados primeiro).
pub fn grid_cards<'a>(cards: &'a [Card], current: Option<&Resume>, query: &str) -> Vec<&'a Card> {
    search(cards, query)
        .into_iter()
        .filter(|c| current.is_none_or(|r| r.index != c.index))
        .collect()
}

/// Quantas colunas cabem em `area` pixels (regra 3).
pub fn grid_columns(area: f32) -> usize {
    let mut columns: u16 = 1;
    while columns < MAX_COLUMNS && f32::from(columns + 1) * (MIN_COLUMN + GAP) - GAP <= area {
        columns += 1;
    }
    usize::from(columns)
}

/// A altura da imagem do álbum em andamento numa janela de `window` pixels (regra 12).
pub fn resume_height(window: f32) -> f32 {
    if window < 600.0 {
        200.0
    } else if window <= 800.0 {
        272.0
    } else {
        400.0
    }
}

/// A frase da busca sem resultado (regra 6).
pub fn no_results(query: &str) -> String {
    format!("Nenhum álbum com “{}”.", query.trim())
}

/// O balão do cartão: o nome inteiro e a pasta (regra 5).
pub fn card_tip(card: &Card) -> String {
    format!("{} · {}", card.name, card.folder)
}

/// O nome do cartão para leitores de tela (regra 4).
pub fn card_label(card: &Card) -> String {
    if card.missing {
        format!("{}, não encontrado", card.name)
    } else {
        format!("{}, {}, {}", card.name, card.when, card.counts)
    }
}
