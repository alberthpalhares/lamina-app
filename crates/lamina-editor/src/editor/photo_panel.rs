//! O painel de fotos pelo editor (spec PF): a aba mostrada, o laço, "Selecionar todas as fotos
//! mostradas", a primeira foto visível da grade e o zoom e a altura guardados em `janela.json`.
//! As contas da grade estão em [`crate::photo_grid`].

use std::collections::HashSet;

use lamina_core::PhotoId;

use super::Editor;
use crate::ordering::{SortBy, move_to, sorted};
use crate::photo_grid::{clamp_zoom, default_height, shown_height, whole_px};
use crate::photos::{Filter, photo_list};

impl Editor {
    /// A aba do painel de fotos (Todas, Usadas ou Não usadas).
    pub fn photo_filter(&self) -> Filter {
        self.photo_filter
    }

    /// Troca a aba: a seleção continua, inclusive as fotos que a aba esconde (regra 11). A
    /// ampliada fecha (a lista de navegação mudou, spec VF) e a prévia esconde se a foto
    /// escolhida não está na aba (regra 1).
    pub fn set_photo_filter(&mut self, filter: Filter) {
        self.photo_filter = filter;
        self.photo_view.open = false;
        let stays = self
            .photo_view
            .chosen
            .is_some_and(|id| self.shown_photos().contains(&id));
        if !stays {
            self.photo_view.chosen = None;
        }
    }

    /// O critério de ordenação do painel (spec OF); fica na memória, não no arquivo.
    pub fn photo_sort(&self) -> SortBy {
        self.photo_sort
    }

    /// Troca o critério de ordenação (spec OF, regra 1). A prévia esconde se a foto escolhida
    /// sai da aba; a seleção continua.
    pub fn set_photo_sort(&mut self, sort: SortBy) {
        self.photo_sort = sort;
    }

    /// As fotos da aba, na ordem do critério escolhido (spec OF).
    pub fn shown_photos(&self) -> Vec<PhotoId> {
        self.session.as_ref().map_or_else(Vec::new, |s| {
            let project = s.project();
            let capture: Vec<PhotoId> = photo_list(project, self.photo_filter)
                .into_iter()
                .map(|item| item.id)
                .collect();
            if self.photo_sort == SortBy::Capture {
                return capture;
            }
            // Ordena pelo critério, mas só as fotos da aba atual.
            let in_tab: std::collections::HashSet<PhotoId> = capture.iter().copied().collect();
            let ordered: Vec<PhotoId> =
                sorted(project.photos.iter(), self.photo_sort, &project.photo_order)
                    .into_iter()
                    .map(|p| p.id)
                    .filter(|id| in_tab.contains(id))
                    .collect();
            ordered
        })
    }

    /// Move a foto `id` para a posição `to` na ordem personalizada (spec OF, regra 4). Grava no
    /// projeto como uma edição "Ordenar fotos". Só vale com o critério Personalizada.
    pub fn move_photo_to(&mut self, id: PhotoId, to: usize) -> bool {
        if self.photo_sort != SortBy::Custom {
            return false;
        }
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let capture: Vec<PhotoId> = s.project().photos.iter().map(|p| p.id).collect();
        let order = move_to(&s.project().photo_order, &capture, id, to);
        if order == s.project().photo_order {
            return false;
        }
        s.edit("Ordenar fotos", |p| {
            p.photo_order = order.clone();
            Ok::<(), crate::EditorError>(())
        })
        .is_ok()
    }

    /// O laço terminou sobre `picked` (regra 9): sem tecla, a seleção passa a ser elas (vazia se
    /// o laço não cruzou nenhuma); com Ctrl ou Shift (`add`), elas se somam. A seleção fica na
    /// ordem do painel; as fotos que a aba esconde continuam no começo, como estavam.
    pub fn lasso_photos(&mut self, picked: &[PhotoId], add: bool) {
        let kept: &[PhotoId] = if add { &self.selection } else { &[] };
        let chosen: HashSet<PhotoId> = kept.iter().chain(picked).copied().collect();
        let shown = self.shown_photos();
        let visible: HashSet<PhotoId> = shown.iter().copied().collect();
        let mut selection: Vec<PhotoId> = kept
            .iter()
            .copied()
            .filter(|id| !visible.contains(id))
            .collect();
        selection.extend(shown.into_iter().filter(|id| chosen.contains(id)));
        self.selection = selection;
    }

    /// "Selecionar todas as fotos mostradas" (regra 10): a seleção são as fotos da aba, na ordem.
    pub(super) fn select_all_photos(&mut self) -> bool {
        let shown = self.shown_photos();
        if shown.is_empty() {
            return false;
        }
        self.selection = shown;
        true
    }

    /// A primeira foto visível da grade, guardada pela foto (não em pixels) para voltar a ela
    /// depois de esconder o painel ou de mudar o zoom ou a largura (regra 7).
    pub fn grid_anchor(&self) -> Option<PhotoId> {
        self.grid_anchor
    }

    pub fn set_grid_anchor(&mut self, photo: Option<PhotoId>) {
        self.grid_anchor = photo;
    }

    /// O zoom das miniaturas (64 a 240 px).
    pub fn photos_zoom(&self) -> u32 {
        clamp_zoom(i64::from(self.strips.strips.photos_zoom))
    }

    /// Muda o zoom (preso à faixa) e grava em `janela.json`. Devolve o zoom que ficou.
    pub fn set_photos_zoom(&mut self, zoom: i64) -> u32 {
        let zoom = clamp_zoom(zoom);
        if zoom != self.strips.strips.photos_zoom {
            self.strips.strips.photos_zoom = zoom;
            self.save_strips();
        }
        zoom
    }

    /// A altura guardada do painel de fotos (`None`: a padrão, duas linhas).
    pub fn photos_height(&self) -> Option<u32> {
        self.strips.strips.photos_height
    }

    /// A divisória foi solta (ou o duplo clique voltou à padrão): grava a altura que ficou na
    /// tela (regra 5).
    pub fn set_photos_height(&mut self, height: u32) {
        if self.strips.strips.photos_height != Some(height) {
            self.strips.strips.photos_height = Some(height);
            self.save_strips();
        }
    }

    /// Duplo clique na divisória: volta à altura padrão (duas linhas no zoom atual) e grava.
    pub fn reset_photos_height(&mut self) -> u32 {
        let height = whole_px(default_height(self.photos_zoom()));
        self.set_photos_height(height);
        height
    }

    /// A altura que a grade mostra numa janela de altura `window`, com `chrome` de barras e
    /// cabeçalhos (os limites da regra 5; o arquivo não muda).
    pub fn shown_photos_height(&self, window: f32, chrome: f32) -> f32 {
        let stored = self.photos_height().map(|h| h as f32);
        shown_height(stored, window, chrome, self.photos_zoom())
    }
}
