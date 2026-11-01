//! Movimento e fluidez (spec MV, ADR 0011): o editor planeja, a tela anima. Aqui ficam as
//! contas sem janela: a medição de tempos, as vagas, as listas, o arrastar e o rolo de lâminas
//! (spec RL, ADR 0012).

pub mod drag;
pub mod durations;
pub mod navigation;
pub mod quiet;
pub mod roll;
pub mod roll_bar;
pub mod roll_nav;
mod roll_plan;
mod roll_revisions;
pub mod roll_timeline;
pub mod roll_view;
pub mod rows;
pub mod slots;
pub mod strip_drag;
pub mod timings;

pub use drag::{
    CrossHit, GHOST_PX, Ghost, Reorder, cross_target, drop_target, frame_ghost, photo_ghost,
    reorder,
};
pub use durations::Durations;
pub use navigation::{Navigator, RAPID, Transition};
pub use quiet::{BAR_GRACE_MS, DOUBLE_CLICK_MS, HeaderClicks, HoverGrace, Quiet, RESIZE_QUIET_MS};
pub use roll::{
    Landing, ROLL_BYTES, Revisions, RollAsk, RollGot, RollPlan, RollView, SCREEN_MAX, SMALL_WIDTH,
    Tier,
};
pub use roll_bar::{BarButton, RollPill, SPREAD_BAR_COMMANDS, SpreadBar, roll_shown, spread_bar};
pub use roll_nav::{NEAR, Pill, RollMove, RollScroll, pill, roll_move};
pub use roll_timeline::{TIMELINE, TimelineSizes, spread_badge, timeline_run, timeline_scroll};
pub use roll_view::{
    EDGE_PX, EDGE_SPEED, NARROW, ROLL_GAP, Roll, RollHit, RollWidth, edge_speed, roll_gap,
    roll_hit, roll_width, textured, visible_range,
};
pub use rows::{
    PanelKey, PhotoRow, RowOp, StripRow, TimelineKey, panel_key, photo_rows, row_ops, strip_rows,
    timeline_rows,
};
pub use slots::{Place, Slot, SlotContent, Slots};
pub use strip_drag::{
    STRIP_SPACING, strip_drop, strip_gap, strip_landing, strip_lefts, strip_shift,
};
pub use timings::{BURST_GAP, TIMINGS_VAR, Timings};
