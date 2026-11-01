//! Fotos nos quadros pelo editor (spec FQ): quadro selecionado, seleção de fotos do navegador e
//! as edições de fotos, lâminas e cortes. As contas estão em [`crate::edits`].

use lamina_core::{PhotoId, Project};

use super::Editor;
use crate::EditorError;
use crate::edits::{
    capacity, delete_spread, insert_spread, layout_with, move_cut, move_spread, place_photo,
    remove_photos, swap_photos,
};
use crate::suggestions::{Selection, Unavailable};

impl Editor {
    pub fn selected_frame(&self) -> Option<usize> {
        self.selected_frame
    }

    /// Clique num quadro da lâmina atual (ou `None` para desmarcar).
    pub fn select_frame(&mut self, frame: Option<usize>) {
        if frame != self.crop_frame {
            self.exit_crop();
        }
        self.selected_frame = frame;
    }

    /// Fotos selecionadas no navegador, na ordem de captura.
    pub fn selection(&self) -> &[PhotoId] {
        &self.selection
    }

    pub fn set_selection(&mut self, photos: Vec<PhotoId>) {
        self.selection = photos;
    }

    /// Clique na foto `index` da lista `shown` do navegador: sozinho seleciona só ela; com Ctrl,
    /// acrescenta ou retira; com Shift, seleciona o intervalo desde o último clique. A seleção
    /// fica na ordem da lista.
    pub fn click_photo(&mut self, shown: &[PhotoId], index: usize, ctrl: bool, shift: bool) {
        let Some(&id) = shown.get(index) else {
            return;
        };
        let anchor = self.anchor.and_then(|a| shown.iter().position(|p| *p == a));
        let mut chosen: Vec<PhotoId> = match (ctrl, shift, anchor) {
            (_, true, Some(a)) => shown[a.min(index)..=a.max(index)].to_vec(),
            (true, _, _) if self.selection.contains(&id) => self
                .selection
                .iter()
                .copied()
                .filter(|p| *p != id)
                .collect(),
            (true, _, _) => self.selection.iter().copied().chain([id]).collect(),
            _ => vec![id],
        };
        chosen.sort_by_key(|p| shown.iter().position(|s| s == p));
        if !shift {
            self.anchor = Some(id);
        }
        self.selection = chosen;
    }

    /// A seleção do navegador como seleção do motor, com as páginas da lâmina atual (regra 5).
    pub(super) fn photo_selection(&self) -> Result<Selection, Unavailable> {
        let session = self.session.as_ref().ok_or(Unavailable::NoPhotos)?;
        let (project, index) = (session.project(), session.current());
        let cap = capacity(project, index);
        if cap == 0 && !project.spreads.is_empty() {
            return Err(Unavailable::BlankSpread);
        }
        if self.selection.len() > cap {
            return Err(Unavailable::TooMany(cap));
        }
        let aspects = self
            .selection
            .iter()
            .filter_map(|id| {
                project
                    .photo(*id)
                    .ok()
                    .map(lamina_core::Photo::aspect_ratio)
            })
            .collect();
        let active = project.content_pages(index);
        Ok(Selection {
            photos: self.selection.clone(),
            aspects,
            active,
            whole: true,
            // Álbum sem lâminas: o espaço do álbum (spec TR, regra 10).
            gap: project
                .spreads
                .get(index)
                .map_or(project.gap, |s| project.spread_gap(s)),
        })
    }

