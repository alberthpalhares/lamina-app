//! Velocidade da MV (MV-B24; AGENTS.md, regra 13): o trabalho do editor e da tela numa troca de
//! layout e numa navegação, sem nenhuma foto de tela pronta (nada lido do disco).

use std::time::{Duration, Instant};

use lamina_core::Project;
use lamina_core::testing::{TRIES, budget};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::Command;
use crate::motion::{Slots, row_ops, strip_rows};
use crate::screens::ScreenPlan;
use crate::view::{Area, FrameView, spread_view};

/// 60 lâminas com 8 fotos cada (480 fotos: a montagem automática põe no máximo 8 por lâmina; as 600 da
/// spec pediriam 75 lâminas).
fn album() -> Project {
    let photos: Vec<(u32, u32)> = (0..480).map(|i| if i % 4 == 3 { V } else { H }).collect();
    built(spec(30.0, 30.0), &photos, 60)
}

fn frames(project: &Project, index: usize) -> Vec<FrameView> {
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    spread_view(project, index, area, &[]).map_or_else(Vec::new, |v| v.frames)
}

#[test]
fn mv_b24_trocar_layout_e_navegar_rapido() {
    let mut editor = editor_at(album(), 30);
    let mut slots = Slots::default();
    let mut plan = ScreenPlan::default();
    let (mut edit, mut screen, mut navigate) = (Duration::MAX, Duration::MAX, Duration::MAX);
    for _ in 0..TRIES {
        let rows = strip_rows(editor.session().unwrap());
        let s = editor.session().unwrap();
        let place = (1, s.project().spreads[s.current()].id);
        slots.update(&frames(s.project(), s.current()), place, false);

        // Trocar o layout: o editor e o trabalho da tela (vagas, faixa, plano das fotos).
        let started = Instant::now();
        assert!(editor.run(Command::NextLayout));
        let ran = started.elapsed();
        let s = editor.session().unwrap();
        slots.update(&frames(s.project(), s.current()), place, false);
        let ops = row_ops(&rows, &strip_rows(s));
        plan.go(s.project(), s.current());
        let total = started.elapsed();
        assert_eq!(ops.len(), 1);
        edit = edit.min(total);
        screen = screen.min(total.saturating_sub(ran));

        // Navegar: o editor e o plano das fotos, sem ler o disco.
        let started = Instant::now();
        assert!(editor.run(Command::NextSpread));
        let s = editor.session().unwrap();
        let place = (1, s.project().spreads[s.current()].id);
        slots.update(&frames(s.project(), s.current()), place, false);
        plan.go(s.project(), s.current());
        navigate = navigate.min(started.elapsed());
        assert!(editor.run(Command::PreviousSpread));
        if edit < budget(50) && screen < budget(10) && navigate < budget(10) {
            break;
        }
    }
    assert!(edit < budget(50), "trocar o layout levou {edit:?}");
    assert!(screen < budget(10), "o trabalho da tela levou {screen:?}");
    assert!(navigate < budget(10), "navegar levou {navigate:?}");
}
