//! Fotos nos quadros pela janela (spec FQ): soltar fotos e quadros na lâmina, mover lâminas na
//! faixa e cortes na timeline. As decisões são do editor. O movimento do arrasto fica em
//! `drag_motion.rs`, e o clique no navegador em `browser.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::motion::TimelineKey;
use lamina_editor::motion::{CrossHit, cross_target};
use lamina_editor::{CrossTarget, DropPlace};

use crate::state::App;
use crate::{AppWindow, TimelineItem};

type Shared = Rc<RefCell<App>>;

/// O quadro de foto da lâmina atual sob o ponto solto (frações da largura da lâmina). Quadro de
/// texto nunca recebe foto: sobre um título, a foto entra como uma a mais (spec ML, regra 13).
fn target(app: &App, fx: f32, fy: f32) -> Option<usize> {
    let s = app.session()?;
    match cross_target(s.project(), s.current(), f64::from(fx), f64::from(fy))? {
        CrossHit::Frame { frame, .. } => Some(frame),
        CrossHit::Spread => None,
    }
}

/// O ponto solto cai dentro da lâmina grande (frações da largura; a altura é a proporção)?
fn inside(app: &App, fx: f32, fy: f32) -> bool {
    let Some(s) = app.session() else {
        return false;
    };
    if s.spread_count() == 0 {
        return false;
    }
    let aspect = crate::convert::aspect(s.project());
    (0.0..=1.0).contains(&fx) && (0.0..=aspect).contains(&fy)
}

/// Onde cai a foto de um quadro solta em (`fx`, `fy`, frações da lâmina atual): um quadro da
/// lâmina atual, uma vizinha do rolo ou uma miniatura da faixa (spec ML, regras 7 e 11).
fn frame_target(app: &App, fx: f32, fy: f32) -> Option<CrossTarget> {
    let current = app.session()?.current();
    let on_cover = app.editor.on_cover();
    if let Some(frame) = target(app, fx, fy) {
        return Some(CrossTarget::Frame {
            spread: current,
            frame,
        });
    }
    // Da capa, o quadro não vai para as lâminas: a projeção só tem a capa.
    if on_cover {
        return None;
    }
    let roll = crate::roll_connect::neighbor_target(app, f64::from(fx), f64::from(fy));
    roll.map(|(t, _)| t).or_else(|| strip_target(app))
}

/// A miniatura da faixa de lâminas sob o ponteiro, com a fração dentro dela (spec ML, ML-B9).
fn strip_target(app: &App) -> Option<CrossTarget> {
    use slint::ComponentHandle as _;
    let w = app.window.upgrade()?;
    let ui = w.global::<crate::RollUi>();
    let spread = usize::try_from(ui.get_strip_spread()).ok()?;
    let (fx, fy) = (f64::from(ui.get_strip_fx()), f64::from(ui.get_strip_fy()));
    let project = app.session()?.project();
    Some(match cross_target(project, spread, fx, fy)? {
        CrossHit::Frame { frame, .. } => CrossTarget::Frame { spread, frame },
        CrossHit::Spread => CrossTarget::Spread(spread),
    })
}

