//! A grade do painel de fotos na janela (spec PF). A largura, a altura e a rolagem que a grade
//! informa viram as linhas visíveis (as contas são do editor, `photo_grid`), os pedidos ao
//! trabalhador de miniaturas da TR e as peças que mostram imagem: só visível ± margem; as outras
//! voltam ao retângulo neutro (regra 14). O zoom, o laço e a divisória também chegam aqui.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use lamina_core::PhotoId;
use lamina_editor::ThumbList;
use lamina_editor::photo_grid::{
    Grid, lasso_scroll, shown_height, thumb_side, wheel_zoom, whole_px, zoom_at_pointer,
    zoom_keeping_top,
};

use slint::ComponentHandle;

use crate::AppWindow;
use crate::state::App;

type Shared = Rc<RefCell<App>>;

/// O que a grade mostra agora, em px lógicos.
#[derive(Debug, Clone, Default)]
pub struct GridView {
    pub width: f32,
    pub height: f32,
    pub scroll: f32,
    /// As posições da lista com miniatura na tela (visível ± margem).
    pub window: Range<usize>,
    /// A altura da janela e o que nela não é nem a lâmina nem a grade (barras e cabeçalhos).
    pub room: f32,
    pub chrome: f32,
    /// A altura pedida pela divisória enquanto ela é arrastada.
    pub drag: Option<f32>,
    /// As linhas visíveis, as colunas e as fotos da última vez que as miniaturas foram pedidas.
    pub shown: Option<(Range<usize>, usize, usize)>,
}

impl App {
    /// A altura da grade dentro dos limites da janela de agora (regra 5): a da divisória durante
    /// o arrasto, senão a guardada (ou a padrão). O arquivo só muda ao soltar.
    pub fn show_grid_height(&mut self) {
        let view = &self.grid_view;
        let stored = view
            .drag
            .or_else(|| self.editor.photos_height().map(|h| h as f32));
        let zoom = self.editor.photos_zoom();
        let height = shown_height(stored, view.room, view.chrome, zoom);
        if let Some(w) = self.window.upgrade() {
            w.set_photo_shown(height);
        }
    }

    /// A grade da lista mostrada, na largura e no zoom de agora.
    pub fn grid(&self) -> Grid {
        Grid::new(
            self.shown_count(),
            self.grid_view.width,
            self.editor.photos_zoom(),
        )
    }

    /// Põe a linha da primeira foto visível guardada no topo (outro zoom, outra largura, a aba
    /// trocada ou o painel aberto de novo; regra 7). Sem ela na lista, volta ao topo.
    pub fn restore_grid_scroll(&mut self) {
        let grid = self.grid();
        let anchor = self.editor.grid_anchor();
        let index = anchor.and_then(|id| self.shown_photos().iter().position(|p| *p == id));
        let scroll = index.map_or(0.0, |i| grid.scroll_to(i, self.grid_view.height));
        if index.is_none() {
            self.editor.set_grid_anchor(None);
        }
        self.grid_view.scroll = scroll;
        if let Some(w) = self.window.upgrade() {
            w.set_photo_scroll(scroll);
        }
    }

    /// Leva à tela as linhas visíveis: pede as miniaturas na ordem do plano e troca as imagens só
    /// das peças que entraram ou saíram de visível ± margem (`full`: confere todas, depois de a
    /// lista mudar).
    pub fn show_grid(&mut self, full: bool) {
        let grid = self.grid();
        let visible = grid.visible(self.grid_view.scroll, self.grid_view.height);
        // Rolar dentro da mesma linha não muda nada: a maior parte dos quadros da rolagem.
        let same = (visible.clone(), grid.columns, grid.count);
        if !full && self.grid_view.shown == Some(same.clone()) {
            return;
        }
        self.grid_view.shown = Some(same);
        if let Some(w) = self.window.upgrade() {
            w.set_photo_columns(i32::try_from(grid.columns).unwrap_or(1));
        }
        let order: Vec<PhotoId> = grid
            .thumb_order(visible.clone())
            .into_iter()
            .filter_map(|i| self.shown_at(i))
            .collect();
        let Some(session) = self.editor.session() else {
            return;
        };
        self.images
            .thumbs_for(ThumbList::Browser, session.album(), &order);
        let old = std::mem::replace(&mut self.grid_view.window, grid.window(visible));
        let new = self.grid_view.window.clone();
        let touched: Vec<usize> = if full {
            (0..self.lists.browser.count()).collect()
        } else {
            old.clone().chain(new.clone()).collect()
        };
        for i in touched {
            let inside = new.contains(&i);
            if !full && inside == old.contains(&i) {
                continue;
            }
            let thumb = inside
                .then(|| {
                    self.lists
                        .browser
                        .key(i)
                        .and_then(|id| self.images.thumb(*id))
                })
                .flatten();
            self.lists.browser.update(i, |row| {
                let has = row.img.size().width > 0;
                match &thumb {
                    Some(img) if !has => row.img = img.clone(),
                    None if has => row.img = slint::Image::default(),
                    _ => {}
                }
            });
        }
    }

