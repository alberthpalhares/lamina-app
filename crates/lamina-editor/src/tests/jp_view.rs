//! A visão da lâmina é a da impressão (JP-B2).

use lamina_core::FrameContent;
use lamina_render::{Scale, bleed, pixels, region};

use super::common::Base;
use crate::{FrameKind, spread_view, view::Area};

#[test]
fn jp_b2_a_visao_da_lamina_e_a_da_impressao() {
    let base = Base::with_photos(12, (600, 400));
    let project = base.project(3);
    let area = Area {
        width: 1200.0,
        height: 700.0,
        margin: 24.0,
    };
    let view = spread_view(&project, 0, area, &[]).unwrap();
    let canvas = project.spec.canvas_rect();

    // Maior escala que cabe, sem distorção, centralizada.
    let expected = ((1200.0 - 48.0) / canvas.width.get()).min((700.0 - 48.0) / canvas.height.get());
    assert!((view.scale - expected).abs() < 1e-9);
    let c = view.canvas;
    assert!((c.x * 2.0 + c.width - 1200.0).abs() < 1e-6, "{c:?}");
    assert!((c.y * 2.0 + c.height - 700.0).abs() < 1e-6, "{c:?}");

    // Cada quadro onde sai na impressão (até 1 px) e com a mesma parte da foto.
    let spec = &project.spec;
    let print_scale = f64::from(spec.dpi().get()) / lamina_core::MM_PER_INCH;
    let size = spec.canvas_size_px();
    let ratio = view.scale / print_scale;
    let spread = &project.spreads[0];
    assert_eq!(view.frames.len(), spread.frames.len());
    for (frame, shown) in spread.frames.iter().zip(&view.frames) {
        let rect = project.frame_rect(frame);
        let bled = bleed(spec, rect);
        let scale = Scale {
            x: print_scale,
            y: print_scale,
        };
        let printed = pixels(bled.rect, scale, size).unwrap();
        let r = shown.rect;
        let close = |a: f64, b: u32| (a - f64::from(b) * ratio).abs() <= 1.0;
        assert!(
            close(r.x - c.x, printed.x0) && close(r.y - c.y, printed.y0),
            "{r:?}"
        );
        assert!(
            close(r.width, printed.width()) && close(r.height, printed.height()),
            "{r:?}"
        );

        let FrameContent::Photo { photo, crop } = &frame.content else {
            panic!("a montagem automática só põe fotos");
        };
        let p = project.photo(*photo).unwrap();
        let (pw, ph) = (f64::from(p.width_px), f64::from(p.height_px));
        let used = region(crop, rect, &bled, (pw, ph));
        let FrameKind::Photo { source, .. } = &shown.kind else {
            panic!("quadro com foto");
        };
        assert!((source.x * pw - used.x).abs() < 1e-6);
        assert!((source.y * ph - used.y).abs() < 1e-6);
        assert!((source.width * pw - used.width).abs() < 1e-6);
        assert!((source.height * ph - used.height).abs() < 1e-6);
    }
}

#[test]
fn guias_marcam_corte_lombada_e_zona_segura() {
    let base = Base::with_photos(4, (600, 400));
    let project = base.project(1);
    let area = Area {
        width: 1000.0,
        height: 600.0,
        margin: 0.0,
    };
    let view = spread_view(&project, 0, area, &[]).unwrap();
    let t = view.trim;
    assert!((view.spine_x - (t.x + t.width / 2.0)).abs() < 1e-9);
    assert!(view.safe[0].x > t.x && view.safe[1].x > view.spine_x);
    assert!(view.inactive.is_empty());
    assert!(
        spread_view(&project, 1, area, &[]).is_none(),
        "lâmina que não existe"
    );
}
