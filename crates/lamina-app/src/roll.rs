//! O rolo de lâminas na janela (spec RL; ADR 0012): só traduz. As revisões, o plano, as contas e a
//! imagem de cada lâmina são do `lamina-editor` (`motion::roll*`, `roll_image`); aqui ficam a
//! fila do trabalhador, as imagens guardadas, as linhas do modelo e o destino da rolagem. É o
//! padrão desde o G1 (regra 28), que se repete antes da beta.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::time::Duration;

use lamina_core::{PhotoId, Project, SpreadId};
use lamina_editor::motion::{
    Revisions, Roll, RollPlan, RollScroll, RollView, SCREEN_MAX, StripRow, Tier, textured,
    visible_range,
};
use lamina_editor::{Cache, screens::Arrival};
use slint::ComponentHandle;

use crate::mailbox::Mailbox;
use crate::roll_cells::{Cells, Stored};
use crate::roll_worker::{Job, Painted, RollWorker};
use crate::state::App;
use crate::{AppWindow, RollUi};

/// Pausa depois da última edição (ou do último redimensionamento) que pede a imagem de tela.
pub const QUIET: Duration = Duration::from_millis(300);

/// O estado do rolo na janela.
pub struct RollState {
    revisions: Revisions,
    plan: RollPlan,
    pub(crate) worker: RollWorker,
    pub scroll: RollScroll,
    store: BTreeMap<SpreadId, Stored>,
    held: BTreeMap<SpreadId, Stored>,
    pub(crate) cells: Cells,
    project: Option<Arc<Project>>,
    missing: Arc<Vec<PhotoId>>,
    warned: Vec<bool>,
    pub(crate) visible: Option<RangeInclusive<usize>>,
    /// Largura física da imagem de tela (muda 300 ms depois de redimensionar).
    pub(crate) width_px: u32,
    /// Quando foi a última edição (tempo da janela), para a imagem da lâmina atual.
    edited_at: Duration,
    pub(crate) sliding: bool,
    /// A lâmina atual da última vez (de onde sai o deslize).
    pub(crate) shown: Option<usize>,
    pub(crate) stats: crate::roll_stats::RollStats,
    /// Os botões da barra mostrados agora.
    pub(crate) buttons: Vec<crate::RollButton>,
    /// A linha do tempo na tela, só perto do trecho visível (regra 22).
    pub(crate) line: crate::line_window::LineWindow,
}

impl RollState {
    pub fn new(cache: &Cache, mailbox: &Mailbox) -> Self {
        RollState {
            revisions: Revisions::default(),
            plan: RollPlan::default(),
            worker: RollWorker::new(cache, mailbox),
            scroll: RollScroll::default(),
            store: BTreeMap::new(),
            held: BTreeMap::new(),
            cells: Cells::new(),
            project: None,
            missing: Arc::default(),
            warned: Vec::new(),
            visible: None,
            width_px: 0,
            edited_at: Duration::ZERO,
            sliding: false,
            shown: None,
            stats: crate::roll_stats::RollStats::default(),
            buttons: Vec::new(),
            line: crate::line_window::LineWindow::new(),
        }
    }

    /// Outro álbum: uma época nova, sem animação (RL-B22).
    pub fn new_album(&mut self) {
        self.worker.forget_images();
        self.revisions.clear();
        self.plan.new_album();
        self.store.clear();
        self.held.clear();
        self.cells.clear();
        self.visible = None;
        self.shown = None;
        self.scroll = RollScroll::default();
    }

    /// As lâminas, na ordem do álbum (as da última edição vista).
    pub fn order(&self) -> &[SpreadId] {
        self.revisions.order()
    }

    /// A lâmina tem a imagem de tela da revisão de agora (regra 13).
    pub fn sharp(&self, id: SpreadId) -> bool {
        self.plan.sharp(id)
    }

    /// O rolo como a janela desenha agora (a área, a largura da lâmina e quantas há).
    pub fn geometry(w: &AppWindow) -> Roll {
        let ui = w.global::<RollUi>();
        let width = f64::from(ui.get_width());
        Roll {
            area: (f64::from(ui.get_area_w()), f64::from(ui.get_area_h())),
            width,
            height: width * f64::from(ui.get_aspect()),
            count: usize::try_from(ui.get_count()).unwrap_or(0),
        }
    }
}

