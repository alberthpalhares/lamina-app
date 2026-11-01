//! Cenários das specs PV e NV com a janela de verdade, sem tela, em 1366×668: a lista só cria as
//! linhas no layout, então cada teste desenha antes de conferir.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::updates::Version;
use lamina_editor::updates::news::{animates, cover};
use slint::platform::WindowEvent;
use slint::{LogicalPosition, Model};

use super::*;
use crate::Motion;
use crate::headless::{self, Screen};

pub(super) const WIDE: (u32, u32) = (1366, 668);

pub(super) fn open_window(size: (u32, u32)) -> Screen {
    let screen = headless::open(size.0, size.1).unwrap();
    connect(&screen.app);
    screen.app.global::<Motion>().set_on(false);
    screen
}

/// As ilustrações animadas que vão dentro do programa.
pub(super) fn animations() -> Vec<&'static str> {
    embedded::IMAGES
        .iter()
        .filter(|(_, bytes)| animates(bytes))
        .map(|(name, _)| *name)
        .collect()
}

pub(super) fn notes_with(count: usize) -> String {
    let names = animations();
    let items: String = (0..count)
        .map(|i| {
            let image = names[i % names.len()];
            format!(
                "- **Novidade {i}.** Um texto curto para a linha ter altura. Experimente: abra.\n  <!-- imagem: {image} -->\n"
            )
        })
        .collect();
    format!("## [0.1.0-beta.9] - 2026-11-12\n\nAbertura.\n\n{items}")
}

pub(super) fn open_notes(screen: &Screen, notes: &str, reduce_motion: bool) {
    let current = Version::parse("0.1.0-beta.9").unwrap();
    let page = from_help(&parse(notes), &current);
    let count = entries(&page).len();
    open_page(
        &screen.app,
        Opening {
            help: page.clone(),
            page,
            tries: vec![TryNow::Hidden; count],
            help_tries: vec![TryNow::Hidden; count],
            reduce_motion,
        },
    );
}

/// Desenha algumas vezes: as linhas nascem no desenho, e as capas chegam no temporizador.
pub(super) fn settle(screen: &Screen) {
    for _ in 0..30 {
        screen.draw();
    }
}

fn wheel(screen: &Screen, dy: f32) {
    let position = LogicalPosition::new(600.0, 400.0);
    screen
        .window
        .dispatch_event(WindowEvent::PointerMoved { position });
    screen.window.dispatch_event(WindowEvent::PointerScrolled {
        position,
        delta_x: 0.0,
        delta_y: dy,
    });
    screen.draw();
}

fn loaded() -> (usize, usize) {
    SHOWN.with_borrow(|s| (s.covers.count(), s.covers.bytes()))
}

#[test]
fn pv_b12_abrir_a_pagina_nao_decodifica_tudo() {
    let screen = open_window(WIDE);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (w, log) = (screen.app.as_weak(), seen.clone());
    screen
        .app
        .global::<NewsUi>()
        .on_cover_requested(move |index| {
            log.borrow_mut().push(index);
            if let Some(w) = w.upgrade() {
                covers::request(&w, usize::try_from(index).unwrap());
            }
        });
    open_notes(&screen, &notes_with(25), false);
    settle(&screen);

    let asked = seen.borrow().clone();
    assert!(!asked.is_empty(), "as linhas da tela pedem a capa");
    assert!(asked.len() <= 10, "{} pedidos", asked.len());
    let first_rows: Vec<i32> = (0..i32::try_from(asked.len()).unwrap()).collect();
    assert_eq!(asked, first_rows, "só as linhas da tela");
    let (count, bytes) = loaded();
    assert_eq!(count, asked.len());
    assert!(bytes < 10_000_000, "{bytes} bytes");

    for _ in 0..80 {
        wheel(&screen, -300.0);
    }
    settle(&screen);
    assert!(
        seen.borrow().iter().any(|i| *i >= 20),
        "rolou até o fim: {:?}",
        seen.borrow()
    );
    assert!(loaded().0 <= covers::MAX_COVERS, "{} capas", loaded().0);
}

