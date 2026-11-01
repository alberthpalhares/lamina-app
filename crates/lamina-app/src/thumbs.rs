//! Miniaturas que chegaram do trabalhador (spec TR, regra 6): só as linhas delas mudam no
//! navegador, na linha do tempo e na faixa de lâminas; a lâmina grande e a faixa de layouts se
//! refazem se alguma é da lâmina atual.

use std::collections::HashSet;

use lamina_core::{FrameContent, PhotoId, Spread};
use lamina_editor::motion::TimelineKey;

use crate::state::{App, strip_data};

/// A lâmina tem alguma das fotos `fresh`?
fn has_any(spread: &Spread, fresh: &HashSet<PhotoId>) -> bool {
    spread.frames.iter().any(|f| match &f.content {
        FrameContent::Photo { photo, .. } => fresh.contains(photo),
        _ => false,
    })
}

/// Põe nas listas as miniaturas `fresh`, que acabaram de chegar.
pub fn show(app: &mut App, fresh: &[PhotoId]) {
    let fresh: HashSet<PhotoId> = fresh.iter().copied().collect();
    let Some(session) = app.editor.session() else {
        return;
    };
    // Sem copiar o projeto: com a grade rolando, chegam miniaturas a cada quadro (spec PF).
    let (project, missing) = (session.album(), session.missing());
    let current = project
        .spreads
        .get(session.current())
        .is_some_and(|s| has_any(s, &fresh));
    let images = &app.images;
    let thumb = |id: &PhotoId| images.thumb(*id).unwrap_or_default();
    // Na grade, só as peças perto da tela mostram miniatura (spec PF, regra 14).
    let window = app.grid_view.window.clone();
    app.lists.browser.patch(
        |i, (id, _)| window.contains(&i) && fresh.contains(id),
        |_, (id, _), row| row.img = thumb(id),
    );
    app.lists.line.patch(
        |_, (key, _)| matches!(key, TimelineKey::Photo(id, _) if fresh.contains(id)),
        |_, (key, _), row| {
            if let TimelineKey::Photo(id, _) = key {
                row.img = thumb(id);
            }
        },
    );
    let all = images.thumbs_map();
    app.lists.strip.patch(
        |_, (_, row)| has_any(&row.spread, &fresh),
        |i, (_, row), data| *data = strip_data(project, i, missing, all, row.warned),
    );
    crate::roll_connect::refresh_line(app);
    if current {
        // Os cartões da faixa e a lâmina grande mostram a miniatura até a foto nítida chegar.
        app.lists.panel = None;
        crate::band::refresh(app);
        app.images.arrived = true;
    }
}