/// Depois de uma edição (a faixa já calculou as linhas): as revisões, o álbum que o trabalhador
/// desenha e os pedidos.
pub fn edited(app: &mut App, rows: &[(SpreadId, StripRow)]) {
    let Some(session) = app.editor.session() else {
        return;
    };
    let project = session.album();
    let changed = app.roll.revisions.update(rows, &project.spec);
    let count_changed = app.roll.warned.len() != rows.len();
    app.roll.warned = rows.iter().map(|(_, r)| r.warned).collect();
    if !changed.is_empty() || count_changed || app.roll.project.is_none() {
        app.roll.project = Some(session.album_shared());
        app.roll.missing = Arc::new(session.missing().to_vec());
        app.roll.edited_at = app.clock.now();
        crate::mailbox::later(QUIET + Duration::from_millis(20), quiet);
    }
    if let Some(w) = app.window.upgrade() {
        crate::roll_connect::badges(&w, app);
    }
    sync(app);
}

/// 300 ms sem edição: a imagem de tela da lâmina atual pode ser pedida (regra 7).
fn quiet(app: &mut App) {
    if app.clock.now().saturating_sub(app.roll.edited_at) >= QUIET {
        ask(app);
    }
}

/// As linhas do modelo e os pedidos para o que se vê agora.
pub fn sync(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let roll = RollState::geometry(&w);
    // Antes de a janela medir a área, não há rolo para pedir.
    if roll.width < 1.0 || roll.area.0 < 1.0 {
        return;
    }
    let pos = f64::from(w.global::<RollUi>().get_pos());
    let last = roll.count.checked_sub(1);
    let visible = visible_range(&roll, pos)
        .zip(last)
        .map(|(v, last)| (*v.start()).min(last)..=(*v.end()).min(last));
    let Some(visible) = visible else {
        return;
    };
    let keep = visible.start().saturating_sub(2)..=visible.end() + 2;
    let want = textured(&visible, roll.count);
    app.roll
        .cells
        .sync((keep, want), app.roll.revisions.order(), &app.roll.warned);
    if app.roll.visible.as_ref() != Some(&visible) {
        app.roll.visible = Some(visible);
        ask(app);
    }
}

/// Manda ao trabalhador os pedidos do plano (a fila dele troca inteira).
pub fn ask(app: &mut App) {
    let (Some(w), Some(visible), Some(project)) = (
        app.window.upgrade(),
        app.roll.visible.clone(),
        app.roll.project.clone(),
    ) else {
        return;
    };
    let Some(current) = app.session().map(lamina_editor::Session::current) else {
        return;
    };
    let ui = w.global::<RollUi>();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let center = f64::from(ui.get_pos()).round().max(0.0) as usize;
    if app.roll.width_px == 0 {
        app.roll.width_px = physical(&w);
    }
    let view = RollView {
        center,
        visible,
        current,
        current_idle: app.clock.now().saturating_sub(app.roll.edited_at) >= QUIET,
        width: app.roll.width_px,
    };
    let asks = app.roll.plan.go(&view, &app.roll.revisions);
    app.roll.stats.asked(asks.len());
    app.roll.worker.replace(Job {
        project,
        missing: app.roll.missing.clone(),
        shown: f64::from(ui.get_width()),
        asks,
    });
}

/// A largura física do lugar: a largura lógica × a escala da tela, com o teto de 2.048 px.
pub fn physical(w: &AppWindow) -> u32 {
    let logical = w.global::<RollUi>().get_width() * w.window().scale_factor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = logical.round().max(1.0) as u32;
    px.min(SCREEN_MAX)
}

