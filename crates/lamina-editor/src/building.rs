//! "Montar o álbum…" (spec BR): as fotos ainda não usadas viram lâminas novas depois da última,
//! pelo Auto Build, numa edição só. A conta ([`build_unused`]) roda fora da linha da janela; a
//! edição entra no histórico pelo editor (`Editor::finish_build`).

use std::collections::HashSet;

use lamina_core::{AlbumEdge, FrameContent, PhotoId, Project, Spread};
use lamina_layout::{
    BuildError, BuildOptions, PlanError, PlanOptions, SpreadReport, auto_build, suggest_spreads,
    validate_plan,
};

use crate::EditorError;
use crate::album::suggestion;

/// Todas as fotos já estão em alguma lâmina (seção 6).
pub const ALL_USED: &str = "Todas as fotos já estão em lâminas.";
/// O álbum não tem fotos (seção 6).
pub const NO_PHOTOS: &str = "Este álbum ainda não tem fotos. Importe fotos antes de montar.";
/// Mais fotos do que uma lâmina leva, no álbum em branco (regra 4).
pub const ONE_SPREAD_LIMIT: &str =
    "Uma lâmina leva até 24 fotos. Para montar várias lâminas de uma vez, use Montar o álbum…";

/// O que "Montar o álbum…" faria no álbum de agora.
#[derive(Debug, Clone, PartialEq)]
pub struct AlbumBuild {
    /// As fotos que ainda não estão em nenhuma lâmina, na ordem do álbum.
    pub unused: Vec<PhotoId>,
    /// Quantas lâminas ficam antes das novas (as vazias saem; regra 6).
    pub kept: usize,
    /// O número de lâminas sugerido (cerca de 4 fotos por lâmina; regra 8).
    pub suggestion: usize,
}

/// Uma lâmina sem foto, sem texto e sem página em branco de propósito (os quadros vazios não
/// contam; spec PB, regra 8).
fn is_empty(spread: &Spread) -> bool {
    spread.blank.is_none()
        && spread
            .frames
            .iter()
            .all(|f| matches!(f.content, FrameContent::Empty))
}

/// As fotos usadas em alguma lâmina.
fn used(project: &Project) -> HashSet<PhotoId> {
    project
        .spreads
        .iter()
        .flat_map(|s| &s.frames)
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, .. } => Some(*photo),
            _ => None,
        })
        .collect()
}

/// O projeto que o Auto Build monta: só as fotos não usadas, sem lâminas. Com lâminas que ficam,
/// as novas são miolo: a primeira nunca começa com página simples (regra 7).
fn unused_project(project: &Project) -> Project {
    let used = used(project);
    let mut sub = project.clone();
    sub.photos.retain(|p| !used.contains(&p.id));
    sub.spreads.clear();
    if project.spreads.iter().any(|s| !is_empty(s)) {
        let edges = project.spec.edges();
        sub.spec = sub.spec.with_edges(AlbumEdge::FullSpread, edges.end);
    }
    sub
}

/// O que montar no `project`, ou a mensagem de por que não dá (seção 6).
pub fn album_build(project: &Project) -> Result<AlbumBuild, &'static str> {
    if project.photos.is_empty() {
        return Err(NO_PHOTOS);
    }
    let sub = unused_project(project);
    if sub.photos.is_empty() {
        return Err(ALL_USED);
    }
    let kept = project.spreads.iter().filter(|s| !is_empty(s)).count();
    let suggestion = if kept == 0 {
        suggestion(&sub)
    } else {
        suggest_spreads(sub.photos.len(), sub.spec.edges(), &PlanOptions::default())
    };
    Ok(AlbumBuild {
        unused: sub.photos.iter().map(|p| p.id).collect(),
        kept,
        suggestion,
    })
}

impl AlbumBuild {
    /// "As 48 fotos do álbum viram lâminas." ou "48 fotos ainda não usadas viram lâminas novas,
    /// depois da lâmina 4."
    pub fn what(&self) -> String {
        let n = self.unused.len();
        match (self.kept, n) {
            (0, 1) => "A foto do álbum vira uma lâmina.".to_owned(),
            (0, _) => format!("As {n} fotos do álbum viram lâminas."),
            (k, 1) => format!("1 foto ainda não usada vira lâmina nova, depois da lâmina {k}."),
            (k, _) => {
                format!("{n} fotos ainda não usadas viram lâminas novas, depois da lâmina {k}.")
            }
        }
    }

    /// "Sugestão: 12 lâminas (cerca de 4 fotos por lâmina)."
    pub fn suggestion_text(&self) -> String {
        crate::album::suggestion_text(self.suggestion)
    }

    /// `spreads` lâminas novas cabem as fotos? Com a mensagem do plano, se não (regra 8).
    pub fn check(&self, project: &Project, spreads: usize) -> Result<(), PlanError> {
        let edges = unused_project(project).spec.edges();
        validate_plan(self.unused.len(), spreads, edges, &PlanOptions::default())
    }

    /// "O álbum fica com 16 lâminas (32 páginas)." e, fora da faixa de páginas do laboratório, o
    /// aviso (que é só aviso; spec PR).
    pub fn result(&self, project: &Project, spreads: usize) -> (String, Option<String>) {
        let total = self.kept + spreads;
        let pages = project.spec.edges().page_count(total);
        let word = if total == 1 { "lâmina" } else { "lâminas" };
        let line = format!("O álbum fica com {total} {word} ({pages} páginas).");
        (line, project.page_limits.warning(pages))
    }
}

/// O rótulo da edição: "Montar 12 lâminas com 48 fotos · álbum".
pub fn build_label(spreads: usize, photos: usize) -> String {
    let s = if spreads == 1 { "lâmina" } else { "lâminas" };
    let p = if photos == 1 { "foto" } else { "fotos" };
    format!("Montar {spreads} {s} com {photos} {p} · álbum")
}

/// O álbum depois de montar as fotos não usadas em `spreads` lâminas novas depois da última
/// (regras 5 a 7 e 10): as lâminas vazias saem, as outras ficam (menos a antiga última de um
/// álbum que termina com página simples, rearranjada), as novas ganham a aparência do álbum e
/// ids novos. Devolve também o relatório do Auto Build, uma linha por lâmina nova.
pub fn build_unused(
    project: &Project,
    spreads: usize,
    options: &BuildOptions,
) -> Result<(Project, Vec<SpreadReport>), EditorError> {
    let sub = unused_project(project);
    if sub.photos.is_empty() {
        return Err(EditorError::Build(BuildError::Plan(PlanError::NoPhotos)));
    }
    let built = auto_build(&sub, spreads, options)?;
    let mut out = built.project;
    crate::new_project::apply_look(&mut out);
    let kept: Vec<Spread> = project
        .spreads
        .iter()
        .filter(|s| !is_empty(s))
        .cloned()
        .collect();
    let new = std::mem::take(&mut out.spreads);
    // O `out` vem do projeto inteiro: os ids novos continuam a contagem dele.
    out.photos.clone_from(&project.photos);
    out.spec = project.spec.clone();
    out.spreads = kept;
    crate::spreads::append_spreads(&mut out, new)?;
    Ok((out, built.report))
}

/// Quantas fotos uma lâmina nova leva no álbum em branco (o limite da regra 4).
pub fn blank_capacity() -> usize {
    2 * lamina_layout::MAX_PER_PAGE
}
