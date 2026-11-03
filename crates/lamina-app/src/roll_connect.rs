//! Os gestos do rolo (spec RL, regras 11 a 21) e o que a janela mostra dele: a roda, o clique numa
//! vizinha, a barra, a pílula, a alça, a rolagem nas bordas e o ir para outra lâmina. As decisões
//! são do editor (`motion::roll_*`); aqui só se traduz.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::motion::{CrossHit, cross_target};
use lamina_editor::motion::{
    Roll, RollHit, RollMove, SPREAD_BAR_COMMANDS, TIMELINE, Transition, edge_speed, pill, reorder,
    roll_gap, roll_hit, roll_move, roll_shown, roll_width, spread_badge, timeline_rows,
    timeline_run, timeline_step,
};
use lamina_editor::{Command, CrossTarget, DropPlace, PxRect};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::roll::{QUIET, RollState, physical};
use crate::state::App;
use crate::{AppWindow, RollButton, RollHitData, RollUi, SpreadData, TimelineStep};

type Shared = Rc<RefCell<App>>;

/// Quanto cada pixel da roda anda (a roda dá 60 px lógicos por dente no Windows; como no G0).
const WHEEL_GAIN: f64 = 4.0;
/// O intervalo do relógio da rolagem nas bordas.
const EDGE_TICK: f64 = 0.016;

/// Leva o rolo ao destino `target` (lâminas) em `ms` milissegundos (zero = na hora).
fn kick(w: &AppWindow, target: f64, ms: u32) {
    let ui = w.global::<RollUi>();
    #[allow(clippy::cast_possible_truncation)]
    ui.set_target(target as f32);
    ui.set_kick_ms(i32::try_from(ms).unwrap_or(0));
    ui.set_kick(ui.get_kick().wrapping_add(1));
}

/// A coluna `column` (de `columns`) da lâmina `index` aparece na área com o rolo em `pos`?
pub fn column_seen(roll: &Roll, pos: f64, (index, column, columns): (usize, usize, usize)) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let share = roll.width / columns.max(1) as f64;
    #[allow(clippy::cast_precision_loss)]
    let x = roll.left(index, pos) + column as f64 * share;
    x < roll.area.0 && x + share > 0.0
}

/// O que a janela mostra do rolo: ligado ou não, a lâmina atual, a proporção, os botões, a
/// pílula e o destino (o rolo acompanha a lâmina atual até rolar à mão).
pub fn refresh(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<RollUi>();
    ui.set_on(roll_shown(&app.editor));
    let Some(session) = app.session() else {
        return;
    };
    let (current, count) = (session.current(), session.spread_count());
    let aspect = crate::convert::aspect(session.album());
    let rows = timeline_rows(session.album());
    let (x, width) = timeline_run(&rows, current, TIMELINE);
    #[allow(clippy::cast_possible_truncation)]
    {
        ui.set_run_x(x as f32);
        ui.set_run_w(width as f32);
    }
    let changed = ui.get_aspect().to_bits() != aspect.to_bits();
    ui.set_aspect(aspect);
    ui.set_count(i32::try_from(count).unwrap_or(0));
    ui.set_current(i32::try_from(current).unwrap_or(0));
    // A barra só muda quando a disponibilidade muda (refazer os ícones a cada navegação custa).
    let now = buttons(app);
    if app.roll.buttons != now {
        ui.set_buttons(ModelRc::new(VecModel::from(now.clone())));
        app.roll.buttons = now;
    }
    if changed {
        area_changed(app);
    }
    set_pill(app);
}

/// Os botões da barra, com a dica e a disponibilidade do menu (regra 15).
fn buttons(app: &App) -> Vec<RollButton> {
    SPREAD_BAR_COMMANDS
        .iter()
        .map(|c| {
            let key = c.shortcut().map(|s| s.text()).unwrap_or_default();
            RollButton {
                name: c.name().into(),
                tip: format!("{} ({key})", c.label()).into(),
                enabled: app.editor.available(*c),
            }
        })
        .collect()
}

/// O selo de cada lâmina (a barra mostra o da lâmina sob o mouse).
pub fn badges(w: &AppWindow, app: &App) {
    let Some(session) = app.session() else {
        return;
    };
    let (edges, count) = (session.album().spec.edges(), session.album().spreads.len());
    let badges: Vec<SharedString> = (0..count)
        .map(|i| spread_badge(edges, i, count).into())
        .collect();
    w.global::<RollUi>()
        .set_badges(ModelRc::new(VecModel::from(badges)));
}

