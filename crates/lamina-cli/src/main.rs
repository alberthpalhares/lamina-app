//! `lamina`: o Lamina pela linha de comando.
//!
//! Serve para testar o núcleo sem interface e para demonstrações. A entrega do Marco 1 é o
//! `lamina build` (importar → montagem automática → projeto → exportar).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use lamina_core::Edges;

mod argumentos;
mod auto;
mod build;
mod formato;
mod importar;
mod layouts;
mod mcp;
mod plano;
mod reabrir;
mod relatorio;
mod spec;

#[derive(Debug, Parser)]
#[command(
    name = "lamina",
    version,
    about = "Diagramação de álbuns pela linha de comando"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Mostra as medidas de impressão de uma especificação de álbum.
    Spec(spec::SpecArgs),
    /// Importa as fotos de uma pasta: data, câmera, orientação e miniaturas, com catálogo.
    Importar {
        /// Pasta com as fotos (as subpastas também são lidas).
        pasta: PathBuf,
        /// Pasta do catálogo e das miniaturas (padrão: cache do sistema).
        #[arg(long)]
        cache: Option<PathBuf>,
    },
    /// Gera os arranjos possíveis para as fotos de uma lâmina e desenha num SVG.
    Layouts {
        /// Proporções das fotos, em ordem: H (3:2), V (2:3), Q (1:1) ou L:A (ex.: 16:9).
        #[arg(long)]
        fotos: String,
        /// Largura da página, em centímetros.
        #[arg(long, default_value_t = 30.0)]
        largura_cm: f64,
        /// Altura da página, em centímetros.
        #[arg(long, default_value_t = 30.0)]
        altura_cm: f64,
        /// Zona segura, em milímetros.
        #[arg(long, default_value_t = 10.0)]
        zona_segura_mm: f64,
        /// Modo página: nada atravessa a lombada.
        #[arg(long)]
        modo_pagina: bool,
        /// Lâmina de ponta: só a página direita recebe fotos.
        #[arg(long)]
        pagina_simples: bool,
        /// Arquivo SVG de saída.
        #[arg(long, default_value = "layouts.svg")]
        svg: PathBuf,
        /// Quantas opções desenhar de cada lista (na ordem e reordenadas).
        #[arg(long, default_value_t = 24)]
        max: usize,
    },
    /// Faz o álbum inteiro: importa, monta, grava o projeto e exporta (spec comando-build).
    Build(build::BuildArgs),
    /// Reabre um projeto `.lamina` salvo e exporta os JPEGs de novo.
    Exportar(reabrir::ReopenArgs),
    /// Conferência: monta o álbum e desenha as lâminas num SVG (specs auto-build, renderizacao,
    /// exportacao).
    Auto(auto::AutoArgs),
    /// Lista os presets: os do Lamina e os salvos na janela (spec presets).
    Presets,
    /// Controle por IA: servidor MCP pela entrada e saída padrão, para o Claude e outros clientes
    /// (spec controle-por-ia).
    Mcp(mcp::McpArgs),
    /// Mostra o plano de lâminas: quais fotos vão em cada lâmina (spec plano-de-laminas).
    Plano {
        /// Pasta com as fotos.
        pasta: PathBuf,
        /// Número de lâminas do álbum.
        #[arg(long)]
        laminas: usize,
        /// O miolo começa com uma página simples (só a direita).
        #[arg(long)]
        inicia_com_pagina: bool,
        /// O miolo termina com uma página simples (só a esquerda).
        #[arg(long)]
        termina_com_pagina: bool,
        /// Pasta do catálogo e das miniaturas (padrão: cache do sistema).
        #[arg(long)]
        cache: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return argumentos::report(&err),
    };
    match run(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Erro: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::Spec(args) => spec::print_spec(&args.album.to_spec()?, args.laminas),
        Command::Presets => spec::print_presets(),
        Command::Mcp(args) => mcp::run(args)?,
        Command::Build(args) => build::run(&args)?,
        Command::Exportar(args) => reabrir::run(&args)?,
        Command::Auto(args) => auto::run(&args)?,
        Command::Importar { pasta, cache } => importar::run(&pasta, cache)?,
        Command::Layouts {
            fotos,
            largura_cm,
            altura_cm,
            zona_segura_mm,
            modo_pagina,
            pagina_simples,
            svg,
            max,
        } => {
            let aspects = layouts::parse_aspects(&fotos)?;
            layouts::run(&layouts::Args {
                aspects: &aspects,
                width_cm: largura_cm,
                height_cm: altura_cm,
                safe_mm: zona_segura_mm,
                page_mode: modo_pagina,
                single_page: pagina_simples,
                svg: &svg,
                max,
            })?;
        }
        Command::Plano {
            pasta,
            laminas,
            inicia_com_pagina,
            termina_com_pagina,
            cache,
        } => {
            let edges = Edges::new(
                spec::edge(inicia_com_pagina),
                spec::edge(termina_com_pagina),
            );
            plano::run(&pasta, laminas, edges, cache)?;
        }
    }
    Ok(())
}
