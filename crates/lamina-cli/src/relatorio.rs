//! O que os comandos `auto`, `build` e `exportar` mostram na tela: relatório da montagem automática,
//! pré-impressão e andamento da exportação.

use std::io::Write as _;
use std::path::Path;

use anyhow::Result;
use lamina_core::Project;
use lamina_export::{ExportOptions, ExportReport, Warning, export};
use lamina_layout::AutoBuild;
use lamina_render::PhotoSource;

use crate::layouts::family_name;
use crate::plano::role_name;

/// Só o nome do arquivo, para as mensagens.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Uma linha por lâmina: momento, papel, fotos, nota, motivo e família.
pub fn print_build_report(built: &AutoBuild) {
    println!(
        "{:<7} {:<8} {:<9} {:<6} {:<5} motivo",
        "lâmina", "momento", "papel", "fotos", "nota"
    );
    for (i, r) in built.report.iter().enumerate() {
        println!(
            "{:<7} {:<8} {:<9} {:<6} {:<5.0} {} — {} (opção {} da lista)",
            i + 1,
            r.moment + 1,
            role_name(r.role),
            r.photos,
            r.score,
            r.reason,
            family_name(&r.family),
            r.position
        );
    }
}

/// Os avisos da pré-impressão (ou "nenhum aviso").
pub fn print_warnings(warnings: &[Warning]) {
    if warnings.is_empty() {
        println!("pré-impressão: nenhum aviso");
        return;
    }
    println!("pré-impressão: {} aviso(s)", warnings.len());
    for warning in warnings {
        println!("  {warning}");
    }
}

/// Exporta mostrando "exportando: N de M" e, no fim, quantos arquivos e quanto tempo.
pub fn export_with_progress(
    project: &Project,
    folder: &Path,
    photos: &dyn PhotoSource,
    options: &ExportOptions,
) -> Result<ExportReport> {
    let progress = |done: usize, total: usize| {
        print!("\rexportando: {done} de {total}");
        let _ = std::io::stdout().flush();
    };
    let report = export(project, folder, photos, options, &progress)?;
    println!(
        "\n{} arquivo(s) em {} ({:.1} s)",
        report.files.len(),
        folder.display(),
        report.elapsed.as_secs_f64()
    );
    match report.removed.len() {
        0 => {}
        1 => println!("1 arquivo antigo apagado"),
        n => println!("{n} arquivos antigos apagados"),
    }
    Ok(report)
}
