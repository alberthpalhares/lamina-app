//! O movimento na janela (spec MV): copia para o `global Motion` as durações que o editor decide
//! ("Reduzir movimento" zera todas; regra 14).

use lamina_editor::motion::Durations;
use slint::ComponentHandle;

use crate::{AppWindow, Motion};

/// As durações e o interruptor do movimento.
pub fn refresh(w: &AppWindow, durations: Durations, reduce: bool) {
    let ms = |v: u32| i32::try_from(v).unwrap_or(0);
    let m = w.global::<Motion>();
    m.set_on(!reduce);
    m.set_slide_ms(ms(durations.slide));
    m.set_fade_ms(ms(durations.fade));
    m.set_navigate_ms(ms(durations.navigate));
    m.set_scroll_ms(ms(durations.scroll));
    m.set_land_ms(ms(durations.land));
    m.set_sharp_ms(ms(durations.sharp));
    m.set_reorder_ms(ms(durations.reorder));
    m.set_band_ms(ms(durations.band));
}