    /// Edição na lâmina atual com rótulo; `true` = mudou.
    pub(super) fn change(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Project, usize) -> Result<(), EditorError>,
    ) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let index = session.current();
        match session.edit(label, |p| f(p, index)) {
            Ok(()) => true,
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }

    /// Solta `photo` no quadro `frame` da lâmina atual (regra 1).
    pub fn drop_photo(&mut self, frame: usize, photo: PhotoId) -> bool {
        self.change("Colocar foto", |p, i| place_photo(p, i, frame, photo))
    }

    /// Solta a foto do quadro `from` no quadro `to` (regra 2).
    pub fn drop_frame(&mut self, from: usize, to: usize) -> bool {
        from != to && self.change("Trocar fotos", |p, i| swap_photos(p, i, from, to))
    }

    pub(super) fn can_clear(&self) -> bool {
        let (Some(s), Some(f)) = (&self.session, self.selected_frame) else {
            return false;
        };
        // Sem `[]`: depois de desfazer a última lâmina, o quadro selecionado não existe mais
        // (spec TR, regra 10).
        s.project()
            .spreads
            .get(s.current())
            .and_then(|spread| spread.frames.get(f))
            .is_some_and(|fr| matches!(fr.content, lamina_core::FrameContent::Photo { .. }))
    }

    pub(super) fn clear_selected(&mut self) -> bool {
        let Some(frame) = self.selected_frame else {
            return false;
        };
        let done = self.change("Tirar foto", |p, i| remove_photos(p, i, &[frame]));
        if done {
            self.selected_frame = None;
        }
        done
    }

    /// Enter ou clique numa opção com seleção: a lâmina atual só com as fotos dela (regra 5).
    pub(super) fn build_from_selection(&mut self, option: usize) -> bool {
        // No álbum em branco, Enter faz o mesmo que soltar na lâmina vazia (spec BR, regra 4).
        if self.is_blank() {
            let photos = self.selection.clone();
            return self.first_spread(&photos);
        }
        let panel = self.panel(crate::Part::Spread);
        let (Some(sel), Some(c)) = (panel.selection, panel.options.get(option).cloned()) else {
            return false;
        };
        // A mesma montagem da prévia na lâmina (spec FL, regra 3).
        let done = self.change("Montar lâmina", |p, i| {
            super::band::apply_choice(p, i, (&sel, &c), true)
        });
        if done {
            self.selection.clear();
        }
        done
    }

    /// Shift+Enter: lâmina nova depois da atual com a seleção, na 1ª opção (regra 5).
    pub(super) fn new_spread_from_selection(&mut self) -> bool {
        let photos = self.selection.clone();
        let mut at = 0;
        let label = self.new_spread_label("Nova lâmina com a seleção");
        let done = self.change(&label, |p, i| {
            at = insert_spread(p, i);
            layout_with(p, at, &photos)
        });
        if done {
            self.selection.clear();
            self.go(at);
        }
        done
    }

    pub(super) fn new_spread(&mut self) -> bool {
        let mut at = 0;
        let label = self.new_spread_label("Nova lâmina");
        let done = self.change(&label, |p, i| {
            at = insert_spread(p, i);
            Ok(())
        });
        if done {
            self.go(at);
        }
        done
    }

    pub(super) fn delete_current(&mut self) -> bool {
        let done = self.change("Apagar lâmina", delete_spread);
        if done && let Some(s) = self.session.as_mut() {
            let last = s.spread_count().saturating_sub(1);
            s.go_to(s.current().min(last));
        }
        done
    }

    /// Muda a lâmina atual para a posição `to` (arrastar na faixa; regra 7).
    pub fn move_current_to(&mut self, to: usize) -> bool {
        let from = self.session.as_ref().map_or(0, crate::Session::current);
        let label = format!("Mover lâmina · {} → {}", from + 1, to + 1);
        let done = self.change(&label, |p, i| move_spread(p, i, to));
        if done {
            self.go(to);
        }
        done
    }

    pub(super) fn move_by(&mut self, delta: isize) -> bool {
        let Some(s) = &self.session else {
            return false;
        };
        let (current, count) = (s.current(), s.spread_count());
        current
            .checked_add_signed(delta)
            .is_some_and(|to| to < count && self.move_current_to(to))
    }

    /// Arrasta o corte depois da lâmina `k` em `delta` fotos (regra 8).
    pub fn drag_cut(&mut self, k: usize, delta: i32) -> bool {
        let mut moved = false;
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let result = session.edit("Mover corte", |p| {
            moved = move_cut(p, k, delta)?;
            if moved {
                Ok(())
            } else {
                Err(EditorError::NoFrame)
            }
        });
        result.is_ok() && moved
    }

    /// A lâmina nova entra depois da atual (num álbum sem lâminas, é a primeira): o rótulo diz a
    /// posição dela.
    fn new_spread_label(&self, what: &str) -> String {
        let at = self
            .session
            .as_ref()
            .map_or(0, |s| (s.current() + 1).min(s.spread_count()))
            + 1;
        format!("{what} · lâmina {at}")
    }

    pub(super) fn go(&mut self, index: usize) {
        self.selected_frame = None;
        if let Some(s) = self.session.as_mut() {
            s.go_to(index);
        }
    }
}
