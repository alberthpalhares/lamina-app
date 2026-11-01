//! Exportar pela IA (spec IA, regras 6 e 10; spec EI): confere antes, não substitui sem
//! `substituir: true` e nunca apaga arquivo. Os padrões são os da janela e as preferências do
//! álbum (spec PR).

use std::path::Path;

use lamina_core::{ColorSpace, OutputFormat};
use lamina_export::{CoverExport, ExportFormat, ExportOptions, leftovers, output_names};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Agent, Progress, ToolError, ToolReply, session};
use crate::{DEFAULT_QUALITY, Editor, export_project, export_summary};

/// Formato da exportação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FormatArg {
    Jpeg,
    Pdf,
    Tiff,
}

/// Espaço de cor da saída.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColorArg {
    Srgb,
    AdobeRgb,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportCall {
    /// Pasta de saída (criada se não existir).
    pub pasta: String,
    /// Padrão: o formato das preferências do álbum.
    pub formato: Option<FormatArg>,
    /// Padrão: a cor das preferências do álbum.
    pub cor: Option<ColorArg>,
    /// Lâminas a exportar, como "1, 3-5"; padrão: todas.
    pub intervalo: Option<String>,
    /// Marcas de corte (só no PDF).
    #[serde(default)]
    pub marcas_de_corte: bool,
    /// Gravar por cima de arquivos com os mesmos nomes.
    #[serde(default)]
    pub substituir: bool,
    /// A capa, num álbum que tem capa: junto com as lâminas (padrão), só ela ou sem ela.
    pub capa: Option<CoverArg>,
}

/// O que fazer com a capa ao exportar (spec CA, regra 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverArg {
    Junto,
    SoACapa,
    SemACapa,
}

impl Agent {
    pub(super) fn export(
        &mut self,
        editor: &mut Editor,
        a: &ExportCall,
        progress: Progress<'_>,
    ) -> Result<ToolReply, ToolError> {
        let folder = self.roots.check(Path::new(&a.pasta))?;
        let s = session(editor)?;
        let options = options_for(a, s.export_prefs());
        let check = s.check_export(&folder, &options)?;
        if let (false, Some(first)) = (a.substituir, check.existing.first()) {
            return Err(ToolError::ExportExists {
                count: check.existing.len(),
                folder: folder.display().to_string(),
                example: first.clone(),
            });
        }
        refuse_leftovers(s.project(), &folder, &options)?;
        let report = export_project(s.project(), &folder, &options, &|done, total| {
            progress(done, total);
        })?;
        let warnings: Vec<String> = check.warnings.iter().map(ToString::to_string).collect();
        let files: Vec<String> = report
            .files
            .iter()
            .map(|f| f.display().to_string())
            .collect();
        let data =
            json!({ "arquivos": files, "avisos": warnings, "aviso_de_paginas": check.album });
        Ok(ToolReply::new(export_summary(&report, &folder), data))
    }
}

/// As opções da janela Exportar: o que a IA pediu, ou as preferências do álbum.
fn options_for(a: &ExportCall, prefs: lamina_core::ExportPrefs) -> ExportOptions {
    let format = match a.formato {
        Some(FormatArg::Jpeg) => ExportFormat::Jpeg,
        Some(FormatArg::Pdf) => ExportFormat::Pdf,
        Some(FormatArg::Tiff) => ExportFormat::Tiff,
        None => match prefs.format {
            OutputFormat::Jpeg => ExportFormat::Jpeg,
            OutputFormat::Pdf => ExportFormat::Pdf,
            OutputFormat::Tiff => ExportFormat::Tiff,
        },
    };
    let space = match a.cor {
        Some(ColorArg::Srgb) => ColorSpace::Srgb,
        Some(ColorArg::AdobeRgb) => ColorSpace::AdobeRgb,
        None => prefs.space,
    };
    ExportOptions {
        quality: DEFAULT_QUALITY,
        range: a.intervalo.clone(),
        pages: prefs.pages,
        overwrite: a.substituir,
        format,
        crop_marks: format == ExportFormat::Pdf && a.marcas_de_corte,
        space,
        cover: match a.capa {
            Some(CoverArg::SoACapa) => CoverExport::Only,
            Some(CoverArg::SemACapa) => CoverExport::Without,
            _ => CoverExport::WithSpreads,
        },
    }
}

/// Substituir apagaria as sobras de uma exportação anterior (spec EI, regra 4): a IA recusa e
/// diz quais são, em vez de apagar (spec IA, regra 6).
fn refuse_leftovers(
    project: &lamina_core::Project,
    folder: &Path,
    options: &ExportOptions,
) -> Result<(), ToolError> {
    if !options.overwrite || options.format == ExportFormat::Pdf {
        return Ok(());
    }
    let ext = if options.format == ExportFormat::Tiff {
        ".tif"
    } else {
        ".jpg"
    };
    // "Só a capa" não apaga nada: as lâminas da exportação anterior continuam valendo.
    if options.cover == CoverExport::Only && project.cover.is_some() {
        return Ok(());
    }
    let names = output_names(project, options)?;
    let found = leftovers(folder, &names, ext)?;
    match found.first() {
        None => Ok(()),
        Some(first) => Err(ToolError::Leftovers {
            count: found.len(),
            folder: folder.display().to_string(),
            example: super::paths::file_name(first),
        }),
    }
}
