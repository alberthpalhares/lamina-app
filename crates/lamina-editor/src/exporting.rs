//! Exportar pela janela (spec EI, regras 1 a 3): a conferência antes, a exportação fora da linha
//! da janela, "Ir para a lâmina" e os textos de andamento e resumo.

use std::path::Path;

use lamina_core::Project;
pub use lamina_export::{CoverExport, DEFAULT_QUALITY, ExportFormat, ExportOptions, Warning};
use lamina_export::{ExportReport, export};
use lamina_media::original_source_in;

use crate::{Editor, EditorError};

/// O que a janela mostra antes de exportar: os avisos da pré-impressão das lâminas escolhidas e
/// os arquivos que já existem na pasta (para perguntar "Substituir os arquivos?").
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportCheck {
    pub warnings: Vec<Warning>,
    pub existing: Vec<String>,
    /// Aviso do álbum inteiro: fora da faixa de páginas do laboratório (spec PR, regra 4).
    pub album: Option<String>,
}

impl ExportCheck {
    /// "Ignorar": o aviso some da lista desta exportação. `false` se ele não existe.
    pub fn ignore(&mut self, index: usize) -> bool {
        if index >= self.warnings.len() {
            return false;
        }
        self.warnings.remove(index);
        true
    }
}

/// Exporta o projeto como está na tela (não precisa salvar antes), lendo os originais no espaço
/// de cor da saída (spec CT): os mesmos
/// arquivos do `lamina exportar` com as mesmas opções (regra 1). Roda em qualquer linha;
/// `progress` recebe (feitos, total) a cada arquivo.
pub fn export_project(
    project: &Project,
    folder: &Path,
    options: &ExportOptions,
    progress: &dyn Fn(usize, usize),
) -> Result<ExportReport, EditorError> {
    Ok(export(
        project,
        folder,
        &original_source_in(options.space),
        options,
        progress,
    )?)
}

/// "Exportando 3 de 10…".
pub fn export_progress(done: usize, total: usize) -> String {
    format!("Exportando {done} de {total}…")
}

/// "10 arquivos em <pasta> (12 s)", e quantos arquivos antigos saíram ao substituir.
pub fn export_summary(report: &ExportReport, folder: &Path) -> String {
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let seconds = report.elapsed.as_secs_f64().round().max(1.0);
    let mut text = format!(
        "{} em {} ({seconds:.0} s)",
        count(report.files.len(), "arquivo", "arquivos"),
        folder.display()
    );
    if !report.removed.is_empty() {
        let removed = count(
            report.removed.len(),
            "arquivo antigo apagado",
            "arquivos antigos apagados",
        );
        text.push_str("; ");
        text.push_str(&removed);
    }
    text
}

impl Editor {
    /// "Ir para a lâmina": mostra a lâmina do aviso com o quadro dele selecionado (regra 2).
    pub fn go_to_warning(&mut self, warning: &Warning) -> bool {
        let Some(index) = warning.spread.checked_sub(1) else {
            return false;
        };
        let Some(session) = self.session_mut() else {
            return false;
        };
        if !session.go_to(index) {
            return false;
        }
        self.settle_visit();
        let Some(session) = self.session_mut() else {
            return false;
        };
        let frames = session
            .album()
            .spreads
            .get(index)
            .map_or(0, |s| s.frames.len());
        self.select_frame((warning.frame < frames).then_some(warning.frame));
        true
    }
}
