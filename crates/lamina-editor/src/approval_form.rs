//! A caixa "Enviar para aprovação do cliente" sem a janela (specs AC e AH, seção 3): os campos, o que
//! cada um reclama, as opções que eles dão e a geração, fora da linha da janela.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lamina_core::{Photo, PhotoId, Project};
use lamina_export::{
    ApprovalHtml, ApprovalOptions, ApprovalReport, WHATSAPP_START, Warning, approval_photos_found,
    approval_question, approval_summary, clean_whatsapp, client_name_problem, email_problem,
    export_approval, outputs_problem, parse_range, watermark_problem, whatsapp_display,
};
use lamina_media::approval_source;

use crate::approval_store::{ApprovalStore, PhotographerData};
use crate::{Cache, EditorError, Human};

/// Os campos da caixa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalForm {
    pub jpg: bool,
    pub html: bool,
    pub all_spreads: bool,
    /// Só vale para o JPG, como o número.
    pub range: String,
    pub number: bool,
    pub watermark_on: bool,
    pub watermark_text: String,
    /// O nome que o cliente vê: começa com o nome do álbum, e só o HTML o usa.
    pub client_name: String,
    /// Como foi digitado; a caixa mostra ao lado como ficou ([`lamina_export::whatsapp_display`]).
    pub whatsapp: String,
    pub email: String,
}

/// O que marcar em cada campo. A mensagem é a da spec, sem o detalhe técnico.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApprovalProblems {
    pub outputs: Option<String>,
    pub range: Option<String>,
    pub watermark: Option<String>,
    pub name: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
}

impl ApprovalProblems {
    /// Nada a marcar: "Gerar arquivos" fica disponível.
    pub fn is_clear(&self) -> bool {
        *self == ApprovalProblems::default()
    }
}

impl ApprovalForm {
    /// A caixa ao abrir: as duas saídas ligadas, o álbum inteiro e o que o fotógrafo usou por último.
    pub fn new(data: &PhotographerData, project: &Project) -> Self {
        ApprovalForm {
            jpg: true,
            html: true,
            all_spreads: true,
            range: String::new(),
            number: true,
            watermark_on: data.watermark_on,
            watermark_text: data.watermark_text.clone(),
            client_name: project.name.clone(),
            whatsapp: if data.whatsapp.trim().is_empty() {
                WHATSAPP_START.to_owned()
            } else {
                data.whatsapp.clone()
            },
            email: data.email.clone(),
        }
    }

    /// O que cada campo reclama. Campo de uma saída desligada não reclama de nada.
    pub fn problems(&self, project: &Project) -> ApprovalProblems {
        let text = |problem: Option<&str>| problem.map(str::to_owned);
        let mut problems = ApprovalProblems {
            outputs: text(outputs_problem(project, self.jpg, self.html)),
            ..ApprovalProblems::default()
        };
        if self.jpg && !self.all_spreads {
            problems.range = parse_range(self.range.trim(), project.spreads.len())
                .err()
                .map(|e| lamina_export::ExportError::human(&e).title);
        }
        if self.watermark_on {
            problems.watermark = watermark_problem(&self.watermark_text);
        }
        if self.html {
            problems.name = text(client_name_problem(&self.client_name));
            problems.whatsapp = text(clean_whatsapp(&self.whatsapp).err());
            problems.email = text(email_problem(&self.email));
        }
        problems
    }

    /// As opções da geração, com os campos já conferidos por [`ApprovalForm::problems`].
    pub fn options(&self, overwrite: bool) -> ApprovalOptions {
        let filled = |text: &str| Some(text.trim().to_owned()).filter(|t| !t.is_empty());
        ApprovalOptions {
            range: (self.jpg && !self.all_spreads).then(|| self.range.trim().to_owned()),
            number: self.number,
            watermark: self
                .watermark_on
                .then(|| filled(&self.watermark_text))
                .flatten(),
            overwrite,
            jpg: self.jpg,
            html: self.html.then(|| ApprovalHtml {
                name: self.client_name.trim().to_owned(),
                whatsapp: clean_whatsapp(&self.whatsapp).ok().flatten(),
                email: filled(&self.email),
            }),
        }
    }

    /// Como o número ficou, para a caixa mostrar ao lado do campo; vazio sem número ou com erro.
    pub fn whatsapp_shown(&self) -> String {
        clean_whatsapp(&self.whatsapp)
            .ok()
            .flatten()
            .map_or_else(String::new, |clean| whatsapp_display(&clean))
    }

    /// O que fica guardado depois de gerar com sucesso, como foi digitado.
    pub fn data(&self) -> PhotographerData {
        PhotographerData {
            watermark_on: self.watermark_on,
            watermark_text: self.watermark_text.clone(),
            // Só o código do país, como o campo abre, não é um número.
            whatsapp: if clean_whatsapp(&self.whatsapp) == Ok(None) {
                String::new()
            } else {
                self.whatsapp.clone()
            },
            email: self.email.clone(),
        }
    }
}

/// O que a caixa mostra antes de gerar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApprovalCheck {
    /// O que muda o que o cliente vê (AC, regra 15).
    pub warnings: Vec<Warning>,
    /// O que seria substituído, para perguntar ([`lamina_export::approval_question`]).
    pub existing: Vec<String>,
    /// O HTML vai sair com a capa lisa (AH, regra 8).
    pub cover: Option<String>,
}

/// Gera os arquivos, depois de conferir as fotos: foto não encontrada para tudo, antes de gravar (AC,
/// regra 16). Roda em qualquer linha; `progress` recebe (feitos, total).
pub fn generate_approval(
    project: &Project,
    folder: &Path,
    options: &ApprovalOptions,
    cache: &Cache,
    progress: &dyn Fn(usize, usize),
) -> Result<ApprovalReport, EditorError> {
    approval_photos_found(project, options)?;
    let copies = |find: &dyn Fn(&Photo) -> Option<PathBuf>| -> HashMap<PhotoId, PathBuf> {
        project
            .photos
            .iter()
            .filter_map(|photo| Some((photo.id, find(photo)?)))
            .collect()
    };
    let source = approval_source(
        copies(&|photo| cache.thumbnail(photo)),
        copies(&|photo| cache.screen(photo)),
    );
    Ok(export_approval(
        project, folder, &source, options, progress,
    )?)
}

/// O título e o texto da pergunta de substituir (AC, regra 14).
pub fn approval_replace(existing: usize, folder: &Path, form: &ApprovalForm) -> (String, String) {
    let name = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    approval_question(existing, &name, form.jpg && !form.all_spreads)
}

/// Depois de gerar com sucesso: guarda os dados do fotógrafo (AC, regra 22) e devolve o resumo. Não
/// conseguir guardar não desfaz os arquivos: o resumo só avisa.
pub fn approval_finished(
    mut store: ApprovalStore,
    data: PhotographerData,
    report: &ApprovalReport,
) -> String {
    store.data = data;
    let mut text = approval_summary(report);
    if let Err(problem) = store.save() {
        text.push('\n');
        text.push_str(&problem);
    }
    text
}

/// "Gerando 3 de 41…".
pub fn approval_progress(done: usize, total: usize) -> String {
    format!("Gerando {done} de {total}…")
}
