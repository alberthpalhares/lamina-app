//! O fundo em volta da lâmina (spec AJ, regra 13): cada fundo tem as cores do que é desenhado direto
//! sobre ele, medidas aqui, porque as do tema não passam em qualquer fundo.

use super::{Palette, pairs, ratio};

/// Os fundos de `ui/theme/backdrop.slint`: nome → cores.
fn backdrops(text: &str) -> Vec<(String, Palette)> {
    let mut out: Vec<(String, Palette)> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let t = line.trim();
        if let Some(name) = t
            .strip_prefix("out property <BackdropColors> ")
            .and_then(|r| r.strip_suffix(": {"))
        {
            out.push((name.to_owned(), Palette::new()));
            inside = true;
        } else if t == "};" {
            inside = false;
        } else if inside && let Some((_, colors)) = out.last_mut() {
            colors.extend(pairs(t));
        }
    }
    out
}

/// Os problemas dos fundos: texto e segundo plano a 4,5:1, e fios (e o anel de foco, que usa a cor do
/// texto) a 3:1.
pub fn problems(text: &str) -> Vec<String> {
    let all = backdrops(text);
    if all.len() != 3 {
        return vec![format!("fundos da lâmina: achei {}, e são 3", all.len())];
    }
    let mut out = Vec::new();
    for (name, colors) in &all {
        let get = |k: &str| colors.get(k).copied().unwrap_or([0, 0, 0]);
        for (fg, min) in [("text", 4.5), ("text-2", 4.5), ("line", 3.0)] {
            let r = ratio(get(fg), get("color"));
            if r < min {
                out.push(format!("fundo {name}: {fg} dá {r:.2}:1 (mínimo {min}:1)"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> String {
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../crates/lamina-app/ui/theme/backdrop.slint"
        );
        std::fs::read_to_string(file).unwrap()
    }

    #[test]
    fn aj_b18_contraste_do_que_fica_sobre_o_fundo() {
        assert_eq!(problems(&real()), Vec::<String>::new());
    }

    #[test]
    fn aj_b18_texto_branco_no_cinza_18_reprova() {
        let broken = real().replace(
            "color: #777777, text: #000000",
            "color: #777777, text: #ffffff",
        );
        let found = problems(&broken);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("fundo gray: text dá 4.48:1"), "{found:?}");
    }
}
