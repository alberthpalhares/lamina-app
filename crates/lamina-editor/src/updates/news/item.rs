//! Um item das novidades (spec NV, regra 2): o título, o texto, o "Experimente" e, num comentário
//! que a página da versão não mostra, a ilustração, o comando do "Experimente agora" e os campos da
//! página de novidades (spec PV, seção 3).

use std::collections::BTreeMap;

use super::{NewsItem, NewsKind, NewsSystem};

/// Um item: o `**título**` do começo, o texto e o que vem depois de "Experimente:".
pub(super) fn item(block: &str) -> NewsItem {
    let (block, fields) = extras(block);
    let block = block.as_str();
    let (title, rest) = match block.strip_prefix("**").and_then(|b| b.split_once("**")) {
        Some((title, rest)) => (Some(clean(title)), rest),
        None => (None, block),
    };
    let rest = clean(rest);
    let (text, try_it) = match rest.split_once("Experimente:") {
        Some((text, try_it)) => (text.trim().to_owned(), Some(try_it.trim().to_owned())),
        None => (rest, None),
    };
    NewsItem {
        title,
        text,
        try_it: try_it.filter(|t| !t.is_empty()),
        image: fields.owned("imagem"),
        command: fields.owned("comando"),
        kind: fields.kind(),
        video: fields.owned("video"),
        pinned: fields.get("fixada") == Some("sim"),
        only: fields.only(),
    }
}

/// Os campos do comentário no fim do item (spec PV, seção 3), em qualquer ordem. Campo desconhecido
/// fica aqui e ninguém lê; repetido, vale o primeiro.
#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl Fields {
    fn read(inside: &str) -> Self {
        let mut fields = BTreeMap::new();
        for (key, value) in inside.split(';').filter_map(|pair| pair.split_once(':')) {
            let value = value.trim();
            if !value.is_empty() {
                fields
                    .entry(key.trim().to_owned())
                    .or_insert_with(|| value.to_owned());
            }
        }
        Fields(fields)
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    fn owned(&self, key: &str) -> Option<String> {
        self.get(key).map(str::to_owned)
    }

    /// `tipo`, `fixada` e `so` com valor desconhecido contam como ausentes (regra 2).
    fn kind(&self) -> Option<NewsKind> {
        match self.get("tipo")? {
            "novo" => Some(NewsKind::New),
            "melhoria" => Some(NewsKind::Improvement),
            _ => None,
        }
    }

    fn only(&self) -> Option<NewsSystem> {
        match self.get("so")? {
            "windows" => Some(NewsSystem::Windows),
            "mac" => Some(NewsSystem::Mac),
            "linux" => Some(NewsSystem::Linux),
            _ => None,
        }
    }
}

/// Tira do item o comentário `<!-- imagem: x.png; comando: y -->`, que a página da versão não
/// mostra, e devolve os campos dele.
fn extras(block: &str) -> (String, Fields) {
    let Some((before, rest)) = block.split_once("<!--") else {
        return (block.to_owned(), Fields::default());
    };
    let (inside, after) = rest.split_once("-->").unwrap_or((rest, ""));
    let text = format!("{} {}", before.trim_end(), after.trim_start());
    (text.trim().to_owned(), Fields::read(inside))
}

/// Tira as marcas do Markdown: `**`, `*`, `_`, as crases e o endereço de `[texto](endereço)`.
pub(super) fn clean(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let link = rest[open + 1..]
            .split_once("](")
            .and_then(|(label, after)| Some((label, after.split_once(')')?.1)));
        out.push_str(&rest[..open]);
        match link {
            Some((label, after)) => {
                out.push_str(label);
                rest = after;
            }
            None => {
                out.push('[');
                rest = &rest[open + 1..];
            }
        }
    }
    out.push_str(rest);
    out.retain(|c| !matches!(c, '*' | '_' | '`'));
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
