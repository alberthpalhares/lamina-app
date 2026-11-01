//! `lamina exportar <projeto.lamina>` (spec CB, regra 7): reabre um projeto salvo e exporta de
//! novo. É a prova de que o arquivo do projeto funciona.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use lamina_export::{ExportOptions, open_project, parse_range, preflight};

use lamina_media::original_source_in;

use crate::relatorio::{export_with_progress, print_warnings};

#[derive(Debug, Args)]
pub struct ReopenArgs {
    /// Arquivo do projeto (`.lamina`).
    pub projeto: PathBuf,
    /// Pasta onde gravar os JPEGs de impressão.
    #[arg(long)]
    pub saida: PathBuf,
    /// Qualidade dos JPEGs (1 a 100; padrão 95, a de impressão).
    #[arg(long, default_value_t = lamina_export::DEFAULT_QUALITY)]
    pub qualidade: u8,
    /// Quais lâminas exportar, ex.: "1, 3-4" (padrão: todas).
    #[arg(long)]
    pub laminas_exportar: Option<String>,
    /// Um arquivo por página em vez de um por lâmina.
    #[arg(long)]
    pub paginas: bool,
    /// Substituir os JPEGs que já existem na saída.
    #[arg(long)]
    pub sobrescrever: bool,
    #[command(flatten)]
    pub saida_formato: crate::formato::SaidaArgs,
}

pub fn run(args: &ReopenArgs) -> Result<()> {
    args.saida_formato.check_marks()?;
    let project = open_project(&args.projeto)?;
    // O que o projeto lembra vale quando nada foi digitado (spec PR, regra 5).
    let prefs = project.export;
    let (format, crop_marks) = args.saida_formato.options_with(prefs)?;
    let options = ExportOptions {
        quality: args.qualidade,
        range: args.laminas_exportar.clone(),
        pages: args.paginas || prefs.pages,
        overwrite: args.sobrescrever,
        format,
        crop_marks,
        space: args.saida_formato.space_with(prefs),
        cover: lamina_export::CoverExport::default(),
    };
    let spreads = match &options.range {
        Some(text) => parse_range(text, project.spreads.len())?,
        None => (0..project.spreads.len()).collect(),
    };
    print_warnings(&preflight(&project, &spreads));
    let photos = original_source_in(options.space);
    export_with_progress(&project, &args.saida, &photos, &options)?;
    Ok(())
}
