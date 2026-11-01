//! Trava do contraste (spec DF, regras 2 a 5): mede as paletas de `ui/theme/palette.slint` com a
//! conta da WCAG 2.x, em vez de confiar no olho. Também confere que os cinzas não têm matiz, que o
//! destaque é o ciano da marca e que as superfícies do tema escuro se distinguem.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

type Palette = BTreeMap<String, [u8; 3]>;

/// As paletas `dark` e `light` de um `palette.slint`: nome do token → cor.
pub fn palettes(text: &str) -> BTreeMap<String, Palette> {
    let mut out = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(name) = t
            .strip_prefix("out property <Colors> ")
            .and_then(|r| r.strip_suffix(": {"))
        {
            current = Some(name.to_owned());
            out.insert(name.to_owned(), Palette::new());
        } else if t == "};" {
            current = None;
        } else if let Some(palette) = current.as_ref().and_then(|n| out.get_mut(n)) {
            palette.extend(pairs(t));
        }
    }
    out
}

/// Os pares `token: #rrggbb` de uma linha (o alfa, se houver, é ignorado).
fn pairs(line: &str) -> Vec<(String, [u8; 3])> {
    line.split(',')
        .filter_map(|p| {
            let (name, value) = p.split_once(':')?;
            Some((name.trim().to_owned(), rgb(value.trim())?))
        })
        .collect()
}

