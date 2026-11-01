//! As regras da lista (spec TI, regras 1 a 4): entrar no topo uma vez só, sair o mais antigo não
//! fixado, fixar até 8, lembrar a lâmina, trocar o caminho e tirar da lista.

use std::path::Path;

use super::{MAX_PINNED, MAX_RECENTS, PINNED_FULL, Recent, RecentsStore, key, normalized};

impl RecentsStore {
    /// Abriu, salvou ou criou: o álbum vai para o topo dos não fixados (o fixado fica onde está),
    /// sem repetir o arquivo, e a lista volta a ter no máximo 20 (regras 1 e 2).
    pub fn put(&mut self, mut recent: Recent) {
        recent.path = normalized(&recent.path);
        let at = self.position(&recent.path);
        if let Some(old) = at.map(|i| self.list.remove(i)) {
            recent.pinned = old.pinned;
        }
        let place = match at {
            Some(i) if recent.pinned => i,
            _ => self.pinned_count(),
        };
        self.list.insert(place.min(self.list.len()), recent);
        while self.list.len() > MAX_RECENTS {
            let Some(oldest) = self.list.iter().rposition(|r| !r.pinned) else {
                break;
            };
            let gone = self.list.remove(oldest);
            self.forget_thumb(&gone.path);
        }
    }

    /// Guarda onde o fotógrafo parou: a lâmina (regra 3) e se estava na capa (spec DI, regra 13).
    /// `false` se o álbum não está na lista.
    pub fn remember(&mut self, file: &Path, current: usize, on_cover: bool) -> bool {
        let Some(i) = self.position(file) else {
            return false;
        };
        self.list[i].current = current;
        self.list[i].on_cover = on_cover;
        true
    }

    /// Fixa o álbum `index`: ele vai para o fim dos fixados. O nono é recusado (regra 2).
    pub fn pin(&mut self, index: usize) -> Result<(), String> {
        if index >= self.list.len() || self.list[index].pinned {
            return Ok(());
        }
        if self.pinned_count() >= MAX_PINNED {
            return Err(PINNED_FULL.to_owned());
        }
        let mut recent = self.list.remove(index);
        recent.pinned = true;
        self.list.insert(self.pinned_count(), recent);
        Ok(())
    }

    /// Desafixa o álbum `index`: ele volta para o meio dos outros, pela data em que foi aberto.
    pub fn unpin(&mut self, index: usize) {
        if index >= self.list.len() || !self.list[index].pinned {
            return;
        }
        let mut recent = self.list.remove(index);
        recent.pinned = false;
        let place = self
            .list
            .iter()
            .position(|r| !r.pinned && r.opened_at < recent.opened_at)
            .unwrap_or(self.list.len());
        self.list.insert(place, recent);
    }

    /// O alfinete do cartão: fixa ou desafixa.
    pub fn toggle_pin(&mut self, index: usize) -> Result<(), String> {
        match self.list.get(index) {
            Some(r) if r.pinned => {
                self.unpin(index);
                Ok(())
            }
            _ => self.pin(index),
        }
    }

    /// "Tirar da lista": some o cartão e a miniatura (regra 4).
    pub fn remove(&mut self, index: usize) {
        if index < self.list.len() {
            let gone = self.list.remove(index);
            self.forget_thumb(&gone.path);
        }
    }

    /// "Procurar…": o álbum `index` passa a apontar para `file`, mantendo o resto, e a
    /// miniatura vai junto. Se `file` já estava na lista, os dois viram um só (seção 6).
    pub fn relocate(&mut self, index: usize, file: &Path) {
        if index >= self.list.len() {
            return;
        }
        let old = self.list[index].path.clone();
        if let Some(other) = self.position(file).filter(|&i| i != index) {
            let twin = self.list.remove(other);
            self.forget_thumb(&twin.path);
        }
        let index = self.position(&old).unwrap_or(0);
        if let (Some(from), Some(to)) = (self.thumb_path(&old), self.thumb_path(file)) {
            // Sem miniatura antiga, o cartão mostra o desenho neutro até o próximo salvar.
            let _ = std::fs::rename(from, to);
        }
        if let Some(recent) = self.list.get_mut(index) {
            recent.path = normalized(file);
        }
    }

    /// "Limpar recentes não fixados…": ficam só os fixados (regra 7).
    pub fn clear_unpinned(&mut self) {
        let (kept, gone): (Vec<Recent>, Vec<Recent>) = self.list.drain(..).partition(|r| r.pinned);
        self.list = kept;
        for recent in gone {
            self.forget_thumb(&recent.path);
        }
    }

    /// Há algum álbum não fixado para limpar?
    pub fn has_unpinned(&self) -> bool {
        self.list.iter().any(|r| !r.pinned)
    }

    /// Lista lida do disco: fixados primeiro, sem repetir, até 20 e até 8 fixados.
    pub(super) fn tidy(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.list.retain(|r| seen.insert(key(&r.path)));
        self.list.sort_by_key(|r| !r.pinned);
        for recent in self.list.iter_mut().skip(MAX_PINNED) {
            recent.pinned = false;
        }
        self.list.truncate(MAX_RECENTS);
    }

    fn position(&self, file: &Path) -> Option<usize> {
        let key = key(file);
        self.list.iter().position(|r| super::key(&r.path) == key)
    }

    fn pinned_count(&self) -> usize {
        self.list.iter().filter(|r| r.pinned).count()
    }
}