/// A pílula "← Lâmina 5" quando a lâmina atual saiu de vista (regra 14).
pub fn set_pill(app: &App) {
    let (Some(w), Some(current)) = (
        app.window.upgrade(),
        app.session().map(lamina_editor::Session::current),
    ) else {
        return;
    };
    let p = app.roll.visible.as_ref().and_then(|v| pill(current, v));
    let ui = w.global::<RollUi>();
    ui.set_pill(
        p.as_ref()
            .map(|p| p.label.clone())
            .unwrap_or_default()
            .into(),
    );
    ui.set_pill_left(p.is_some_and(|p| p.left));
}

/// A área mudou de tamanho: a largura da lâmina na hora, sem animar; a imagem de tela é pedida de
/// novo 300 ms depois da última mudança (regra 25).
fn area_changed(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<RollUi>();
    let area = (f64::from(ui.get_area_w()), f64::from(ui.get_area_h()));
    let r = roll_width(area, 0.0, f64::from(ui.get_aspect()));
    #[allow(clippy::cast_possible_truncation)]
    ui.set_width(r.width as f32);
    ui.set_neighbors(r.neighbors);
    crate::mailbox::later(QUIET, |app| {
        if let Some(w) = app.window.upgrade() {
            app.roll.width_px = physical(&w);
        }
        crate::roll::ask(app);
    });
}

/// Ir para a lâmina `index` (regra 12): o rolo desliza até 2 posições do que se vê no centro;
/// mais longe, fica a troca de hoje (devolve `false` para ela). `old` é a lâmina viva que sai e
/// `from`, onde ela estava.
pub fn navigate(
    app: &mut App,
    w: &AppWindow,
    (index, transition): (usize, Transition),
    (old, from): (&SpreadData, Option<usize>),
) -> bool {
    let ui = w.global::<RollUi>();
    let seen = f64::from(ui.get_target());
    let movement = roll_move(seen, index, transition);
    #[allow(clippy::cast_precision_loss)]
    let to = index as f64;
    if movement == RollMove::Stay {
        // A mesma lâmina (uma edição): o rolo só volta a ela se estava acompanhando.
        if app.roll.scroll.follow && (seen - to).abs() > 1e-6 {
            app.roll.scroll.follow(index);
            kick(w, to, app.editor.durations().navigate);
        }
        return true;
    }
    app.roll.scroll.follow(index);
    app.images.reset_reveal();
    match movement {
        RollMove::Stay => true,
        RollMove::Cut => {
            kick(w, to, 0);
            true
        }
        RollMove::Fade(_) => {
            kick(w, to, 0);
            app.roll.stats.navigate(app.clock.now());
            false
        }
        RollMove::Slide => {
            // Sem a imagem de tela da que sai, a lâmina viva cobre o lugar dela (regra 13).
            let leaving = from.and_then(|i| app.roll.order().get(i).copied());
            let covered = leaving.is_some_and(|id| app.roll.sharp(id));
            ui.set_leaving_index(from.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1));
            w.set_leaving(old.clone());
            w.set_has_leaving(!covered);
            app.images.start_slide();
            app.roll.sliding = true;
            app.roll.stats.navigate(app.clock.now());
            kick(w, to, app.editor.durations().navigate);
            true
        }
    }
}

/// O que está no rolo sob o ponto (`fx`, `fy` em frações da lâmina atual, como a lâmina grande
/// mede), e onde fica a lâmina atual em pixels da área (para levar retângulos de uma à outra).
fn roll_at(app: &App, fx: f64, fy: f64) -> Option<(RollHit, Roll, f64)> {
    let w = app.window.upgrade()?;
    if !roll_shown(&app.editor) {
        return None;
    }
    let roll = RollState::geometry(&w);
    let current = app.session()?.current();
    let pos = f64::from(w.global::<RollUi>().get_pos());
    let left = roll.left(current, pos);
    let point = (left + fx * roll.width, roll.top() + fy * roll.width);
    Some((roll_hit(&roll, pos, point)?, roll, pos))
}

