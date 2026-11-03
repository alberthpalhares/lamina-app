//! Cenários da spec `faixa-de-layouts` (FL-B1 a B7): abrir e fechar, os cartões, a prévia na
//! lâmina pelas vagas da MV, trocar de cartão, aplicar, fechar e as setas com a faixa aberta.

use lamina_core::{PhotoId, Project, Spread};
use lamina_layout::{LayoutCandidate, design_key};

use super::tl_common::{H, V, built, editor_at, spec};
use crate::commands::Mode;
use crate::motion::{Place, Slots};
use crate::suggestions::selection;
use crate::text::option_title;
use crate::view::{Area, FrameView, PxRect, spread_view};
use crate::{Command, Editor, Key, List, Part, Shortcut};

/// Álbum com `spreads` lâminas de 4 fotos (uma vertical a cada quatro).
pub(super) fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

/// Editor na lâmina `index` com a opção `option` em uso e o histórico vazio.
pub(super) fn clean_at(project: Project, index: usize, option: usize) -> Editor {
    let mut first = editor_at(project, index);
    assert!(first.apply_option(Part::Spread, option));
    editor_at(first.session().unwrap().project().clone(), index)
}

/// A faixa aberta (tecla L).
pub(super) fn open(editor: &mut Editor) {
    assert!(editor.run(Command::ToggleLayoutBand));
    assert!(editor.band().open);
}

/// Os quadros da lâmina atual como a janela mostra (com a prévia), em frações da largura.
pub(super) fn shown(editor: &Editor) -> Vec<FrameView> {
    let project = editor.view_project().unwrap();
    let index = editor.session().unwrap().current();
    let canvas = project.spec.canvas_rect();
    let area = Area {
        width: 1.0,
        height: canvas.height.get() / canvas.width.get(),
        margin: 0.0,
    };
    spread_view(&project, index, area, &[]).unwrap().frames
}

pub(super) fn place(editor: &Editor) -> Place {
    let s = editor.session().unwrap();
    (1, s.project().spreads[s.current()].id)
}

fn ids(slots: &Slots) -> Vec<u64> {
    slots.slots().iter().map(|s| s.id).collect()
}

fn rects(slots: &Slots) -> Vec<PxRect> {
    slots.slots().iter().map(|s| s.rect).collect()
}

pub(super) fn steps(editor: &Editor) -> Vec<String> {
    let list = editor.session().unwrap().history_list();
    list.steps.into_iter().map(|s| s.label).collect()
}

/// A lâmina atual como a janela mostra e como está na sessão.
fn view_and_real(editor: &Editor) -> (Spread, Spread) {
    let i = editor.session().unwrap().current();
    let view = editor.view_project().unwrap().spreads[i].clone();
    (view, editor.session().unwrap().project().spreads[i].clone())
}

/// A lâmina atual com a opção `i` aplicada, como o clique deixaria.
fn clicked(editor: &Editor, i: usize) -> Spread {
    let index = editor.session().unwrap().current();
    editor
        .preview(&editor.panel(Part::Spread), i)
        .unwrap()
        .spreads[index]
        .clone()
}

#[test]
fn fl_b1_l_abre_e_fecha_so_no_modo_normal() {
    let mut editor = editor_at(album(3), 1);
    let dirty = editor.session().unwrap().is_dirty();
    editor.select_frame(Some(1));
    let three: Vec<PhotoId> = editor.session().unwrap().project().photos[..3]
        .iter()
        .map(|p| p.id)
        .collect();
    editor.set_selection(three.clone());
    open(&mut editor);
    assert_eq!(editor.selected_frame(), None, "o L desmarca o quadro");
    assert!(editor.run(Command::Deselect));
    assert!(!editor.band().open, "o primeiro Esc fecha a faixa");
    assert_eq!(editor.selection(), three, "e só ela");
    assert!(editor.run(Command::Deselect));
    assert!(editor.selection().is_empty(), "o segundo desmarca");
    open(&mut editor);
    assert!(editor.run(Command::ToggleLayoutBand));
    assert!(!editor.band().open, "L de novo fecha");
    open(&mut editor);
    assert!(editor.run(Command::Organize));
    assert!(
        !editor.band().open,
        "entrar no modo organizar fecha a faixa"
    );
    assert!(!editor.available(Command::ToggleLayoutBand));
    assert!(!editor.run(Command::ToggleLayoutBand));
    let l = Shortcut::key(Key::Char('l'));
    assert_eq!(Command::from_shortcut_in(l, Mode::Organize), None);
    assert_eq!(editor.session().unwrap().is_dirty(), dirty);
    assert!(steps(&editor).is_empty(), "nada no histórico");
    // Sem lâminas ou com um trabalho longo, L fica indisponível.
    let mut empty = Editor::new();
    empty.adopt(Project::new("vazio", spec(30.0, 30.0)));
    assert!(!empty.available(Command::ToggleLayoutBand) && !empty.toggle_band());
    let mut busy = editor_at(album(2), 0);
    busy.set_busy(true);
    assert!(!busy.available(Command::ToggleLayoutBand));
}