    /// A rolagem mudou pelo fotógrafo: a primeira foto visível passa a ser a da linha do topo,
    /// se a guardada não está nela.
    /// As miniaturas no tamanho da peça (zoom e escala da tela; spec PF, seção 13): mudou o lado,
    /// as listas são refeitas e pedem de novo.
    pub(crate) fn fit_thumbs(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        let side = thumb_side(self.editor.photos_zoom(), w.window().scale_factor());
        if self.images.set_thumb_side(side) {
            self.grid_view.shown = None;
            self.grid_view.window = 0..0;
            self.lists.reset();
            self.refresh_all();
        }
    }

    fn follow_scroll(&mut self) {
        let grid = self.grid();
        let top = grid.row_range(grid.top_row(self.grid_view.scroll));
        let anchor = self.editor.grid_anchor();
        let kept = anchor.is_some_and(|id| top.clone().any(|i| self.shown_at(i) == Some(id)));
        if !kept {
            self.editor.set_grid_anchor(self.shown_at(top.start));
        }
    }

    /// O zoom mudou (controle ou Ctrl+roda): grava, recalcula as colunas e a rolagem que mantém
    /// o lugar e mostra.
    fn zoom_to(&mut self, zoom: i64, pointer: Option<(f32, f32)>) {
        let old = self.grid();
        let zoom = self.editor.set_photos_zoom(zoom);
        let new = self.grid();
        let (scroll, height) = (self.grid_view.scroll, self.grid_view.height);
        let scroll = match pointer {
            Some(p) => zoom_at_pointer(&old, &new, scroll, p).min(new.max_scroll(height)),
            None => zoom_keeping_top(&old, &new, scroll, height),
        };
        self.grid_view.scroll = scroll;
        if let Some(w) = self.window.upgrade() {
            w.set_photo_zoom(zoom as f32);
            w.set_photo_scroll(scroll);
        }
        self.editor.set_grid_anchor(None);
        self.follow_scroll();
        self.fit_thumbs();
        self.show_grid(true);
        // Uma linha mínima depende do zoom.
        self.show_grid_height();
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    window.on_photo_lasso_speed(lasso_scroll);
    let a = app.clone();
    window.on_photo_room(move |room, chrome| {
        let mut app = a.borrow_mut();
        if let Some(w) = app.window.upgrade() {
            w.set_strip_height(lamina_editor::spread_strip_height(room));
        }
        app.grid_view.room = room;
        app.grid_view.chrome = chrome;
        app.show_grid_height();
    });
    let a = app.clone();
    window.on_photo_height_drag(move |height| {
        let mut app = a.borrow_mut();
        app.grid_view.drag = Some(height);
        app.show_grid_height();
    });
    let a = app.clone();
    window.on_photo_view(move |width, height, scroll| {
        let mut app = a.borrow_mut();
        let before = app.grid().columns;
        app.grid_view.width = width;
        app.grid_view.height = height;
        app.grid_view.scroll = scroll;
        let shown = app.window.upgrade().map(|w| w.get_photo_columns());
        let columns = i32::try_from(app.grid().columns).unwrap_or(1);
        if app.grid().columns != before || shown != Some(columns) {
            app.restore_grid_scroll();
        } else {
            app.follow_scroll();
        }
        // Mudando o tamanho da janela, a grade redesenha uma vez só, no fim (spec FB, regra 6).
        if !app.waits.resizing() {
            app.fit_thumbs();
            app.show_grid(false);
        }
    });
    let a = app.clone();
    window.on_photo_zoom_to(move |zoom| {
        let mut app = a.borrow_mut();
        if u32::try_from(zoom).ok() != Some(app.editor.photos_zoom()) {
            app.zoom_to(i64::from(zoom), None);
        }
    });
    let a = app.clone();
    window.on_photo_zoom_wheel(move |up, x, y| {
        let mut app = a.borrow_mut();
        let zoom = wheel_zoom(app.editor.photos_zoom(), up);
        if zoom != app.editor.photos_zoom() {
            app.zoom_to(i64::from(zoom), Some((x, y)));
        }
    });
    let a = app.clone();
    window.on_photo_lasso(move |x0, y0, x1, y1, add| {
        let edit = |app: &mut App| {
            let shown = app.shown_photos();
            let picked: Vec<PhotoId> = app
                .grid()
                .lasso((x0, y0), (x1, y1))
                .into_iter()
                .filter_map(|i| shown.get(i).copied())
                .collect();
            app.editor.lasso_photos(&picked, add);
            app.grid_focus = true;
            crate::photo_menu::refresh(app);
            true
        };
        let show = |app: &mut App| {
            app.refresh_photos();
            app.refresh_current();
        };
        a.borrow_mut().timed("Laço de fotos", edit, show);
    });
    let a = app.clone();
    window.on_photo_height_set(move |height| {
        let mut app = a.borrow_mut();
        app.grid_view.drag = None;
        app.editor.set_photos_height(whole_px(height));
        app.show_grid_height();
    });
    let a = app.clone();
    window.on_photo_height_reset(move || {
        let mut app = a.borrow_mut();
        app.grid_view.drag = None;
        app.editor.reset_photos_height();
        app.show_grid_height();
    });
}