/// O alvo numa vizinha do rolo (spec ML, regras 7 e 11): a lâmina, o que a soltura faz nela e o
/// retângulo do quadro em frações da lâmina atual (o destaque e o pouso da lâmina grande).
pub fn neighbor_target(app: &App, fx: f64, fy: f64) -> Option<(CrossTarget, Option<PxRect>)> {
    let (hit, roll, pos) = roll_at(app, fx, fy)?;
    let RollHit::Spread { index, fx, fy } = hit else {
        return None;
    };
    let session = app.session()?;
    if index == session.current() {
        return None;
    }
    let shift = (roll.left(index, pos) - roll.left(session.current(), pos)) / roll.width;
    match cross_target(session.project(), index, fx, fy)? {
        CrossHit::Frame { frame, rect } => Some((
            CrossTarget::Frame {
                spread: index,
                frame,
            },
            Some(PxRect {
                x: rect.x + shift,
                ..rect
            }),
        )),
        CrossHit::Spread => Some((CrossTarget::Spread(index), None)),
    }
}

/// Onde caem fotos do navegador soltas fora da lâmina atual (`fx`, `fy` em frações dela): no vão
/// de uma lâmina nova ou no "+ Lâmina nova" (regra 20), ou numa vizinha (spec ML, regra 10).
pub fn drop_place(app: &App, fx: f64, fy: f64) -> Option<DropPlace> {
    let (hit, roll, _) = roll_at(app, fx, fy)?;
    match hit {
        RollHit::Gap(n) => Some(DropPlace::NewSpread(n)),
        RollHit::New => Some(DropPlace::NewSpread(roll.count)),
        RollHit::Spread { .. } => match neighbor_target(app, fx, fy)?.0 {
            CrossTarget::Frame { spread, frame } => Some(DropPlace::Frame { spread, frame }),
            CrossTarget::Spread(spread) => Some(DropPlace::Spread(spread)),
        },
    }
}

/// A linha do tempo na tela: só os itens perto do trecho visível, e a largura dela inteira.
pub fn refresh_line(app: &mut App) {
    let all = app.lists.line.model();
    let total = app.roll.line.refresh(&all);
    if let Some(w) = app.window.upgrade() {
        w.global::<RollUi>().set_line_w(total);
    }
}

