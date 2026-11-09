//! Textos do painel de layouts: o nome da família e a nota explicada (spec TL, seção 4).

use lamina_core::Project;
use lamina_layout::{Family, FamilyKind, Score};

/// Aviso do painel quando todas as opções estão rejeitadas (spec TL, regra 7).
pub const ALL_REJECTED: &str = "Todas as opções desta seleção foram rejeitadas.";

/// Nome da família em português.
pub fn family_name(kind: FamilyKind) -> &'static str {
    match kind {
        FamilyKind::Across => "Atravessada",
        FamilyKind::PerPage => "Por página",
        FamilyKind::WholeSpread => "Lâmina inteira",
        FamilyKind::Hero => "Destaque com apoio",
        FamilyKind::Mirrored => "Espelhado",
        FamilyKind::BleedPage => "Página sangrada",
        FamilyKind::Mosaic => "Mosaico",
        FamilyKind::FullBleed => "Sem borda",
        FamilyKind::Model => "Modelo",
    }
}

/// Título da opção no painel: o nome da família, ou "Modelo · <nome>" (com "· espelhado") para um
/// modelo do fotógrafo (spec MD, seção 4).
pub fn option_title(family: &Family) -> String {
    match family {
        Family::Model {
            name,
            mirrored,
            factory,
        } => {
            let e = if *mirrored { " · espelhado" } else { "" };
            if *factory {
                format!("Modelo de fábrica · {name}{e}")
            } else {
                format!("Modelo · {name}{e}")
            }
        }
        other => family_name(other.kind()).to_owned(),
    }
}

// As partes da nota ficam entre −15 e 65: cabem com folga em i64.
#[allow(clippy::cast_possible_truncation)]
fn points(v: f64) -> i64 {
    v.round() as i64
}

/// "Nota 82 = aproveitamento 45 + menor foto 22 + equilíbrio 20 − recorte 5". A nota mostrada é
/// a soma das partes arredondadas, para a conta fechar (TL-B10).
pub fn score_text(score: &Score) -> String {
    let (c, s, b) = (
        points(score.coverage),
        points(score.smallest),
        points(score.balance),
    );
    let crop = points(-score.crop);
    format!(
        "Nota {} = aproveitamento {c} + menor foto {s} + equilíbrio {b} − recorte {crop}",
        score_total(score)
    )
}

/// A nota mostrada: a soma das partes arredondadas, a mesma de [`score_text`] (TL-B10).
pub fn score_total(score: &Score) -> i64 {
    points(score.coverage) + points(score.smallest) + points(score.balance) - points(-score.crop)
}

/// "#F5F0E6" → a cor; `None` se não for "#" e seis dígitos hexadecimais (spec AP, seção 6).
pub fn parse_color(text: &str) -> Option<lamina_core::Rgb> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(lamina_core::Rgb {
        r: byte(0)?,
        g: byte(2)?,
        b: byte(4)?,
    })
}

/// A cor como "#RRGGBB".
pub fn color_hex(c: lamina_core::Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// Paleta do painel de aparência: neutros e cores comuns em álbuns (spec AP, seção 3).
pub const PALETTE: [(&str, lamina_core::Rgb); 12] = {
    use lamina_core::Rgb;
    [
        (
            "Branco",
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
        ),
        (
            "Marfim",
            Rgb {
                r: 250,
                g: 246,
                b: 238,
            },
        ),
        (
            "Creme",
            Rgb {
                r: 245,
                g: 240,
                b: 230,
            },
        ),
        (
            "Areia",
            Rgb {
                r: 216,
                g: 200,
                b: 168,
            },
        ),
        (
            "Rosé",
            Rgb {
                r: 232,
                g: 207,
                b: 203,
            },
        ),
        (
            "Cinza claro",
            Rgb {
                r: 229,
                g: 229,
                b: 229,
            },
        ),
        (
            "Cinza",
            Rgb {
                r: 154,
                g: 154,
                b: 154,
            },
        ),
        (
            "Grafite",
            Rgb {
                r: 60,
                g: 60,
                b: 60,
            },
        ),
        ("Preto", Rgb { r: 0, g: 0, b: 0 }),
        (
            "Azul-marinho",
            Rgb {
                r: 31,
                g: 42,
                b: 68,
            },
        ),
        (
            "Verde-oliva",
            Rgb {
                r: 85,
                g: 107,
                b: 47,
            },
        ),
        (
            "Vinho",
            Rgb {
                r: 90,
                g: 30,
                b: 43,
            },
        ),
    ]
};

/// Aviso do campo de cor inválida.
pub const BAD_COLOR: &str = "Use uma cor como #F5F0E6.";

/// Ampliação a partir da qual o texto vira aviso (a mesma da pré-impressão).
const WARN_PERCENT: f64 = 120.0;

/// "DPI efetivo 225 · ampliada 133%" do quadro `frame` da lâmina `index`, e se é aviso
/// (spec RC, regra 7). `None` se o quadro não tem foto.
pub fn frame_dpi_text(project: &Project, index: usize, frame: usize) -> Option<(String, bool)> {
    let f = project.spreads.get(index)?.frames.get(frame)?;
    let dpi = project.effective_dpi(f).ok()??;
    let percent = f64::from(project.spec.dpi().get()) / dpi * 100.0;
    let text = format!(
        "DPI efetivo {} · ampliada {}%",
        points(dpi),
        points(percent)
    );
    Some((text, percent > WARN_PERCENT))
}