fn after_edit(app: &mut App, changed: bool) {
    if changed {
        app.refresh_all();
    }
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_drop_photo(move |i, fx, fy| {
        let mut app = a.borrow_mut();
        let Some(photo) = app.photo_at(i) else {
            return;
        };
        // Sobre um quadro, como sempre (FQ); na lâmina, fora dos quadros, as fotos entram nela; na
        // lâmina vazia do álbum em branco, viram a primeira (IM, regras 7 e 8; BR, regra 4).
        let current = app.session().map_or(0, lamina_editor::Session::current);
        let place = match (target(&app, fx, fy), inside(&app, fx, fy)) {
            (Some(frame), _) => DropPlace::Frame {
                spread: current,
                frame,
            },
            (None, true) => DropPlace::Spread(current),
            (None, false) if app.session().is_some_and(|s| s.spread_count() == 0) => {
                DropPlace::Spread(0)
            }
            // Fora da lâmina atual, no rolo: um vão ou o "+ Lâmina nova" (spec RL, regra 20).
            (None, false) => {
                match crate::roll_connect::drop_place(&app, f64::from(fx), f64::from(fy)) {
                    Some(place) => place,
                    None => return,
                }
            }
        };
        let photos = match place {
            DropPlace::Frame { .. } => vec![photo],
            _ => app.editor.drag_photos(photo),
        };
        let done = app.timed(
            "Soltar fotos",
            |app| app.editor.drop_photos(place, &photos),
            App::refresh_all,
        );
        // Na capa aberta, fora dos quadros, a foto vai para a área sob o ponteiro (spec CA, regra 19).
        if app.editor.on_cover() && target(&app, fx, fy).is_none() {
            let photos = app.editor.drag_photos(photo);
            if app.editor.drop_on_cover(f64::from(fx), &photos) {
                app.refresh_all();
            }
            return;
        }
        // Mais de 24 fotos na lâmina vazia: nada muda, e o aviso aparece (spec BR, regra 4).
        if !done {
            app.refresh_current();
        }
    });
    let a = app.clone();
    window.on_drop_photos_gap(move |i, gap| {
        let mut app = a.borrow_mut();
        let (Some(photo), Ok(gap)) = (app.photo_at(i), usize::try_from(gap)) else {
            return;
        };
        let photos = app.editor.drag_photos(photo);
        app.timed(
            "Lâmina nova com fotos",
            |app| app.editor.drop_photos(DropPlace::NewSpread(gap), &photos),
            App::refresh_all,
        );
    });
    let a = app.clone();
    window.on_drop_frame(move |from, fx, fy| {
        let mut app = a.borrow_mut();
        let (Ok(from), Some(target)) = (usize::try_from(from), frame_target(&app, fx, fy)) else {
            return;
        };
        // Na mesma lâmina, troca como sempre (FQ); noutra, enche, troca ou acrescenta (spec ML).
        app.timed(
            "Mover foto",
            |app| app.editor.move_photo(from, target),
            App::refresh_all,
        );
    });
    let a = app.clone();
    window.on_frame_selected(move |i| {
        a.borrow_mut().timed(
            "Selecionar quadro",
            |app| {
                // Fora dos quadros desmarca (no modo organizar, todos).
                app.grid_focus = false;
                match usize::try_from(i) {
                    Ok(frame) => app.editor.select_frame(Some(frame)),
                    Err(_) => app.editor.click_nothing(),
                }
                true
            },
            App::refresh_current,
        );
    });
    let a = app.clone();
    window.on_move_spread(move |from, gap| {
        if let (Ok(from), Ok(gap)) = (usize::try_from(from), usize::try_from(gap)) {
            move_spread(&mut a.borrow_mut(), from, gap);
        }
    });
    let a = app.clone();
    window.on_cut_moved(move |k, delta| {
        let mut app = a.borrow_mut();
        let changed = usize::try_from(k).is_ok_and(|k| app.editor.drag_cut(k, delta));
        after_edit(&mut app, changed);
    });
    connect_sequence(window, app);
}

/// A foto arrastada na Sequência (spec SQ): a linha de onde ela entra, pelo editor, e o soltar.
/// Uma recusa mostra o aviso dela.
fn connect_sequence(window: &AppWindow, app: &Shared) {
    let a = app.clone();
    window.on_seq_marker(move |x| {
        // Uma conta da tela no meio de outra edição: a linha fica fora de vista.
        let at = a
            .try_borrow()
            .ok()
            .and_then(|app| app.editor.sequence_target(f64::from(x)));
        // Pixels da faixa cabem com folga em f32.
        #[allow(clippy::cast_possible_truncation)]
        at.map_or(-100.0, |(_, line)| line as f32)
    });
    let a = app.clone();
    window.on_seq_drop(move |row, x| {
        let mut app = a.borrow_mut();
        let changed =
            usize::try_from(row).is_ok_and(|row| app.editor.sequence_drop(row, f64::from(x)));
        if changed {
            app.refresh_all();
        } else {
            app.refresh_current();
        }
    });
}