fn rgb(hex: &str) -> Option<[u8; 3]> {
    let h = hex.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn luminance(c: [u8; 3]) -> f64 {
    let ch = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.040_45 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

/// Contraste WCAG entre duas cores (de 1 a 21).
pub fn ratio(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

const TAG_BACKGROUNDS: &[&str] = &["tag-new-bg", "tag-bg"];
const SURFACES: &[&str] = &["stage", "strip", "panel", "bar", "raised", "field"];
const NEUTRALS: &[&str] = &[
    "g1",
    "g2",
    "g3",
    "g4",
    "g5",
    "g6",
    "g7",
    "g8",
    "g9",
    "g10",
    "g11",
    "g12",
    "stage",
    "strip",
    "panel",
    "bar",
    "raised",
    "field",
    "line",
    "line-strong",
    "text",
    "text-2",
    "text-3",
    "text-strong",
    "text-disabled",
];

/// Todos os problemas de uma paleta (`theme` é "dark" ou "light", para as mensagens).
pub fn problems(theme: &str, p: &Palette, brand: [u8; 3]) -> Vec<String> {
    let mut out = Vec::new();
    let get = |k: &str| p.get(k).copied().unwrap_or([0, 0, 0]);
    let mut need = |fg: &str, bg: &str, min: f64| {
        let r = ratio(get(fg), get(bg));
        if r < min {
            out.push(format!(
                "{theme}: {fg} sobre {bg} dá {r:.2}:1 (mínimo {min}:1)"
            ));
        }
    };
    for bg in SURFACES {
        for fg in ["text", "text-2"] {
            need(fg, bg, 4.5);
        }
        for fg in ["text-3", "line-strong", "focus"] {
            need(fg, bg, 3.0);
        }
        need("link", bg, 4.5);
        for fg in ["warning", "error", "success"] {
            need(fg, bg, 3.0);
        }
    }
    for bg in TAG_BACKGROUNDS {
        need("tag-text", bg, 4.5);
    }
    need("on-accent", "accent", 4.5);
    need("on-accent", "accent-hover", 4.5);
    out.extend(hue_problems(theme, p));
    if get("accent") != brand {
        out.push(format!("{theme}: o destaque não é o ciano da marca"));
    }
    out
}

fn hue_problems(theme: &str, p: &Palette) -> Vec<String> {
    NEUTRALS
        .iter()
        .filter(|k| {
            p.get(**k).is_some_and(|c| {
                let (hi, lo) = (c.iter().max().unwrap_or(&0), c.iter().min().unwrap_or(&0));
                hi - lo > 1
            })
        })
        .map(|k| format!("{theme}: {k} tem matiz (R, G e B diferentes)"))
        .collect()
}

/// Fundos de etiqueta com alfa: a conta de contraste ignora o alfa, então eles precisam ser opacos
/// (spec PV, regra 13).
pub fn alpha_problems(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut theme = "";
    for line in text.lines().map(str::trim) {
        if let Some(name) = line
            .strip_prefix("out property <Colors> ")
            .and_then(|r| r.strip_suffix(": {"))
        {
            theme = name;
        }
        for (name, value) in line.split(',').filter_map(|p| p.split_once(':')) {
            let name = name.trim();
            let value = value.trim();
            if TAG_BACKGROUNDS.contains(&name)
                && value.starts_with('#')
                && value.len() != "#rrggbb".len()
            {
                out.push(format!(
                    "{theme}: {name} tem alfa: o fundo da etiqueta precisa ser opaco"
                ));
            }
        }
    }
    out
}

/// Palco < faixa < painel em claridade, com pelo menos 1,1:1 entre vizinhas (regra 5 da DF), nos
/// dois temas.
pub fn surface_problems(theme: &str, p: &Palette) -> Vec<String> {
    let get = |k: &str| p.get(k).copied().unwrap_or([0, 0, 0]);
    let order = ["stage", "strip", "panel"];
    order
        .windows(2)
        .filter(|w| {
            let (a, b) = (get(w[0]), get(w[1]));
            luminance(a) >= luminance(b) || ratio(a, b) < 1.1
        })
        .map(|w| {
            format!(
                "{theme}: {} e {} não se distinguem (mínimo 1,1:1)",
                w[0], w[1]
            )
        })
        .collect()
}

/// Tokens que existem numa paleta e faltam na outra.
pub fn missing(a: &Palette, b: &Palette) -> Vec<String> {
    let left = a.keys().filter(|k| !b.contains_key(*k));
    let right = b.keys().filter(|k| !a.contains_key(*k));
    left.chain(right).cloned().collect()
}

/// O ciano da marca (claro, escuro), lido da tabela de cores de `docs/marca/guia.md`.
pub fn brand_cyan(guide: &str) -> Option<([u8; 3], [u8; 3])> {
    let row = guide.lines().find(|l| l.contains("Ciano de processo"))?;
    let hex: Vec<_> = row.split('`').filter_map(rgb).collect();
    Some((*hex.first()?, *hex.get(1)?))
}

pub fn run() -> bool {
    let text = fs::read_to_string("crates/lamina-app/ui/theme/palette.slint").unwrap_or_default();
    let all = palettes(&text);
    let (Some(dark), Some(light)) = (all.get("dark"), all.get("light")) else {
        eprintln!("  contraste: não achei as paletas");
        return false;
    };
    let accent = |p: &Palette| p.get("accent").copied().unwrap_or_default();
    // No repositório público (só o código) não há o guia da marca: fica só a comparação do destaque.
    let (light_cyan, dark_cyan) = if Path::new("docs").is_dir() {
        let guide = fs::read_to_string("docs/marca/guia.md").unwrap_or_default();
        let Some(cyan) = brand_cyan(&guide) else {
            eprintln!("  contraste: não achei o ciano da marca");
            return false;
        };
        cyan
    } else {
        (accent(light), accent(dark))
    };
    let mut found = problems("dark", dark, dark_cyan);
    found.extend(problems("light", light, light_cyan));
    found.extend(surface_problems("dark", dark));
    found.extend(surface_problems("light", light));
    found.extend(alpha_problems(&text));
    found.extend(
        missing(dark, light)
            .into_iter()
            .map(|k| format!("token só numa paleta: {k}")),
    );
    for f in &found {
        eprintln!("  {f}");
    }
    if found.is_empty() {
        println!("  contraste ok");
    }
    found.is_empty()
}

#[cfg(test)]
mod tests;
