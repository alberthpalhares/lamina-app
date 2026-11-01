//! A lista de recentes no editor (spec TI): quem abre, salva, fecha ou sai atualiza a lista, e
//! abrir um álbum conhecido vai direto para onde o fotógrafo parou (regras 1 e 3).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use lamina_core::Project;

use super::Editor;
use crate::recents::{Recent, RecentsStore, now, shows_cover};
use crate::{EditorError, Session};

impl Editor {
    /// Passa a usar a lista de `store` (lida de `recentes.json`). O aviso de leitura, se houver,
    /// vira o próximo aviso da janela.
    pub fn set_recents(&mut self, mut store: RecentsStore) {
        if let Some(warning) = store.warning.take() {
            self.notice = Some(warning);
        }
        self.recents = store;
    }

    pub fn recents(&self) -> &RecentsStore {
        &self.recents
    }

    /// Muda a lista (fixar, tirar, procurar, limpar) e grava; se não gravar, vale nesta sessão,
    /// com aviso (seção 6).
    pub fn change_recents<R>(&mut self, change: impl FnOnce(&mut RecentsStore) -> R) -> R {
        let result = change(&mut self.recents);
        if let Err(message) = self.recents.save() {
            self.notice = Some(message);
        }
        result
    }

    /// Salva no arquivo da sessão, ou em `target` ("Salvar como"), e põe o álbum no topo da
    /// lista com a lâmina atual (regras 1 e 3).
    pub fn save_to(&mut self, target: Option<&Path>) -> Result<PathBuf, EditorError> {
        // Salvar fecha a faixa de layouts antes (spec FL, regra 14).
        self.close_band();
        let session = self.session.as_mut().ok_or(EditorError::NoProject)?;
        let file = match target {
            Some(path) => session.save_as(path)?,
            None => session.save()?,
        };
        self.note_session();
        Ok(file)
    }

    /// Passa a editar o álbum novo e já o grava em `file` (spec NP, regra 8): ele entra no
    /// topo dos recentes na hora.
    pub fn adopt_saved(&mut self, project: Project, file: &Path) -> Result<PathBuf, EditorError> {
        // A pasta sugerida (`Documentos\Lamina`) pode ainda não existir. Se não der para
        // criá-la, o salvar abaixo devolve o erro de gravação.
        if let Some(folder) = file.parent() {
            let _ = std::fs::create_dir_all(folder);
        }
        self.adopt(project);
        self.save_to(Some(file))
    }

    /// "Fechar álbum": guarda a lâmina em que parou e volta à tela inicial. A pergunta de salvar
    /// vem antes, pela janela (regra 6). `false` se não havia álbum.
    pub fn close_album(&mut self) -> bool {
        if self.session.is_none() {
            return false;
        }
        self.remember_current();
        self.forget_view();
        self.session = None;
        true
    }

    /// Guarda onde parou (fechar, abrir outro e sair do Lamina).
    pub fn remember_current(&mut self) {
        let Some((file, current, on_cover)) = self.session.as_ref().and_then(|s| {
            Some((
                s.file()?.to_path_buf(),
                s.last_spread(),
                s.stopped_on_cover(),
            ))
        }) else {
            return;
        };
        if self.recents.remember(&file, current, on_cover) {
            self.change_recents(|_| ());
        }
    }

    /// O que a miniatura precisa (o projeto e onde gravar), para a janela gerar fora da linha
    /// dela depois de salvar (regra 5). Nenhum quando não há arquivo ou lista em disco.
    pub fn thumbnail_job(&self) -> Option<(Arc<Project>, PathBuf)> {
        let session = self.session.as_ref()?;
        let target = self.recents.thumb_path(session.file()?)?;
        Some((session.album_shared(), target))
    }

    /// Onde gravar a miniatura grande do álbum aberto (spec DI, regra 14).
    pub fn big_thumbnail_target(&self) -> Option<PathBuf> {
        self.recents.big_thumb_path(self.session.as_ref()?.file()?)
    }

    /// Acabou de abrir: vai para onde parou, a capa ou a lâmina (ou a última, se hoje há menos), e
    /// põe o álbum no topo da lista.
    pub(super) fn after_open(&mut self) {
        let saved = self
            .session
            .as_ref()
            .and_then(|s| self.recents.find(s.file()?))
            .map(|r| (r.current, r.on_cover));
        if let (Some((current, _)), Some(session)) = (saved, self.session.as_mut()) {
            session.go_to(current.min(session.spread_count().saturating_sub(1)));
        }
        if saved.is_some_and(|(_, on_cover)| on_cover) {
            // Se a capa saiu do arquivo, fica na lâmina guardada.
            self.go_to_cover();
        }
        self.note_session();
        self.warn_missing_fonts();
    }

    /// O aviso das fontes que o álbum usa e este computador não tem (spec FS, regra 5).
    fn warn_missing_fonts(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        let missing = crate::font_picker::missing_fonts(session.album());
        if let Some(message) = crate::font_picker::missing_fonts_notice(&missing) {
            self.notice = Some(match self.notice.take() {
                Some(before) => format!(
                    "{before}
{message}"
                ),
                None => message,
            });
        }
    }

    /// Põe a sessão no topo da lista, com nome, contagens e a lâmina atual, e grava.
    pub(super) fn note_session(&mut self) {
        let Some(recent) = self.session.as_ref().and_then(recent_of) else {
            return;
        };
        self.change_recents(|s| s.put(recent));
    }
}

fn recent_of(session: &Session) -> Option<Recent> {
    // O álbum, não a projeção: com a capa aberta, ela tem a medida e a lâmina só da capa.
    let album = session.album();
    let pages = album.spec.edges().active_pages(0, album.spreads.len());
    let cover_image = shows_cover(album);
    Some(Recent {
        fold: !cover_image && pages.left && pages.right,
        path: session.file()?.to_path_buf(),
        name: session.title().to_owned(),
        opened_at: now(),
        spreads: album.spreads.len(),
        photos: album.photos.len(),
        current: session.last_spread(),
        pinned: false,
        on_cover: session.stopped_on_cover(),
        cover_image,
    })
}
