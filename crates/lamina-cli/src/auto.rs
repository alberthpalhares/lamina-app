//! Comando `lamina auto`: importa uma pasta, monta o álbum com a montagem automática (spec AB) e desenha
//! todas as lâminas num SVG, com o relatório na tela. É para conferência; o comando final é
//! `lamina build` (pedaço CB).

mod exportar;
mod previa;
mod svg;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lamina_core::{AlbumSpec, Dpi, Edges, Mm, PhotoId};
use lamina_export::ExportOptions;
use lamina_layout::{BuildOptions, Reuse, auto_build};

use crate::importar::{import, project_from};
use crate::relatorio::print_build_report;
use crate::spec::edge;

/// Opções do `lamina auto` (comando de conferência dos pedaços AB, RD e EX).
#[derive(Debug, clap::Args)]
pub struct AutoArgs {
    /// Pasta com as fotos.
    pub pasta: PathBuf,
    /// Número de lâminas do álbum.
    #[arg(long)]
    pub laminas: usize,
    /// Quanto repetir o mesmo desenho: baixa (mais variedade), media ou alta.
    #[arg(long, default_value = "media")]
    pub reutilizacao: String,
    /// O miolo começa com uma página simples (só a direita).
    #[arg(long)]
    pub inicia_com_pagina: bool,
    /// O miolo termina com uma página simples (só a esquerda).
    #[arg(long)]
    pub termina_com_pagina: bool,
    /// Arquivo SVG a gravar.
    #[arg(long, default_value = "album.svg")]
    pub svg: PathBuf,
    /// Pasta onde gravar cada lâmina como PNG, com as fotos de verdade (spec renderizacao).
    #[arg(long)]
    pub previa: Option<PathBuf>,
    /// Pasta onde gravar os JPEGs de impressão (spec exportacao).
    #[arg(long)]
    pub exportar: Option<PathBuf>,
    /// Qualidade dos JPEGs exportados (1 a 100; padrão 95, a de impressão).
    #[arg(long, default_value_t = lamina_export::DEFAULT_QUALITY, requires = "exportar")]
    pub qualidade: u8,
    /// Quais lâminas exportar, ex.: "1, 3-4" (padrão: todas).
    #[arg(long, requires = "exportar")]
    pub laminas_exportar: Option<String>,
    /// Exportar um arquivo por página em vez de um por lâmina.
    #[arg(long, requires = "exportar")]
    pub paginas: bool,
    /// Substituir arquivos que já existem na pasta de exportação.
    #[arg(long, requires = "exportar")]
    pub sobrescrever: bool,
    /// Pasta do catálogo e das miniaturas (padrão: cache do sistema).
    #[arg(long)]
    pub cache: Option<PathBuf>,
}

pub fn run(args: &AutoArgs) -> Result<()> {
    let (report, _) = import(&args.pasta, args.cache.clone())?;
    let edges = Edges::new(edge(args.inicia_com_pagina), edge(args.termina_com_pagina));
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300)?)?
        .with_bleed(Mm(3.0))?
        .with_safe_margin(Mm(10.0))?
        .with_edges(edges.start, edges.end);
    let project = project_from(&report, "auto", spec);
    let options = BuildOptions {
        reuse: parse_reuse(&args.reutilizacao)?,
        ..BuildOptions::default()
    };
    let built = auto_build(&project, args.laminas, &options)?;
    // Miniatura de cada foto, para o SVG mostrar as fotos de verdade.
    let by_path: HashMap<&Path, &PathBuf> = report
        .entries
        .iter()
        .map(|e| (e.path.as_path(), &e.thumbnail))
        .collect();
    let thumbs: HashMap<PhotoId, PathBuf> = built
        .project
        .photos
        .iter()
        .filter_map(|p| by_path.get(p.path.as_path()).map(|t| (p.id, (*t).clone())))
        .collect();
    if let Some(dir) = &args.previa {
        let count = previa::write(&built.project, &thumbs, dir)?;
        println!("{count} prévias PNG em {}", dir.display());
    }
    if let Some(dir) = &args.exportar {
        let options = ExportOptions {
            quality: args.qualidade,
            range: args.laminas_exportar.clone(),
            pages: args.paginas,
            overwrite: args.sobrescrever,
            ..ExportOptions::default()
        };
        exportar::run(&built.project, &thumbs, dir, &options)?;
    }
    let drawing = svg::draw(&built, &thumbs, &args.pasta);
    std::fs::write(&args.svg, drawing)
        .with_context(|| format!("Não foi possível gravar {}.", args.svg.display()))?;
    print_build_report(&built);
    println!(
        "\n{} lâminas, {} fotos; desenho em {}",
        built.report.len(),
        built.project.photos.len(),
        args.svg.display()
    );
    Ok(())
}

/// "baixa", "media" ou "alta" → nível de reutilização.
pub fn parse_reuse(text: &str) -> Result<Reuse> {
    match text.trim().to_lowercase().as_str() {
        "baixa" => Ok(Reuse::Low),
        "media" | "média" => Ok(Reuse::Medium),
        "alta" => Ok(Reuse::High),
        other => anyhow::bail!("reutilização \"{other}\": use baixa, media ou alta"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_o_nivel_de_reutilizacao() {
        assert_eq!(parse_reuse("baixa").unwrap(), Reuse::Low);
        assert_eq!(parse_reuse("Média").unwrap(), Reuse::Medium);
        assert_eq!(parse_reuse("alta").unwrap(), Reuse::High);
        let err = parse_reuse("muita").unwrap_err();
        assert!(
            err.to_string().contains("use baixa, media ou alta"),
            "{err}"
        );
    }
}