/// A foto marcada pelo teclado na linha do tempo (spec DD, regra 28). A decisão é do editor.
fn line_step(app: &App, row: i32, delta: i32) -> TimelineStep {
    let none = TimelineStep {
        row: -1,
        x: 0.0,
        spread: -1,
    };
    let Some(session) = app.session() else {
        return none;
    };
    let rows = timeline_rows(session.album());
    let from = usize::try_from(row).ok();
    let Some(i) = timeline_step(&rows, from, delta > 0, session.current()) else {
        return none;
    };
    app.lists
        .line
        .model()
        .row_data(i)
        .map_or(none, |t| TimelineStep {
            row: t.row,
            x: t.x,
            spread: t.spread,
        })
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<RollUi>();
    ui.set_cells(app.borrow().roll.cells.model());
    window.set_timeline(app.borrow().roll.line.model());
    window.set_slots(app.borrow().slot_rows.model());
    let a = app.clone();
    ui.on_line_step(move |row, delta| line_step(&a.borrow(), row, delta));
    let a = app.clone();
    ui.on_line_window(move |start, width| {
        let mut app = a.borrow_mut();
        app.roll.line.set_view(start, width);
        refresh_line(&mut app);
    });
    let a = app.clone();
    ui.on_area_changed(move || area_changed(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_bar_near(move |near| crate::waits::bar_near(&mut a.borrow_mut(), near));
    let a = app.clone();
    ui.on_wheel(move |dx, dy| {
        let mut app = a.borrow_mut();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        let roll = RollState::geometry(&w);
        let (dx, dy) = (f64::from(dx), f64::from(dy));
        let delta = if dx.abs() > dy.abs() { dx } else { dy };
        app.roll
            .scroll
            .wheel(delta * WHEEL_GAIN, roll.step(), roll.count);
        let target = app.roll.scroll.target;
        kick(&w, target, app.editor.durations().scroll);
    });
    let a = app.clone();
    ui.on_edge(move |x| {
        let mut app = a.borrow_mut();
        let Some(w) = app.window.upgrade() else {
            return;
        };
        let roll = RollState::geometry(&w);
        let speed = edge_speed(f64::from(x), roll.area.0);
        if speed != 0.0 {
            app.roll.scroll.nudge(speed * EDGE_TICK, roll.count);
            kick(&w, app.roll.scroll.target, 0);
        }
    });
    let a = app.clone();
    ui.on_pill_clicked(move || {
        let mut app = a.borrow_mut();
        let (Some(w), Some(current)) = (
            app.window.upgrade(),
            app.session().map(lamina_editor::Session::current),
        ) else {
            return;
        };
        app.roll.scroll.follow(current);
        #[allow(clippy::cast_precision_loss)]
        kick(&w, current as f64, app.editor.durations().navigate);
    });
    let a = app.clone();
    ui.on_slide_ended(move || {
        let mut app = a.borrow_mut();
        crate::roll::slide_ended(&mut app);
        if let Some(w) = app.window.upgrade() {
            w.global::<RollUi>().set_leaving_index(-1);
            w.set_has_leaving(false);
        }
        if app.images.slide_ended() {
            crate::crop::refresh_spread(&mut app);
        }
    });
    connect_spreads(&ui, app);
}

/// O clique numa vizinha, a barra e a alça.
fn connect_spreads(ui: &RollUi, app: &Shared) {
    let a = app.clone();
    ui.on_clicked(move |i| {
        if let Ok(i) = usize::try_from(i) {
            a.borrow_mut().timed(
                "Ir para a lâmina",
                |app| app.editor.go_to_spread(i),
                App::refresh_current,
            );
        }
    });
    let a = app.clone();
    ui.on_bar(move |i, name| {
        let Some(command) = Command::from_name(&name) else {
            return;
        };
        // Numa vizinha, ela vira a atual (deslizando) e o comando roda nela (regra 15).
        if let Ok(i) = usize::try_from(i) {
            let mut app = a.borrow_mut();
            if app.session().is_some_and(|s| s.current() != i) && app.editor.go_to_spread(i) {
                app.refresh_current();
            }
        }
        crate::input::run_command(&a, command);
    });
    let a = app.clone();
    // As contas que a tela pede no meio de um desenho: com o estado ocupado, ficam no padrão.
    ui.on_grip_gap(move |x| {
        let Ok(app) = a.try_borrow() else {
            return 0;
        };
        let Some(w) = app.window.upgrade() else {
            return 0;
        };
        let roll = RollState::geometry(&w);
        let pos = f64::from(w.global::<RollUi>().get_pos());
        i32::try_from(roll_gap(&roll, pos, f64::from(x))).unwrap_or(0)
    });
    let a = app.clone();
    ui.on_grip_dropped(move |from, gap| {
        if let (Ok(from), Ok(gap)) = (usize::try_from(from), usize::try_from(gap)) {
            crate::drops::move_spread(&mut a.borrow_mut(), from, gap);
        }
    });
    let a = app.clone();
    ui.on_shift(move |from, gap, i| {
        let count = a.try_borrow().map_or(0, |app| app.editor.album_end());
        let (Ok(from), Ok(gap), Ok(i)) = (
            usize::try_from(from),
            usize::try_from(gap),
            usize::try_from(i),
        ) else {
            return 0;
        };
        reorder(from, gap, count).shift.get(i).copied().unwrap_or(0)
    });
    let a = app.clone();
    ui.on_hit(move |x, y| {
        a.try_borrow()
            .map(|app| hit(&app, f64::from(x), f64::from(y)))
            .unwrap_or_default()
    });
}

/// O que está sob o ponteiro num arrasto de fotos, para o destaque (o mesmo da soltura).
fn hit(app: &App, x: f64, y: f64) -> RollHitData {
    let (Some(w), Some(current)) = (
        app.window.upgrade(),
        app.session().map(lamina_editor::Session::current),
    ) else {
        return RollHitData::default();
    };
    let roll = RollState::geometry(&w);
    let pos = f64::from(w.global::<RollUi>().get_pos());
    let (kind, index) = match roll_hit(&roll, pos, (x, y)) {
        Some(RollHit::Spread { index, .. }) if index == current => (1, index),
        // Numa vizinha, fora dos quadros de foto, ela acende inteira (a foto entra como uma a
        // mais); sobre um quadro, quem acende é o quadro, pela lâmina grande (spec ML, regra 7).
        Some(RollHit::Spread { index, fx, fy }) => match app
            .session()
            .and_then(|s| cross_target(s.project(), index, fx, fy))
        {
            Some(CrossHit::Spread) => (2, index),
            _ => (0, 0),
        },
        Some(RollHit::Gap(n)) => (3, n),
        Some(RollHit::New) => (4, roll.count),
        _ => (0, 0),
    };
    RollHitData {
        kind,
        index: i32::try_from(index).unwrap_or(0),
        ..RollHitData::default()
    }
}