/// Na abertura de verdade, depois de atualizar, a página abre antes de `connect` (`main.rs`).
#[test]
fn a_pagina_que_abre_antes_de_ligar_a_janela_mostra_a_lista() {
    let screen = headless::open(WIDE.0, WIDE.1).unwrap();
    screen.app.global::<Motion>().set_on(false);
    open_notes(&screen, &notes_with(3), false);
    connect(&screen.app);
    settle(&screen);
    let ui = screen.app.global::<NewsUi>();
    assert_eq!(ui.get_entries().row_count(), 3);
    assert!(loaded().0 > 0, "as linhas da tela pediram a capa");
}

#[test]
fn pv_b13_uma_animacao_por_vez_e_nada_toca_sozinho() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    open_notes(&screen, &notes_with(2), false);
    settle(&screen);
    assert_eq!(news_player::playing(), None);
    assert_eq!(ui.get_playing(), -1);

    ui.invoke_cover_clicked(0);
    assert_eq!(news_player::playing(), Some(0));
    assert_eq!(ui.get_playing(), 0);
    ui.invoke_cover_clicked(1);
    assert_eq!(news_player::playing(), Some(1));
    assert_eq!(ui.get_playing(), 1);
    ui.invoke_cover_clicked(1);
    assert_eq!(news_player::playing(), None);
    assert_eq!(ui.get_playing(), -1);
}

#[test]
fn com_reduzir_movimento_a_capa_nao_anima_e_o_video_continua() {
    let screen = open_window(WIDE);
    let image = animations()[0];
    let notes = format!(
        "## [0.1.0-beta.9] - 2026-11-12\n\n\
         - **Animação.** Um. <!-- imagem: {image} -->\n\
         - **Vídeo.** Dois. <!-- imagem: {image}; video: https://alberthpalhares.github.io/lamina-app/v/a.html -->\n"
    );
    open_notes(&screen, &notes, true);
    let rows = screen.app.global::<NewsUi>().get_entries();
    assert!(!rows.row_data(0).unwrap().animated);
    assert!(!rows.row_data(1).unwrap().animated);
    assert_ne!(rows.row_data(1).unwrap().video, "");

    open_notes(&screen, &notes, false);
    let rows = screen.app.global::<NewsUi>().get_entries();
    assert!(rows.row_data(0).unwrap().animated);
    assert!(!rows.row_data(1).unwrap().animated, "a capa abre o vídeo");
}

#[test]
fn pv_b18_rolar_a_lista_para_a_animacao() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    open_notes(&screen, &notes_with(25), false);
    settle(&screen);
    ui.invoke_cover_clicked(0);
    assert_eq!(news_player::playing(), Some(0));
    wheel(&screen, -120.0);
    assert_eq!(news_player::playing(), None);
    assert_eq!(ui.get_playing(), -1);
}

#[test]
fn janela_redimensionada_com_animacao_tocando_continua_tocando() {
    let mut screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    open_notes(&screen, &notes_with(25), false);
    settle(&screen);
    wheel(&screen, -600.0);
    settle(&screen);
    let row = ui.get_entries().row_data(8).unwrap();
    ui.invoke_cover_clicked(row.index);
    screen.resize(1000, 640);
    settle(&screen);
    assert_eq!(news_player::playing(), Some(8));
}

fn kinds_and_photos() -> String {
    let image = animations()[0];
    format!(
        "## [0.1.0-beta.9] - 2026-11-12\n\n\
         - **Novo.** Uma foto. <!-- imagem: {image}; tipo: novo -->\n\
         - **Melhor.** Outra foto. <!-- imagem: {image}; tipo: melhoria -->\n\
         - **Só texto.** Nada.\n"
    )
}

