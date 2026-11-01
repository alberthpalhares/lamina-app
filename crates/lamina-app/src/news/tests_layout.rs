//! O desenho da página com a janela de verdade, sem tela: o botão Fechar inteiro em qualquer tamanho e
//! as capturas para conferir.

use lamina_editor::updates::Version;
use slint::platform::WindowEvent;
use slint::{Model, Rgb8Pixel};

use super::tests::{WIDE, animations, open_notes, open_window, settle};
use super::*;
use crate::Theme;
use crate::headless::Screen;

/// O fio do botão Fechar: duas linhas de contorno inteiras no canto de cima à direita da janela. A
/// beta 8 cortava o fundo dele; se o botão sai da janela ou é cortado, falta uma das linhas.
fn close_outline_rows(screen: &Screen, pixels: &[Rgb8Pixel]) -> Vec<usize> {
    let color = screen.app.global::<Theme>().get_line_strong().to_argb_u8();
    let (width, height) = (screen.width() as usize, screen.height() as usize);
    let look_from = width - 140;
    (0..height.min(120))
        .filter(|y| {
            let row = &pixels[y * width + look_from..(y + 1) * width];
            let same = row
                .iter()
                .filter(|p| (p.r, p.g, p.b) == (color.red, color.green, color.blue))
                .count();
            same >= 50
        })
        .collect()
}

#[test]
fn o_botao_fechar_aparece_inteiro_em_qualquer_tamanho() {
    for (size, scale, dark) in [
        (WIDE, 1.0, true),
        (WIDE, 1.3, true),
        (WIDE, 1.0, false),
        ((1920, 980), 1.0, true),
        ((800, 600), 1.3, true),
    ] {
        let screen = open_window(size);
        let theme = screen.app.global::<Theme>();
        theme.set_text_scale(scale);
        theme.set_dark(dark);
        let image = animations()[0];
        let notes = format!(
            "## [0.1.0-beta.9] - 2026-11-12\n\n- **Com botão.** Um. Experimente: abra.\n  <!-- imagem: {image}; comando: ajuda.atalhos -->\n"
        );
        open_notes(&screen, &notes, false);
        settle(&screen);
        let rows = close_outline_rows(&screen, &screen.draw());
        let (first, last) = (rows.first().copied(), rows.last().copied());
        assert!(rows.len() >= 2, "{size:?} {scale}: {rows:?}");
        let tall = last.unwrap() - first.unwrap();
        assert!((24..=44).contains(&tall), "{size:?} {scale}: {rows:?}");
    }
}

fn press(screen: &Screen, key: slint::platform::Key) {
    let text = slint::SharedString::from(key);
    screen
        .window
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    screen
        .window
        .dispatch_event(WindowEvent::KeyReleased { text });
    settle(screen);
}

#[test]
fn o_teclado_rola_a_lista_e_esc_fecha_a_pagina() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    open_notes(&screen, &super::tests::notes_with(25), false);
    settle(&screen);
    let before = screen.draw();
    press(&screen, slint::platform::Key::PageDown);
    assert_ne!(screen.draw(), before, "PageDown rola: o foco está na lista");
    press(&screen, slint::platform::Key::Home);
    assert_eq!(screen.draw(), before, "Home volta ao começo");
    press(&screen, slint::platform::Key::Escape);
    assert!(!ui.get_show());
    assert_eq!(ui.get_entries().row_count(), 0);
}

/// Com `LAMINA_CAPTURAS=<pasta>`, grava a página em PNG para conferir o desenho.
#[test]
fn capturas_para_conferir() {
    let Ok(folder) = std::env::var("LAMINA_CAPTURAS") else {
        return;
    };
    let shots = [
        ("1366-escuro", WIDE, 1.0, true),
        ("1366-claro", WIDE, 1.0, false),
        ("1366-130", WIDE, 1.3, true),
        ("1920-escuro", (1920, 980), 1.0, true),
        ("800-130", (800, 600), 1.3, true),
    ];
    for (name, size, scale, dark) in shots {
        let screen = open_window(size);
        let theme = screen.app.global::<Theme>();
        theme.set_text_scale(scale);
        theme.set_dark(dark);
        let image = animations()[0];
        let notes = format!(
            "## [0.1.0-beta.9] - 2026-11-12\n\nEsta versão traz a página de novidades.\n\n\
             - **Página de novidades.** Tudo numa página, com filtros e busca. Experimente: digite algo na busca.\n  <!-- imagem: {image}; comando: lamina.organizar; tipo: novo; fixada: sim -->\n\
             - **Vídeo da janela.** A janela nova filmada de verdade. Experimente: assista ao vídeo.\n  <!-- imagem: {image}; video: https://alberthpalhares.github.io/lamina-app/v/janela-nova.html; tipo: melhoria; fixada: sim; so: mac; comando: ajuda.atalhos -->\n\
             - **Sem capa.** Uma novidade sem ilustração nenhuma, com texto para quebrar a linha em telas pequenas. Experimente: leia.\n\n\
             ## [0.1.0-beta.8] - 2026-10-31\n\nA versão anterior.\n\n\
             - **Antiga.** Sem etiqueta. <!-- imagem: {image} -->\n"
        );
        let current = Version::parse("0.1.0-beta.9").unwrap();
        let page = from_help(&parse(&notes), &current);
        let tries = vec![
            TryNow::Unavailable("Abra um álbum para experimentar.".to_owned()),
            TryNow::Ready,
            TryNow::Hidden,
            TryNow::Hidden,
        ];
        open_page(
            &screen.app,
            Opening {
                help: page.clone(),
                page,
                help_tries: tries.clone(),
                tries,
                reduce_motion: false,
            },
        );
        settle(&screen);
        let pixels = screen.draw();
        let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
        let out = image::RgbImage::from_raw(screen.width(), screen.height(), bytes).unwrap();
        out.save(format!("{folder}/{name}.png")).unwrap();
    }
}