/// A lâmina `from` solta no vão `gap` (da faixa ou da alça do rolo). A decisão é do editor.
pub fn move_spread(app: &mut App, from: usize, gap: usize) {
    let moved = app.editor.move_spread_to_gap(from, gap);
    after_edit(app, moved);
}

/// A timeline: as fotos de cada lâmina e um corte entre uma lâmina e a outra (regra 8), só as
/// posições que mudaram (spec MV, regra 3).
pub fn refresh_timeline(app: &mut App) {
    let Some((project, rows)) = app.editor.sequence_rows() else {
        return;
    };
    let thumbs = app.thumbs(&project);
    let mut items: Vec<TimelineItem> = rows
        .iter()
        .enumerate()
        .map(|(i, (key, spread))| {
            let spread = i32::try_from(*spread).unwrap_or(0);
            let row = i32::try_from(i).unwrap_or(-1);
            match key {
                TimelineKey::Photo(id, _) => TimelineItem {
                    img: thumbs.get(id).cloned().unwrap_or_default(),
                    cut: false,
                    spread,
                    x: 0.0,
                    row,
                },
                TimelineKey::Cut(_) => TimelineItem {
                    img: slint::Image::default(),
                    cut: true,
                    spread,
                    x: 0.0,
                    row,
                },
            }
        })
        .collect();
    crate::line_window::place(&mut items);
    app.lists
        .line
        .sync(with_position(rows), |i| items[i].clone());
    crate::roll_connect::refresh_line(app);
}

/// As linhas da Sequência com a posição de cada uma junto da lâmina: um item que só andou (a foto
/// levada para outra lâmina empurra os do meio) também é regravado, com o `x` novo (spec SQ).
fn with_position<K>(rows: Vec<(K, usize)>) -> Vec<(K, (usize, usize))> {
    rows.into_iter()
        .enumerate()
        .map(|(i, (key, spread))| (key, (spread, i)))
        .collect()
}

#[cfg(test)]
mod tests {
    use slint::Model;

    use super::*;
    use crate::lists::Synced;

    /// A linha da tela para estas chaves (um algarismo é o corte depois daquela lâmina), com as
    /// posições já calculadas.
    fn line(keys: &str) -> (Vec<(char, usize)>, Vec<TimelineItem>) {
        let mut spread = 0;
        let rows: Vec<(char, usize)> = keys
            .chars()
            .map(|k| {
                let row = (k, spread);
                if k.is_ascii_digit() {
                    spread += 1;
                }
                row
            })
            .collect();
        let mut items: Vec<TimelineItem> = rows
            .iter()
            .map(|(k, s)| TimelineItem {
                cut: k.is_ascii_digit(),
                spread: i32::try_from(*s).unwrap_or(0),
                ..TimelineItem::default()
            })
            .collect();
        crate::line_window::place(&mut items);
        (rows, items)
    }

    #[test]
    fn foto_levada_para_outra_lamina_regrava_quem_andou() {
        let mut list: Synced<char, (usize, usize), TimelineItem> = Synced::new();
        let (rows, items) = line("ab0cd1e");
        list.sync(with_position(rows), |i| items[i].clone());
        // "b" vai para o meio da lâmina 2: o corte e o "c" andam uma vaga para a esquerda.
        let (rows, items) = line("a0cbd1e");
        list.sync(with_position(rows), |i| items[i].clone());
        let model = list.model();
        let got: Vec<f32> = (0..model.row_count())
            .filter_map(|i| model.row_data(i))
            .map(|t| t.x)
            .collect();
        let want: Vec<f32> = items.iter().map(|t| t.x).collect();
        assert_eq!(got, want, "cada item no x novo");
    }
}