#[test]
fn pv_b19_fechar_limpa_a_busca_o_filtro_e_as_capas() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    open_notes(&screen, &kinds_and_photos(), false);
    settle(&screen);
    assert!(loaded().0 > 0, "as capas chegaram");
    assert_eq!(ui.get_filters().row_count(), 3);
    ui.invoke_filter_chosen(2);
    ui.set_query("foto".into());
    ui.invoke_search("foto".into());
    settle(&screen);
    assert_eq!(ui.get_entries().row_count(), 1);
    assert_eq!(ui.get_filter(), 2);

    ui.invoke_close();
    assert!(!ui.get_show());
    assert_eq!(ui.get_entries().row_count(), 0);
    assert_eq!(loaded().0, 0);

    open_notes(&screen, &kinds_and_photos(), false);
    settle(&screen);
    assert_eq!(ui.get_filter(), 0);
    assert_eq!(ui.get_query(), "");
    assert_eq!(ui.get_entries().row_count(), 3);
}

#[test]
fn nv_b17_fechar_solta_as_capas_da_memoria() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    open_notes(&screen, &notes_with(2), false);
    settle(&screen);
    ui.invoke_cover_clicked(1);
    assert_eq!(news_player::playing(), Some(1));

    ui.invoke_close();

    assert!(!ui.get_show());
    assert_eq!(ui.get_entries().row_count(), 0);
    assert_eq!(news_player::playing(), None);
    assert_eq!(ui.get_playing(), -1);
    assert_eq!(loaded(), (0, 0));
}

#[test]
fn as_notas_embutidas_leem_e_toda_ilustracao_citada_esta_embutida() {
    let all = sections();
    assert!(!all.is_empty());
    assert!(all.iter().all(|s| !s.items.is_empty()));
    let page = from_help(&all, &Version::parse("9.9.9").unwrap());
    for entry in entries(&page) {
        if let Some(name) = &entry.item.image {
            let bytes = rows::bytes_of(&entry);
            assert!(bytes.and_then(cover).is_some(), "{name} não está embutida");
        }
    }
}

#[test]
fn destaque_rola_a_lista_ate_o_item() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    let image = animations()[0];
    let mut notes = notes_with(25);
    notes.push_str(&format!(
        "- **Fixada.** Um destaque. <!-- imagem: {image}; fixada: sim -->\n"
    ));
    open_notes(&screen, &notes, false);
    settle(&screen);
    assert_eq!(ui.get_highlights().row_count(), 1);
    assert!(ui.get_show_highlights());
    ui.set_jump(25);
    for _ in 0..60 {
        screen.draw();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(ui.get_arrived(), 25);
}

#[test]
fn ver_todas_as_versoes_troca_a_lista_e_a_busca_continua_valendo() {
    let screen = open_window(WIDE);
    let ui = screen.app.global::<NewsUi>();
    let notes = "## [0.1.0-beta.9] - 2026-11-12\n\n- **Foto nova.** Um.\n\n\
                 ## [0.1.0-beta.8] - 2026-10-31\n\n- **Foto velha.** Dois.\n- **Outra.** Três.\n";
    let sections = parse(notes);
    let current = Version::parse("0.1.0-beta.9").unwrap();
    let fresh = after_update(
        &sections,
        &Version::parse("0.1.0-beta.8").unwrap(),
        &current,
    )
    .unwrap();
    let help = from_help(&sections, &current);
    open_page(
        &screen.app,
        Opening {
            tries: vec![TryNow::Hidden; entries(&fresh).len()],
            help_tries: vec![TryNow::Hidden; entries(&help).len()],
            page: fresh,
            help,
            reduce_motion: false,
        },
    );
    assert!(ui.get_more());
    assert_eq!(ui.get_title(), "Lamina atualizado para 0.1.0-beta.9");
    assert_eq!(ui.get_entries().row_count(), 1);
    ui.set_query("foto".into());
    ui.invoke_search("foto".into());
    ui.invoke_all_versions();
    assert!(!ui.get_more());
    assert_eq!(ui.get_subtitle(), "");
    assert_eq!(ui.get_title(), "O que há de novo no Lamina");
    assert_eq!(ui.get_entries().row_count(), 2, "a busca continua valendo");
}
