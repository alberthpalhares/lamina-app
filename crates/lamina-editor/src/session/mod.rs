//! A sessão de edição (spec JP, seção 4): o projeto aberto, o arquivo dele, a lâmina atual, as
//! alterações não salvas, as fotos ausentes e os avisos de pré-impressão.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use lamina_core::{ExportPrefs, History, PhotoId, Project};
use lamina_export::{Warning, preflight, read_project, save_project};

use crate::EditorError;
use crate::photos::used_photos;

mod edit;
mod export;
mod history;
mod relink;

pub use history::{HistoryList, HistoryStep};

/// Quantos nomes de fotos ausentes aparecem no aviso antes de "e mais N".
const MISSING_NAMES: usize = 5;

#[derive(Debug)]
pub struct Session {
    history: History<Project>,
    file: Option<PathBuf>,
    /// O projeto como está no disco; `None` = nunca salvo. Compartilhado com o histórico quando é
    /// o mesmo passo (spec LV, regra 1).
    saved: Option<Arc<Project>>,
    current: usize,
    /// A projeção da capa, enquanto a capa está aberta (spec CA, regra 5). `project` e `current`
    /// passam a falar dela, e as dezenas de lugares que leem lâmina e índice valem sem mudar.
    canvas: Option<Arc<Project>>,
    missing: Vec<PhotoId>,
    /// A janela "Algumas fotos mudaram de lugar" já abriu nesta abertura (spec RF, regra 2).
    relink_asked: bool,
    warnings: Vec<Warning>,
    /// Aberto de um arquivo (o histórico começa em "Álbum aberto"; spec HI).
    opened: bool,
    /// Última pasta de exportação usada nesta sessão (spec EI, seção 3).
    export_folder: Option<PathBuf>,
    /// Preferências de exportação do projeto: mudam sem entrar no histórico e vão para o arquivo
    /// ao salvar (spec PR, regra 5).
    export: ExportPrefs,
    /// Chave da última edição que aceita emenda (mesmo campo do mesmo quadro; spec TX, regra 10).
    merge: Option<String>,
    /// Começo do rótulo das edições: "IA: " enquanto uma ferramenta da IA roda (spec IA, regra 4).
    label_prefix: &'static str,
}

impl Session {
    /// Álbum novo, ainda sem arquivo: nasce com alterações não salvas (regra 8).
    pub fn new_unsaved(project: Project) -> Self {
        Session::with(project, None, false)
    }

    /// Abre o arquivo, mesmo com fotos ausentes (regra 11).
    pub fn open(file: &Path) -> Result<Self, EditorError> {
        let project = read_project(file)?;
        let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
        Ok(Session::with(project, Some(file), true))
    }

    fn with(project: Project, file: Option<PathBuf>, saved: bool) -> Self {
        let missing = missing_of(&project);
        let export = project.export;
        let warnings = warnings_of(&project);
        let history = History::new(project);
        Session {
            saved: saved.then(|| history.present_shared()),
            history,
            file,
            current: 0,
            canvas: None,
            missing,
            relink_asked: false,
            warnings,
            opened: saved,
            export_folder: None,
            export,
            merge: None,
            label_prefix: "",
        }
    }

    /// O que está sendo editado: o álbum, ou a projeção da capa quando a capa está aberta.
    pub fn project(&self) -> &Project {
        self.canvas
            .as_deref()
            .unwrap_or_else(|| self.history.present())
    }

    /// O projeto compartilhado, para guardar sem copiar o álbum (spec LV, regra 1).
    pub fn project_shared(&self) -> Arc<Project> {
        self.canvas
            .clone()
            .unwrap_or_else(|| self.history.present_shared())
    }

    /// O álbum inteiro, com a capa aberta ou não: contagens, fotos usadas, gravar e exportar.
    pub fn album(&self) -> &Project {
        self.history.present()
    }

    /// O álbum inteiro, compartilhado.
    pub fn album_shared(&self) -> Arc<Project> {
        self.history.present_shared()
    }

    /// A capa está aberta? (spec CA, regra 5)
    pub fn on_cover(&self) -> bool {
        self.canvas.is_some()
    }

