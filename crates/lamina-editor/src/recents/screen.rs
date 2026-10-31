//! O que a tela inicial mostra (spec TI, seções 4 e 5): os cartões, os botões do topo, o menu
//! "Abrir recente ▸" e a busca. Nada aqui lê os arquivos dos álbuns (TI-B13).

use std::path::{Path, PathBuf};

use super::{Recent, key};
use crate::Command;
use crate::shortcuts::fold;

/// Quantos álbuns o menu "Abrir recente ▸" mostra (regra 7).
pub const MENU_RECENTS: usize = 10;
/// A busca aparece com mais álbuns que isto (regra 8).
pub const SEARCH_FROM: usize = 8;

const MONTHS: [&str; 12] = [
    "jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.", "dez.",
];

/// O relógio da tela: agora (segundos desde 1970, UTC) e o fuso do computador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub now: i64,
    /// Segundos a leste do UTC (Brasília: −10 800).
    pub offset: i32,
}

impl Clock {
    /// A hora e o fuso deste computador.
    pub fn local() -> Self {
        let now = chrono::Local::now();
        Clock {
            now: now.timestamp(),
            offset: now.offset().local_minus_utc(),
        }
    }
}

/// Um cartão da tela inicial. `index` é a posição na lista, para os botões do cartão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub index: usize,
    pub name: String,
    /// A pasta do arquivo, para a busca e para a dica do cartão.
    pub folder: String,
    pub path: PathBuf,
    /// "hoje, 14:32", "ontem", "12 de out.".
    pub when: String,
    /// "18 lâminas · 240 fotos".
    pub counts: String,
    pub pinned: bool,
    /// O arquivo não existe mais (regra 4).
    pub missing: bool,
    /// O álbum não tem lâminas (spec BR): o cartão mostra o desenho do álbum em branco.
    pub blank: bool,
}

/// Uma linha do menu "Abrir recente ▸"; o arquivo que sumiu fica desabilitado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub index: usize,
    pub label: String,
    pub enabled: bool,
}

/// Os cartões, na ordem da tela: os fixados e depois os outros. `missing` são os arquivos que a
/// conferência de existência não achou.
pub fn cards(list: &[Recent], missing: &[PathBuf], clock: Clock) -> Vec<Card> {
    let gone: Vec<String> = missing.iter().map(|p| key(p)).collect();
    list.iter()
        .enumerate()
        .map(|(index, r)| Card {
            index,
            name: r.name.clone(),
            folder: folder_of(&r.path),
            path: r.path.clone(),
            when: when_text(r.opened_at, clock),
            counts: counts_text(r.spreads, r.photos),
            pinned: r.pinned,
            missing: gone.contains(&key(&r.path)),
            blank: r.spreads == 0,
        })
        .collect()
}

/// Os 10 primeiros cartões, para o menu (TI-B10).
pub fn recent_menu(cards: &[Card]) -> Vec<MenuEntry> {
    cards
        .iter()
        .take(MENU_RECENTS)
        .map(|c| MenuEntry {
            index: c.index,
            label: c.name.clone(),
            enabled: !c.missing,
        })
        .collect()
}

/// Os cartões cujo nome ou pasta têm `query`, sem diferença de acentos e de maiúsculas.
pub fn search<'a>(cards: &'a [Card], query: &str) -> Vec<&'a Card> {
    let query = fold(query.trim());
    cards
        .iter()
        .filter(|c| fold(&c.name).contains(&query) || fold(&c.folder).contains(&query))
        .collect()
}

/// A busca aparece com mais de 8 álbuns.
pub fn show_search(count: usize) -> bool {
    count > SEARCH_FROM
}

/// Onde ficam os botões da tela inicial, na ordem em que aparecem (spec DI, saídas 3 e 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WelcomeButtons {
    /// No corpo da tela; o primeiro é o principal.
    pub body: &'static [Command],
    /// Na barra de cima.
    pub bar: &'static [Command],
}

/// Sem recentes, o novo álbum é o principal, com o exemplo ao lado. Com recentes, o principal é
/// continuar o álbum em andamento e os três botões ficam na barra.
pub fn welcome_buttons(has_recents: bool) -> WelcomeButtons {
    if has_recents {
        WelcomeButtons {
            body: &[],
            bar: &[Command::OpenExample, Command::Open, Command::NewAlbum],
        }
    } else {
        WelcomeButtons {
            body: &[Command::NewAlbum, Command::OpenExample],
            bar: &[Command::Open],
        }
    }
}

/// "18 lâminas · 240 fotos".
pub fn counts_text(spreads: usize, photos: usize) -> String {
    let plural = |n: usize, word: &str| {
        if n == 1 {
            format!("{n} {word}")
        } else {
            format!("{n} {word}s")
        }
    };
    format!("{} · {}", plural(spreads, "lâmina"), plural(photos, "foto"))
}

/// Quando foi aberto, na hora do computador: "hoje, 14:32", "ontem", "12 de out." e, de outro
/// ano, "19 de out. de 2025".
pub fn when_text(opened_at: i64, clock: Clock) -> String {
    let Some(zone) = chrono::FixedOffset::east_opt(clock.offset) else {
        return String::new();
    };
    let local = |t: i64| chrono::DateTime::from_timestamp(t, 0).map(|d| d.with_timezone(&zone));
    let (Some(at), Some(now)) = (local(opened_at), local(clock.now)) else {
        return String::new();
    };
    let (day, today) = (at.date_naive(), now.date_naive());
    if day == today {
        return at.format("hoje, %H:%M").to_string();
    }
    if today.pred_opt() == Some(day) {
        return "ontem".to_owned();
    }
    use chrono::Datelike as _;
    let month = MONTHS
        .get(day.month0() as usize)
        .copied()
        .unwrap_or_default();
    if day.year() == today.year() {
        format!("{} de {month}", day.day())
    } else {
        format!("{} de {month} de {}", day.day(), day.year())
    }
}

/// A pasta de `file`, como o cartão mostra.
fn folder_of(file: &Path) -> String {
    file.parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}
