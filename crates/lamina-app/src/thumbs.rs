//! Miniaturas que chegaram do trabalhador (spec TR, regra 6): só as linhas delas mudam no
//! navegador, na linha do tempo e na faixa de lâminas, e o espaço da capa, se alguma é dela; a
//! lâmina grande e a faixa de layouts se refazem se alguma é da lâmina atual.

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
    let cover = project
        .cover
        .as_ref()
        .is_some_and(|c| has_any(&c.content, &fresh));
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
    if cover {
        crate::cover::thumb_arrived(app);
    }
    if current {
        // Os cartões da faixa e a lâmina grande mostram a miniatura até a foto nítida chegar.
        app.lists.panel = None;
        crate::band::refresh(app);
        app.images.arrived = true;
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use lamina_core::{AlbumSpec, CoverMeasures, Dpi, Mm, PhotoId, Project};
    use lamina_editor::{Cache, Command, Editor, prepare_images};
    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    use slint::platform::{Platform, WindowAdapter};
    use slint::{ComponentHandle, Model};

    use crate::mailbox::Mailbox;
    use crate::state::App;
    use crate::{AppWindow, CoverUi};

    struct Headless(Rc<MinimalSoftwareWindow>);

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    /// Um álbum de uma lâmina (com `a`) e uma capa só com `b`, foto que nenhuma lâmina usa.
    fn album_with_own_cover_photo(folder: &std::path::Path) -> (Editor, PhotoId) {
        let cm = Mm::from_cm(30.0);
        let spec = AlbumSpec::new(cm, cm, Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("capa", spec);
        let mut photo = |name: &str| {
            let file = folder.join(name);
            image::RgbImage::new(300, 200).save(&file).unwrap();
            let id = project.add_photo(file, 300, 200);
            // A miniatura mora no cache pela impressão digital, como numa foto importada.
            project.photos.last_mut().unwrap().fingerprint = Some(name.to_owned());
            id
        };
        let (a, b) = (photo("a.jpg"), photo("b.jpg"));
        let mut editor = Editor::new();
        editor.adopt(project);
        editor.set_selection(vec![a]);
        assert!(editor.run(Command::BuildFromSelection));
        let measures = CoverMeasures::defaults(&editor.session().unwrap().album().spec);
        assert!(editor.create_cover(measures, None, &[b]));
        assert!(editor.go_to_spread(0));
        editor.set_selection(Vec::new());
        (editor, b)
    }

    fn tile_has_photo(window: &AppWindow) -> bool {
        let tile = window.global::<CoverUi>().get_tile();
        tile.frames.iter().any(|f| f.has_img)
    }

    /// Roda na própria linha: o Slint guarda a plataforma por linha, e cada teste precisa da sua.
    #[test]
    fn ca_b35_a_miniatura_da_capa_mostra_a_foto_que_nenhuma_lamina_usa() {
        std::thread::spawn(|| {
            let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
            slint::platform::set_platform(Box::new(Headless(window))).unwrap();
            let dir = tempfile::tempdir().unwrap();
            let (editor, cover_photo) = album_with_own_cover_photo(dir.path());
            let album = editor.session().unwrap().album_shared();
            assert!(
                !album.spreads[0].frames.iter().any(|f| matches!(
                    f.content,
                    lamina_core::FrameContent::Photo { photo, .. } if photo == cover_photo
                )),
                "a foto da capa não pode estar na lâmina"
            );
            let cache = Cache::new(dir.path().join("cache"));
            assert_eq!(prepare_images(&album, 0, &cache), 0);
            let w = AppWindow::new().unwrap();
            let (tx, rx) = Mailbox::new();
            let mut app = App::new(&w, cache, editor, tx);

            app.refresh_strip();
            while app.images.thumbs_pending() > 0 {
                let msg = rx.recv_timeout(Duration::from_secs(30)).unwrap();
                crate::new_album::message(&mut app, msg);
                crate::images::after_arrivals(&mut app);
            }

            assert!(tile_has_photo(&w), "a capa da faixa ficou cinza");
        })
        .join()
        .unwrap();
    }
}