    /// Abre a capa; `false` se o álbum não tem capa ou as medidas dela não fecham.
    pub fn go_to_cover(&mut self) -> bool {
        self.merge = None;
        self.canvas = self
            .history
            .present()
            .cover_canvas()
            .ok()
            .flatten()
            .map(Arc::new);
        self.canvas.is_some()
    }

    /// Sai da capa, de volta à última lâmina aberta.
    pub fn leave_cover(&mut self) {
        self.canvas = None;
    }

    /// Refaz a projeção depois de o álbum mudar.
    pub(crate) fn rebuild_canvas(&mut self) {
        if self.canvas.is_some() {
            self.go_to_cover();
        }
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        // Mesmo passo do histórico: basta olhar as preferências. Passo diferente: compara sem copiar,
        // porque editar e voltar à mão também volta a "sem alteração".
        let Some(saved) = &self.saved else {
            return true;
        };
        saved.export != self.export
            || !(Arc::ptr_eq(saved, &self.history.present_shared())
                || saved.same_except_export(self.album()))
    }

    /// O álbum recuperado de uma cópia de segurança (spec CS, regra 5): o projeto da cópia, o
    /// arquivo do álbum (Ctrl+S grava nele) e o projeto que está nesse arquivo, para as
    /// alterações contarem como não salvas; na lâmina em que estava.
    pub fn recovered(
        project: Project,
        file: Option<PathBuf>,
        on_disk: Option<Project>,
        spread: usize,
    ) -> Self {
        let mut session = Session::with(project, file, false);
        session.saved = on_disk.map(Arc::new);
        session.opened = true;
        session.go_to(spread);
        session
    }

    /// O projeto como vai para o arquivo: o atual com as preferências de exportação.
    pub(crate) fn for_disk(&self) -> Project {
        let mut project = self.album().clone();
        project.export = self.export;
        project
    }

    /// Formato, cor e páginas soltas do projeto (a janela Exportar abre com elas).
    pub fn export_prefs(&self) -> ExportPrefs {
        self.export
    }

    pub fn set_export_prefs(&mut self, prefs: ExportPrefs) {
        self.export = prefs;
    }

    /// "O álbum tem 18 páginas; o mínimo é 20." fora da faixa do laboratório (spec PR, regra 4).
    pub fn page_warning(&self) -> Option<String> {
        let project = self.album();
        project.page_limits.warning(project.page_count())
    }

    /// O medidor de páginas da barra de cima (spec DJ, regra 13); sem limites, nenhum.
    pub fn page_meter(&self) -> Option<crate::page_meter::PageMeter> {
        crate::page_meter::page_meter(self.project().page_limits)
    }

    /// Nome do projeto, para o título da janela.
    pub fn title(&self) -> &str {
        &self.project().name
    }

    /// Lâmina atual, contada a partir de 0.
    pub fn current(&self) -> usize {
        if self.on_cover() { 0 } else { self.current }
    }

    /// A última lâmina aberta, com a capa aberta ou não: reabrir o álbum volta para ela.
    pub fn last_spread(&self) -> usize {
        self.current
    }

    pub fn spread_count(&self) -> usize {
        self.project().spreads.len()
    }

    /// Vai para a lâmina `index`; `false` (e nada muda) se ela não existe.
    pub fn go_to(&mut self, index: usize) -> bool {
        if index >= self.album().spreads.len() {
            return false;
        }
        self.canvas = None;
        self.current = index;
        true
    }

    /// Volta para a posição 0, que vale também num álbum sem lâminas (spec LL).
    pub(crate) fn rewind(&mut self) {
        self.current = 0;
    }

    /// "3 lâminas (6 páginas) · 12 de 12 fotos usadas".
    pub fn counter(&self) -> String {
        let project = self.album();
        let spreads = project.spreads.len();
        let pages = project.page_count();
        let (s, p) = (plural(spreads, "lâmina"), plural(pages, "página"));
        let used = used_photos(project);
        let total = project.photos.len();
        format!("{spreads} {s} ({pages} {p}) · {used} de {total} fotos usadas")
    }

    pub fn missing(&self) -> &[PhotoId] {
        &self.missing
    }