/// Uma imagem chegou do trabalhador.
pub fn arrived(app: &mut App, painted: Result<Painted, lamina_editor::HumanMessage>) {
    let painted = match painted {
        Ok(p) => p,
        Err(problem) => {
            // Não foi uma ação do fotógrafo: só o registro de erros (regra 6 da spec, EH).
            app.editor.record(&problem);
            return;
        }
    };
    let tier = u8::from(painted.got.ask.tier == Tier::Screen);
    let key = (painted.got.ask.revision, tier, painted.got.ask.width);
    let stored = Stored {
        key,
        columns: painted.columns,
    };
    let spread = painted.got.ask.spread;
    app.roll.stats.painted(painted.ms);
    let landing = app.roll.plan.arrived(painted.got, app.roll.sliding);
    for gone in &landing.evict {
        app.roll.store.remove(gone);
        app.roll.held.remove(gone);
        app.roll.cells.forget(*gone);
    }
    match landing.arrival {
        Arrival::Show if !landing.evict.contains(&spread) => {
            app.roll.store.insert(spread, stored);
            check_sharp(app);
            request_frame(app);
        }
        Arrival::Hold if !landing.evict.contains(&spread) => {
            app.roll.held.insert(spread, stored);
        }
        _ => {}
    }
}

/// A foto nítida `id` entra nas vagas da lâmina viva que mostravam a miniatura dela, sem refazer a
/// lâmina (a versão da vaga troca, e a tela sobe só esta textura).
fn reveal(app: &mut App, id: PhotoId, img: &slint::Image) {
    use slint::Model as _;
    let (Ok(thumb), Ok(sharp)) = (i32::try_from(id.get() * 4), i32::try_from(id.get() * 4 + 1))
    else {
        return;
    };
    let model = app.slot_rows.model();
    for r in 0..model.row_count() {
        if let Some(mut f) = model.row_data(r)
            && f.photo == thumb
        {
            f.img = img.clone();
            f.photo = sharp;
            model.set_row_data(r, f);
        }
    }
}

/// Medição: as vizinhas à vista já têm a imagem de tela da revisão de agora?
fn check_sharp(app: &mut App) {
    let (Some(visible), Some(current)) = (
        app.roll.visible.clone(),
        app.session().map(lamina_editor::Session::current),
    ) else {
        return;
    };
    if !app.roll.stats.waiting() {
        return;
    }
    let order = app.roll.revisions.order();
    let sharp = visible
        .filter(|i| *i != current)
        .all(|i| order.get(i).is_some_and(|id| app.roll.plan.sharp(*id)));
    if sharp {
        let now = app.clock.now();
        app.roll.stats.sharp(now);
    }
}

/// O deslize terminou: as imagens seguradas entram (regra 8).
pub fn slide_ended(app: &mut App) {
    app.roll.sliding = false;
    for id in app.roll.plan.slide_ended() {
        if let Some(s) = app.roll.held.remove(&id) {
            app.roll.store.insert(id, s);
        }
    }
    check_sharp(app);
    request_frame(app);
}

/// Pede um quadro para a porta dos envios andar.
fn request_frame(app: &App) {
    if let Some(w) = app.window.upgrade() {
        w.window().request_redraw();
    }
}

/// Depois de cada quadro desenhado: as linhas, a porta dos envios e a medição.
pub fn frame_drawn(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<RollUi>();
    let at = app.clock.now();
    app.roll
        .stats
        .frame(at, ui.get_moving(), app.roll.plan.bytes());
    sync(app);
    let roll = RollState::geometry(&w);
    let pos = f64::from(ui.get_pos());
    let seen = |index: usize, column: usize, columns: usize| {
        crate::roll_connect::column_seen(&roll, pos, (index, column, columns))
    };
    #[allow(clippy::cast_precision_loss)]
    let near = |index: usize| (index as f64 - pos).abs();
    // Uma coluna nova pede o próximo quadro sozinha (a linha do modelo mudou); sem coluna nova,
    // nada desenha com a janela parada (regra 27).
    let column = app.roll.cells.frame(&app.roll.store, &seen, &near);
    // As fotos nítidas da lâmina viva, uma por quadro e nunca no mesmo quadro de uma coluna.
    if !column {
        let ids = app.session().map_or_else(Vec::new, |s| {
            lamina_editor::edits::spread_photos(s.project(), s.current())
        });
        if let Some((id, img)) = app.images.reveal_next(&ids) {
            reveal(app, id, &img);
        }
    }
}