#[test]
fn fl_b2_as_mesmas_opcoes_do_painel_de_layouts() {
    let mut editor = clean_at(built(spec(30.0, 30.0), &[H, H, V], 1), 0, 0);
    let panel = editor.panel(Part::Spread);
    let aspects = panel.selection.clone().unwrap().aspects;
    let key = |c: &LayoutCandidate| design_key(c, &aspects);
    let (used, favorite, rejected) = (
        key(&panel.options[0]),
        key(&panel.options[6]),
        key(&panel.options[1]),
    );
    assert!(editor.mark_option(6, true));
    let at = editor
        .panel(Part::Spread)
        .options
        .iter()
        .position(|c| key(c) == rejected);
    assert!(editor.mark_option(at.unwrap(), false));
    open(&mut editor);
    let panel = editor.panel(Part::Spread);
    let cards = panel.cards();
    assert_eq!(cards.len(), panel.options.len());
    for (i, (card, c)) in cards.iter().zip(&panel.options).enumerate() {
        assert_eq!(
            card.title,
            format!("{}. {}", i + 1, option_title(&c.family))
        );
        assert_eq!(card.favorite, panel.favorite[i]);
    }
    assert_eq!(key(&panel.options[0]), favorite, "o favorito em 1º");
    assert!(cards[0].favorite, "com a estrela");
    assert!(
        panel.options.iter().all(|c| key(c) != rejected),
        "o rejeitado fora"
    );
    assert_eq!(
        key(&panel.options[panel.in_use.unwrap()]),
        used,
        "o em uso marcado"
    );
    let in_order: Vec<String> = cards.into_iter().map(|c| c.title).collect();
    assert!(editor.run(Command::ToggleReordered));
    assert!(editor.band().open, "R deixa a faixa aberta");
    let panel = editor.panel(Part::Spread);
    assert_eq!(panel.list, List::Reordered);
    let titles: Vec<String> = panel.cards().into_iter().map(|c| c.title).collect();
    assert_eq!(titles.len(), panel.options.len());
    assert_ne!(titles, in_order, "os cartões passam a ser a outra lista");
}

#[test]
fn fl_b3_a_previa_nao_grava_nada() {
    let mut editor = clean_at(album(3), 1, 0);
    let before = editor.session().unwrap().project().clone();
    let dirty = editor.session().unwrap().is_dirty();
    open(&mut editor);
    let mut slots = Slots::default();
    slots.update(&shown(&editor), place(&editor), false);
    let start = ids(&slots);
    assert!(editor.hover_option(Some(4)));
    assert_eq!(editor.band().hover, Some(4));
    let view = editor.view_project().unwrap().into_owned();
    assert_eq!(
        view.spreads[1],
        clicked(&editor, 4),
        "os quadros da opção 5"
    );
    assert_ne!(view.spreads[1], before.spreads[1]);
    assert_eq!(
        (&view.spreads[0], &view.spreads[2]),
        (&before.spreads[0], &before.spreads[2])
    );
    let s = editor.session().unwrap();
    assert_eq!(s.project().spreads, before.spreads, "a sessão não muda");
    assert!(s.is_dirty() == dirty && steps(&editor).is_empty());
    let frames = shown(&editor);
    assert!(!slots.update(&frames, place(&editor), false));
    assert_eq!(ids(&slots), start, "as mesmas vagas, nenhuma nasce");
    for slot in slots.slots() {
        assert!(slot.live && !slot.born);
        assert_eq!(
            slot.rect,
            frames[slot.frame.unwrap()].rect,
            "o retângulo novo"
        );
    }
    assert!(
        slots.slots().iter().any(|s| s.fly),
        "marcadas para deslizar"
    );
}

