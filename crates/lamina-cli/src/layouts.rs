//! Comando `lamina layouts`: gera as duas listas de sugestões (spec VL) e desenha num SVG para
//! conferência.

mod svg;

use std::path::Path;

use anyhow::{Context, Result};
use lamina_core::{ActivePages, AlbumSpec, DesignMode, Dpi, Mm, PageSide};
use lamina_layout::{
    Align, Arrangement, Family, FullBleed, LayoutCandidate, LayoutOptions, PageLayout, Scope,
    SupportSide, layout_options,
};

/// "H,V,Q,16:9" → proporções.
pub fn parse_aspects(text: &str) -> Result<Vec<f64>> {
    text.split(',')
        .map(|item| match item.trim().to_uppercase().as_str() {
            "H" => Ok(1.5),
            "V" => Ok(2.0 / 3.0),
            "Q" => Ok(1.0),
            ratio => {
                let (w, h) = ratio
                    .split_once(':')
                    .with_context(|| format!("foto \"{item}\": use H, V, Q ou L:A (ex.: 16:9)"))?;
                Ok(w.trim().parse::<f64>()? / h.trim().parse::<f64>()?)
            }
        })
        .collect()
}

pub struct Args<'a> {
    pub aspects: &'a [f64],
    pub width_cm: f64,
    pub height_cm: f64,
    pub safe_mm: f64,
    pub page_mode: bool,
    pub single_page: bool,
    pub svg: &'a Path,
    /// Quantas opções desenhar de cada lista.
    pub max: usize,
}

pub fn run(args: &Args<'_>) -> Result<()> {
    let mode = if args.page_mode {
        DesignMode::Page
    } else {
        DesignMode::Spread
    };
    let spec = AlbumSpec::new(
        Mm::from_cm(args.width_cm),
        Mm::from_cm(args.height_cm),
        Dpi::new(300)?,
    )?
    .with_safe_margin(Mm(args.safe_mm))?
    .with_mode(mode);
    let active = ActivePages {
        left: !args.single_page,
        right: true,
    };
    let lists = layout_options(&spec, active, args.aspects, &LayoutOptions::default())?;
    let max = args.max.max(1);
    let sections = [
        ("Na ordem", &lists.in_order[..lists.in_order.len().min(max)]),
        (
            "Reordenadas",
            &lists.reordered[..lists.reordered.len().min(max)],
        ),
    ];
    std::fs::write(args.svg, svg::draw(&spec, active, args.aspects, &sections))
        .with_context(|| format!("Não foi possível gravar {}.", args.svg.display()))?;
    println!(
        "{} na ordem + {} reordenadas; até {max} de cada em {}",
        lists.in_order.len(),
        lists.reordered.len(),
        args.svg.display()
    );
    for (i, c) in lists.in_order.iter().take(12).enumerate() {
        println!(
            "  #{:<3} nota {:>5.1}  {}",
            i + 1,
            c.score.total,
            describe(c)
        );
    }
    Ok(())
}

/// Família e respiro em português, para a legenda.
pub fn describe(c: &LayoutCandidate) -> String {
    let family = family_name(&c.family);
    let margin = ["", ", margem +8%", ", margem +16%", ", margem +24%"]
        .get(usize::from(c.breathing.margin_level))
        .copied()
        .unwrap_or_default();
    let align = match c.breathing.align {
        Align::Center => "",
        Align::Start => ", encostado na borda",
        Align::End => ", encostado do outro lado",
    };
    format!("{family}{margin}{align}")
}

/// Família em português, sem o respiro.
pub fn family_name(family: &Family) -> String {
    match family {
        Family::Across => "atravessada".to_owned(),
        Family::PerPage { left, right } => {
            format!(
                "por página: esq. {} | dir. {}",
                page(left.as_ref()),
                page(right.as_ref())
            )
        }
        Family::WholeSpread(p) => format!("lâmina inteira, {}", layout(p)),
        Family::Hero {
            scope,
            support,
            layout: p,
        } => format!(
            "destaque {}, apoio {} {}",
            scope_name(*scope),
            support_name(*support),
            layout(p)
        ),
        Family::Mirrored(p) => format!("espelhado, {}", layout(p)),
        Family::BleedPage { side, rest } => {
            format!(
                "página {} sangrada, outra {}",
                side_name(*side),
                page(rest.as_ref())
            )
        }
        Family::Mosaic { scope, layout: p } => {
            format!("mosaico {}, {}", scope_name(*scope), layout(p))
        }
        Family::FullBleed(FullBleed::Pages { left, right }) => {
            format!(
                "sem borda, esq. {} | dir. {}",
                page(left.as_ref()),
                page(right.as_ref())
            )
        }
        Family::FullBleed(FullBleed::Spread(p)) => format!("sem borda, lâmina {}", layout(p)),
        Family::FullBleed(FullBleed::Hero { support, layout: p }) => format!(
            "sem borda, destaque com apoio {} {}",
            support_name(*support),
            layout(p)
        ),
        Family::Model { name, mirrored, .. } => {
            let e = if *mirrored { ", espelhado" } else { "" };
            format!("modelo {name}{e}")
        }
    }
}

fn layout(p: &PageLayout) -> String {
    let kind = match p.arrangement {
        Arrangement::Rows => "linhas",
        Arrangement::Columns => "colunas",
    };
    format!("{kind} {:?}", p.groups)
}

fn page(p: Option<&PageLayout>) -> String {
    p.map_or_else(|| "vazia".to_owned(), layout)
}

fn side_name(side: PageSide) -> &'static str {
    match side {
        PageSide::Left => "esquerda",
        PageSide::Right => "direita",
    }
}

fn scope_name(scope: Scope) -> String {
    match scope {
        Scope::Page(side) => format!("na página {}", side_name(side)),
        Scope::Spread => "na lâmina".to_owned(),
    }
}

fn support_name(side: SupportSide) -> &'static str {
    match side {
        SupportSide::Left => "à esquerda",
        SupportSide::Right => "à direita",
        SupportSide::Below => "embaixo",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_as_proporcoes() {
        let a = parse_aspects("H, v,Q,16:9").unwrap();
        assert_eq!(a.len(), 4);
        assert!((a[0] - 1.5).abs() < 1e-12 && (a[3] - 16.0 / 9.0).abs() < 1e-12);
    }

    #[test]
    fn proporcao_invalida_explica_o_formato() {
        let err = parse_aspects("H,X").unwrap_err();
        assert!(err.to_string().contains("use H, V, Q ou L:A"), "{err}");
    }

    #[test]
    fn descreve_familia_e_respiro() {
        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_safe_margin(Mm(10.0))
            .unwrap();
        let both = ActivePages {
            left: true,
            right: true,
        };
        let lists = layout_options(&spec, both, &[1.5, 1.5], &LayoutOptions::default()).unwrap();
        let texts: Vec<String> = lists.in_order.iter().map(describe).collect();
        assert!(
            texts.iter().any(|t| t.starts_with("lâmina inteira")),
            "{texts:?}"
        );
        assert!(texts.iter().any(|t| t.contains("margem +16%")), "{texts:?}");
    }
}