    /// "1 foto não foi encontrada: IMG_0003.jpg." (até 5 nomes, depois "e mais N").
    pub fn missing_message(&self) -> Option<String> {
        let names: Vec<String> = self
            .missing
            .iter()
            .filter_map(|id| self.project().photo(*id).ok())
            .map(|p| file_name(&p.path))
            .collect();
        let n = names.len();
        let list = match n {
            0 => return None,
            1 => names[0].clone(),
            _ if n > MISSING_NAMES => format!(
                "{} e mais {}",
                names[..MISSING_NAMES].join(", "),
                n - MISSING_NAMES
            ),
            _ => format!("{} e {}", names[..n - 1].join(", "), names[n - 1]),
        };
        Some(if n == 1 {
            format!("1 foto não foi encontrada: {list}.")
        } else {
            format!("{n} fotos não foram encontradas: {list}.")
        })
    }

    /// Avisos de pré-impressão da lâmina `index` (contada a partir de 0).
    pub fn warnings_for(&self, index: usize) -> Vec<&Warning> {
        // Na capa, os avisos dela (lâmina 0, fora da numeração).
        let wanted = if self.on_cover() { 0 } else { index + 1 };
        self.warnings
            .iter()
            .filter(|w| w.spread == wanted)
            .collect()
    }

    /// Avisos da lâmina `index` do álbum, com a capa aberta ou não (a faixa de lâminas).
    pub fn spread_warnings(&self, index: usize) -> Vec<&Warning> {
        self.warnings
            .iter()
            .filter(|w| w.spread == index + 1)
            .collect()
    }

    /// Avisos da capa, com a capa aberta ou não (lâmina 0, fora da numeração).
    pub fn cover_warnings(&self) -> Vec<&Warning> {
        self.warnings.iter().filter(|w| w.spread == 0).collect()
    }

    /// A capa tem avisos? (o selo da capa na faixa)
    pub fn cover_warned(&self) -> bool {
        self.warnings.iter().any(|w| w.spread == 0)
    }

    /// Salva no arquivo da sessão.
    pub fn save(&mut self) -> Result<PathBuf, EditorError> {
        let file = self.file.clone().ok_or(EditorError::NoFile)?;
        self.save_as(&file)
    }

    /// Salva em `file` e passa a usar esse arquivo (regra 9).
    pub fn save_as(&mut self, file: &Path) -> Result<PathBuf, EditorError> {
        let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
        let folder = file.parent().unwrap_or(Path::new("."));
        let name = file_name(&file);
        let disk = self.for_disk();
        let saved = save_project(&disk, folder, &name)?;
        self.saved = Some(if disk.export == self.album().export {
            self.history.present_shared()
        } else {
            Arc::new(disk)
        });
        self.file = Some(saved.clone());
        Ok(saved)
    }
}

/// As fotos cujo arquivo não existe no caminho do projeto.
/// As fotos cujo arquivo não existe agora (spec RF, regra 6: só olha se existe, sem ler o
/// conteúdo). Pode levar um tempo num disco de rede: a janela chama fora da linha dela.
pub fn still_missing(project: &Project) -> Vec<PhotoId> {
    missing_of(project)
}

/// Os caminhos das fotos do álbum, para a conferência de 5 em 5 segundos levar só eles para fora
/// da linha da janela, sem copiar o álbum (spec RF, regra 10).
pub fn photo_paths(project: &Project) -> Vec<(PhotoId, PathBuf)> {
    project
        .photos
        .iter()
        .map(|p| (p.id, p.path.clone()))
        .collect()
}

/// As fotos de `paths` cujo arquivo não existe agora (só olha se existe; regra 10).
pub fn missing_among(paths: &[(PhotoId, PathBuf)]) -> Vec<PhotoId> {
    paths
        .iter()
        .filter(|(_, path)| !path.exists())
        .map(|(id, _)| *id)
        .collect()
}

fn missing_of(project: &Project) -> Vec<PhotoId> {
    project
        .photos
        .iter()
        .filter(|p| !p.path.exists())
        .map(|p| p.id)
        .collect()
}

/// Avisos de pré-impressão de todas as lâminas.
fn warnings_of(project: &Project) -> Vec<Warning> {
    let all: Vec<usize> = (0..project.spreads.len()).collect();
    let mut warnings = lamina_export::cover_preflight(project);
    warnings.extend(preflight(project, &all));
    warnings
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}
