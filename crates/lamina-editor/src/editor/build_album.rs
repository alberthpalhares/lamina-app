//! "Montar o álbum…" pelo editor (spec BR): quando pode, o que vai acontecer e a edição única no
//! fim. A conta mora em [`crate::building`]; a janela roda num trabalhador e chama
//! [`Editor::finish_build`], a IA e os testes usam [`Editor::build_album`].

use lamina_core::Project;
use lamina_layout::{BuildError, BuildOptions, PlanError, Reuse, SpreadReport};

use super::Editor;
use crate::EditorError;
use crate::building::{AlbumBuild, album_build, build_label, build_unused};

impl Editor {
    /// O que montar no álbum aberto, ou a mensagem de por que não dá (seção 6).
    pub fn album_build(&self) -> Result<AlbumBuild, &'static str> {
        let session = self.session.as_ref().ok_or(crate::building::NO_PHOTOS)?;
        album_build(session.album())
    }

    /// "Montar o álbum…" pode rodar agora? Álbum aberto, fora do modo organizar, editor livre e
    /// alguma foto não usada (regra 12; o editor ocupado o `available` já confere).
    pub(super) fn can_build_album(&self) -> bool {
        self.mode() == crate::commands::Mode::Normal && self.album_build().is_ok()
    }

    /// As opções do Auto Build da janela: a variedade escolhida, os favoritos, os rejeitados e
    /// os modelos do fotógrafo (regra 6).
    pub fn build_options(&self, reuse: Reuse) -> BuildOptions {
        BuildOptions {
            reuse,
            preferences: self.preferences().clone(),
            models: self.models().to_vec(),
            ..BuildOptions::default()
        }
    }

    /// A conta terminou: o álbum `built` (de `spreads` lâminas novas com `photos` fotos) entra no
    /// histórico numa edição só; a lâmina atual passa a ser a primeira nova e a seleção do painel
    /// é limpa (regra 10).
    pub fn finish_build(&mut self, built: Project, spreads: usize, photos: usize) -> bool {
        self.leave_cover();
        let first = built.spreads.len().saturating_sub(spreads);
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let label = build_label(spreads, photos);
        if session
            .edit(&label, |p| -> Result<(), EditorError> {
                *p = built;
                Ok(())
            })
            .is_err()
        {
            return false;
        }
        self.selection.clear();
        self.go(first);
        true
    }

    /// Monta já, na mesma linha (a IA e os testes): `spreads` lâminas novas, ou a sugestão.
    /// Devolve o relatório do Auto Build, uma linha por lâmina nova. Quem chama confere antes
    /// [`Editor::album_build`], que tem as mensagens da seção 6.
    pub fn build_album(
        &mut self,
        spreads: Option<usize>,
        reuse: Reuse,
    ) -> Result<Vec<SpreadReport>, EditorError> {
        let nothing = || EditorError::Build(BuildError::Plan(PlanError::NoPhotos));
        let plan = self.album_build().map_err(|_| nothing())?;
        let spreads = spreads.unwrap_or(plan.suggestion);
        let session = self.session.as_ref().ok_or(EditorError::NoProject)?;
        let options = self.build_options(reuse);
        let (built, report) = build_unused(session.album(), spreads, &options)?;
        self.finish_build(built, spreads, plan.unused.len());
        Ok(report)
    }
}
