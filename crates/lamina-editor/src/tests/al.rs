//! Cenários AL-B5 e AL-B10 da spec `specs/aprendizado-de-layouts.md`: as listas da janela. A ordem
//! em si está no `lamina-layout`.

use lamina_core::RelRect;
use lamina_layout::{LayoutCandidate, LayoutOptions, Learned, design_key, layout_options};

use super::tl_common::{H, V, built, editor_at, frames, spec};
use crate::{Command, DropPlace, Part};

/// Um peso que leva à 1ª posição um desenho a menos de 7 pontos da 1ª opção.
fn rival(list: &[LayoutCandidate], aspects: &[f64]) -> Learned {
    let top = &list[0];
    let key = design_key(top, aspects);
    let near = list
        .iter()
        .find(|c| design_key(c, aspects) != key && top.score.total - c.score.total < 7.0)
        .expect("um desenho perto da 1ª opção");
    Learned::from_weights([(design_key(near, aspects), u32::MAX)])
}

#[test]
fn al_b5_favorito_e_rejeitado_mandam_mais() {
    let mut editor = editor_at(built(spec(30.0, 30.0), &[H, H, V], 1), 0);
    let panel = editor.panel(Part::Spread);
    let aspects = panel.selection.unwrap().aspects;
    let key = |c: &LayoutCandidate| design_key(c, &aspects);
    let (taught, favorite, rejected, used) = (
        key(&panel.options[1]),
        key(&panel.options[2]),
        key(&panel.options[3]),
        key(&panel.options[4]),
    );
    editor.set_learned(Learned::from_weights(
        [&taught, &rejected, &used].map(|k| (k.clone(), 200_000)),
    ));
    let at = |editor: &crate::Editor, wanted: &str| {
        let options = editor.panel(Part::Spread).options;
        options.iter().position(|c| key(c) == wanted).unwrap()
    };
    let favorite_at = at(&editor, &favorite);
    assert!(editor.mark_option(favorite_at, true));
    let rejected_at = at(&editor, &rejected);
    assert!(editor.mark_option(rejected_at, false));

    let panel = editor.panel(Part::Spread);
    assert_eq!(key(&panel.options[0]), favorite, "o favorito vem primeiro");
    assert!(panel.options.iter().all(|c| key(c) != rejected));
    assert_eq!(
        editor.learned().weight(&rejected),
        0,
        "rejeitar apaga o peso"
    );
    assert_eq!(editor.learned().weight(&taught), 200_000);

    // Rejeitar o layout em uso também apaga.
    let used_at = at(&editor, &used);
    assert!(editor.apply_option(Part::Spread, used_at));
    assert!(editor.run(Command::Reject));
    assert_eq!(editor.learned().weight(&used), 0);
}

/// Os retângulos em ordem, para comparar sem olhar qual foto está em qual.
fn sorted(rects: impl IntoIterator<Item = RelRect>) -> Vec<[u64; 4]> {
    let mut out: Vec<[u64; 4]> = rects
        .into_iter()
        .map(|r| [r.x(), r.y(), r.width(), r.height()].map(|v| (v * 1e6).round().to_bits()))
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn al_b10_a_remontagem_segue_a_regra_e_o_enter_segue_a_faixa() {
    // Quatro fotos na lâmina e uma quinta para soltar.
    let mut project = built(spec(30.0, 30.0), &[H; 4], 1);
    project.add_photo("f04.jpg", H.0, H.1);
    let extra = project.photos[4].id;
    let all: Vec<_> = project.photos.iter().map(|p| p.id).collect();
    let five = [1.5; 5];
    let plain = |aspects: &[f64]| {
        let options = LayoutOptions::default();
        layout_options(
            &project.spec,
            lamina_core::ActivePages {
                left: true,
                right: true,
            },
            aspects,
            &options,
        )
        .unwrap()
        .in_order
    };
    let list = plain(&five);
    let learned = rival(&list, &five);
    let mut editor = editor_at(project.clone(), 0);
    editor.set_learned(learned);

    // Soltar a foto refaz a lâmina com a 1ª opção da regra.
    assert!(editor.drop_photos(DropPlace::Spread(0), &[extra]));
    let got = sorted(frames(&editor).into_iter().map(|(_, r)| r));
    assert_eq!(got, sorted(list[0].frames.iter().copied()));

    // A faixa já mostra o aprendido na frente, e o Enter usa o que ela mostra.
    editor.set_selection(all);
    let shown = editor.panel(Part::Spread).options;
    assert_ne!(
        sorted(shown[0].frames.iter().copied()),
        sorted(list[0].frames.iter().copied()),
        "o peso devia mudar a 1ª opção da faixa"
    );
    assert!(editor.run(Command::BuildFromSelection));
    let got = sorted(frames(&editor).into_iter().map(|(_, r)| r));
    assert_eq!(got, sorted(shown[0].frames.iter().copied()));
}
