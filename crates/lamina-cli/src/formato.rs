//! `--formato`, `--marcas-de-corte` e `--cor` do `lamina build` e do `lamina exportar` (specs PD
//! e CT, seção 3).

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use lamina_core::{ColorSpace, ExportPrefs, OutputFormat};
use lamina_export::ExportFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Formato {
    /// Um JPEG por lâmina (ou por página).
    #[default]
    Jpeg,
    /// Um PDF/X-4 do álbum inteiro.
    Pdf,
    /// Um TIFF sem perda por lâmina (ou por página).
    Tiff,
}

/// Espaço de cor da saída: só sRGB (pedido do dono 2026-11-09 — os laboratórios pedem sRGB e a
/// opção de Adobe RGB confundia; a leitura de fotos Adobe RGB que chegam continua).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Cor {
    #[default]
    Srgb,
}

#[derive(Debug, Args)]
pub struct SaidaArgs {
    /// Formato da exportação (padrão: o do preset, ou jpeg).
    #[arg(long, value_enum)]
    pub formato: Option<Formato>,
    /// Marcas de corte nos cantos (só no PDF).
    #[arg(long)]
    pub marcas_de_corte: bool,
    /// Espaço de cor da saída: só srgb (Adobe RGB saiu; aceito para não quebrar roteiros).
    #[arg(long, value_enum)]
    pub cor: Option<Cor>,
}

impl SaidaArgs {
    /// Marcas de corte pedem `--formato pdf` digitado (o erro sai antes de abrir qualquer coisa).
    pub fn check_marks(&self) -> Result<()> {
        if self.marcas_de_corte && self.formato != Some(Formato::Pdf) {
            bail!("marcas de corte só existem no PDF (use --formato pdf).");
        }
        Ok(())
    }

    /// Como [`Self::options`], com o formato do preset ou do projeto quando nenhum foi digitado
    /// (spec PR, regra 5).
    pub fn options_with(&self, prefs: ExportPrefs) -> Result<(ExportFormat, bool)> {
        self.check_marks()?;
        let formato = self.formato.unwrap_or(match prefs.format {
            OutputFormat::Jpeg => Formato::Jpeg,
            OutputFormat::Pdf => Formato::Pdf,
            OutputFormat::Tiff => Formato::Tiff,
        });
        match formato {
            Formato::Jpeg | Formato::Tiff if self.marcas_de_corte => {
                bail!("marcas de corte só existem no PDF (use --formato pdf).")
            }
            Formato::Jpeg => Ok((ExportFormat::Jpeg, false)),
            Formato::Tiff => Ok((ExportFormat::Tiff, false)),
            Formato::Pdf => Ok((ExportFormat::Pdf, self.marcas_de_corte)),
        }
    }

    /// O espaço digitado, ou o do preset — mas sempre sRGB na saída (Adobe RGB saiu, 2026-11-09):
    /// um preset antigo em Adobe RGB não continua exportando num espaço que a janela não oferece
    /// mais.
    pub fn space_with(&self, prefs: ExportPrefs) -> ColorSpace {
        let _ = self.cor;
        let _ = prefs.space;
        ColorSpace::Srgb
    }
}
