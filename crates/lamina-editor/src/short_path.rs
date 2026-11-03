//! O caminho curto dos diálogos (spec DD, regra 17): corta o meio, onde há menos a dizer, e mantém
//! o começo e o fim, onde está o nome.

const CUT: char = '…';

fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

fn count(text: &str) -> usize {
    text.chars().count()
}

/// O caminho com no máximo `limit` letras. O começo fica até a primeira barra (`C:\`); do fim, entra
/// o nome do arquivo e tantas pastas quantas couberem. O caminho que cabe aparece inteiro.
pub fn short_path(path: &str, limit: usize) -> String {
    if count(path) <= limit {
        return path.to_owned();
    }
    let head_end = path
        .char_indices()
        .find(|(_, c)| is_separator(*c))
        .map_or(0, |(i, c)| i + c.len_utf8());
    let (head, rest) = path.split_at(head_end);
    let room = limit.saturating_sub(count(head) + 1);
    let mut tail = last_chars(rest, room);
    // O corte cai numa barra sempre que dá: uma pasta pela metade não diz nada.
    if let Some(i) = tail.find(is_separator) {
        tail.drain(..i);
    }
    format!("{head}{CUT}{tail}")
}

fn last_chars(text: &str, how_many: usize) -> String {
    let skip = count(text).saturating_sub(how_many);
    text.chars().skip(skip).collect()
}
