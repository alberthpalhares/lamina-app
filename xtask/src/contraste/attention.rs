//! A barra de atualização (spec AV, regras 6 e 7): amarelo com letras pretas, nos dois temas, e só
//! nela. É a única exceção ao estilo sóbrio, então a trava confere também onde o token é usado.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Palette, ratio, rgb};

const BAR: &str = "update_bar.slint";
/// Quem pode citar os tokens: a paleta que os define, a barra e a galeria que a mostra.
const ALLOWED: &[&str] = &["palette.slint", BAR, "gallery.slint"];

/// As cores do global `Fixed` de um `palette.slint`: nome do token → cor.
pub fn fixed(text: &str) -> Palette {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("out property <color> ")?;
            let (name, value) = rest.strip_suffix(';')?.split_once(':')?;
            Some((name.trim().to_owned(), rgb(value.trim())?))
        })
        .collect()
}

/// Texto e botões: letra sobre fundo, no repouso e sob o mouse. O anel de foco é o preto sobre o
/// amarelo: com 4,5:1, já passa dos 3:1 que ele pede.
const PAIRS: &[(&str, &str)] = &[
    ("on-attention", "attention"),
    ("on-attention", "attention-hover"),
    ("attention", "on-attention"),
    ("attention", "on-attention-hover"),
];

/// Os problemas das cores: amarelo, preto e os contrastes do texto, dos botões e do foco.
pub fn color_problems(fixed: &Palette) -> Vec<String> {
    let mut out = Vec::new();
    for token in PAIRS.iter().flat_map(|(a, b)| [a, b]) {
        if !fixed.contains_key(*token) && !out.contains(&format!("falta o token {token}")) {
            out.push(format!("falta o token {token}"));
        }
    }
    if !out.is_empty() {
        return out;
    }
    let [r, g, b] = fixed["attention"];
    if r < 0xe0 || g < 0xc0 || b > 0x40 {
        out.push("attention não é amarelo".to_owned());
    }
    if fixed["on-attention"] != [0, 0, 0] {
        out.push("on-attention não é preto".to_owned());
    }
    for (fg, bg) in PAIRS {
        let found = ratio(fixed[*fg], fixed[*bg]);
        if found < 4.5 {
            out.push(format!("{fg} sobre {bg} dá {found:.2}:1 (mínimo 4,5:1)"));
        }
    }
    out
}

/// Os problemas de uso: a barra sem `Theme`, e os tokens em nenhuma outra tela. `files` são os
/// `.slint`, como (nome do arquivo, texto).
pub fn use_problems(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (name, text) in files {
        let code: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        if name == BAR {
            if code.contains("Theme") {
                out.push(format!(
                    "{BAR} usa Theme: as cores da barra não mudam com o tema"
                ));
            }
            if !code.contains("Fixed.attention") {
                out.push(format!("{BAR} não usa o token attention"));
            }
        } else if code.contains("attention") && !ALLOWED.contains(&name.as_str()) {
            out.push(format!("attention fora da barra de atualização: {name}"));
        }
    }
    out
}

/// Os `.slint` de uma pasta, com as subpastas.
fn slint_files(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            slint_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "slint") {
            out.push(path);
        }
    }
}

/// Tudo o que a trava reprova, lendo a interface do repositório.
pub fn problems(palette: &str, ui: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    slint_files(ui, &mut paths);
    let files: Vec<(String, String)> = paths
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            Some((name, fs::read_to_string(p).ok()?))
        })
        .collect();
    let mut found = color_problems(&fixed(palette));
    found.extend(use_problems(&files));
    found
}