#[test]
fn fl_b4_trocar_de_cartao_e_sair_da_faixa() {
    let mut editor = clean_at(album(3), 1, 0);
    open(&mut editor);
    let mut slots = Slots::default();
    slots.update(&shown(&editor), place(&editor), false);
    let (start, real) = (ids(&slots), rects(&slots));
    assert!(editor.hover_option(Some(4)));
    slots.update(&shown(&editor), place(&editor), false);
    assert!(editor.hover_option(Some(2)));
    assert_eq!(view_and_real(&editor).0, clicked(&editor, 2));
    slots.update(&shown(&editor), place(&editor), false);
    assert_eq!(ids(&slots), start);
    assert!(
        slots.slots().iter().any(|s| s.fly),
        "seguem deslizando para a 3"
    );
    assert!(
        !editor.hover_option(Some(2)),
        "o mesmo cartão não muda nada"
    );
    assert!(editor.hover_option(None), "saiu da faixa");
    let (view, now) = view_and_real(&editor);
    assert_eq!(view, now, "volta à lâmina real");
    slots.update(&shown(&editor), place(&editor), false);
    assert_eq!(
        (ids(&slots), rects(&slots)),
        (start, real),
        "aos retângulos de antes"
    );
    assert!(slots.slots().iter().any(|s| s.fly));
    assert!(steps(&editor).is_empty());
}

#[test]
fn fl_b5_o_clique_aplica_sem_segundo_deslize() {
    let mut editor = clean_at(album(3), 1, 0);
    let before = editor.session().unwrap().project().spreads[1].clone();
    open(&mut editor);
    let mut slots = Slots::default();
    slots.update(&shown(&editor), place(&editor), false);
    assert!(editor.hover_option(Some(4)));
    let preview = view_and_real(&editor).0;
    slots.update(&shown(&editor), place(&editor), false);
    assert!(editor.apply_option(Part::Spread, 4));
    assert!(steps(&editor)[0].starts_with("Trocar layout"));
    let (view, now) = view_and_real(&editor);
    assert_eq!(now, preview, "igual à prévia, quadro a quadro");
    assert_eq!(view, now, "a prévia foi limpa");
    slots.update(&shown(&editor), place(&editor), false);
    assert!(slots.slots().iter().all(|s| !s.fly), "nada desliza de novo");
    assert_eq!(editor.panel(Part::Spread).in_use, Some(4));
    assert_eq!(editor.band().hover, None);
    assert!(editor.band().open, "a faixa continua aberta");
    assert!(editor.run(Command::Undo));
    assert!(!editor.band().open, "o Ctrl+Z fecha a faixa");
    assert_eq!(editor.session().unwrap().project().spreads[1], before);
}

#[test]
fn fl_b6_fechar_desfaz_a_previa() {
    let mut editor = clean_at(album(3), 1, 0);
    let dirty = editor.session().unwrap().is_dirty();
    open(&mut editor);
    assert!(editor.hover_option(Some(4)));
    assert!(editor.close_band(), "clique fora");
    let (view, now) = view_and_real(&editor);
    assert_eq!(view, now, "a lâmina volta");
    open(&mut editor);
    assert!(editor.hover_option(Some(3)));
    assert!(editor.run(Command::Deselect), "Esc");
    assert_eq!(editor.band(), crate::editor::Band::default());
    let (view, now) = view_and_real(&editor);
    assert_eq!(view, now);
    let s = editor.session().unwrap();
    assert!(
        s.is_dirty() == dirty && steps(&editor).is_empty(),
        "nem histórico nem arquivo"
    );
    assert!(!editor.close_band(), "já fechada");
}

#[test]
fn fl_b7_setas_com_a_faixa_aberta() {
    let mut editor = clean_at(album(3), 1, 1);
    open(&mut editor);
    assert!(editor.hover_option(Some(5)));
    assert!(editor.run(Command::NextLayout));
    assert_eq!(editor.band().hover, None, "a prévia é limpa antes");
    assert_eq!(
        editor.panel(Part::Spread).in_use,
        Some(2),
        "a 3, a partir da 2"
    );
    assert!(editor.run(Command::NextLayout));
    assert_eq!(editor.panel(Part::Spread).in_use, Some(3));
    assert!(editor.band().open);
    // ← e → mudam de lâmina com a faixa aberta, e a faixa mostra as opções da lâmina nova.
    assert!(editor.hover_option(Some(5)));
    assert!(editor.run(Command::NextSpread));
    assert!(editor.band().open, "a faixa continua aberta");
    assert_eq!(editor.band().hover, None, "a prévia é limpa");
    assert_eq!(editor.session().unwrap().current(), 2);
    let panel = editor.panel(Part::Spread);
    let sel = selection(editor.session().unwrap().project(), 2, Part::Spread).unwrap();
    assert_eq!(
        panel.selection.unwrap().photos,
        sel.photos,
        "as fotos da lâmina 3"
    );
    assert!(editor.run(Command::PreviousSpread));
    assert!(editor.band().open);
    assert_eq!(editor.session().unwrap().current(), 1);
    assert_eq!(editor.panel(Part::Spread).in_use, Some(3), "a troca ficou");
}
