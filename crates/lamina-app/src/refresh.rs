//! A atualização da barra do projeto, dos comandos, da lâmina atual e dos painéis (saiu do
//! `state.rs`; auditoria do Marco 3, M10).

use lamina_editor::forms::album_info;
use lamina_editor::{BAR_COMMANDS, Command, bar_tip, photos_title, strip_titles};
use slint::SharedString;

use crate::convert::model;
use crate::state::App;

impl App {
    /// Título, contador, comandos disponíveis e a lâmina atual com o painel e a faixa de layouts.
    pub fn refresh_current(&mut self) {
        let Some(w) = self.window.upgrade() else {
            return;
        };
        w.set_has_project(self.session().is_some());
        w.set_can_save(self.editor.available(Command::Save));
        w.set_can_undo(self.editor.available(Command::Undo));
        w.set_can_redo(self.editor.available(Command::Redo));
        w.set_has_missing(self.editor.available(Command::FindMissing));
        let strips = self.editor.strips();
        w.set_spreads_open(strips.spreads);
        w.set_timeline_open(strips.timeline);
        w.set_photos_open(strips.photos);
        // O painel aparece pela escolha guardada ou por um pedido em curso (spec JE, regra 5).
        let panel = self.editor.panel_shown();
        w.set_panel_open(panel);
        crate::panel_width::refresh(self);
        let tips: Vec<SharedString> = BAR_COMMANDS
            .iter()
            .map(|&c| {
                let open = match c {
                    Command::ToggleStrips => strips.spreads || strips.timeline || strips.photos,
                    _ => panel,
                };
                bar_tip(c, open).into()
            })
            .collect();
        w.set_bar_tips(model(tips));
        // O zoom e a altura guardada da grade de fotos (spec PF, regras 4 e 5).
        w.set_photo_zoom(self.editor.photos_zoom() as f32);
        self.show_grid_height();
        w.set_can_prev(self.editor.available(Command::PreviousSpread));
        w.set_can_next(self.editor.available(Command::NextSpread));
        w.set_guides(self.editor.guides());
        w.set_gutter_shadow(self.editor.gutter_shadow_shown());
        w.set_gutter_shadow_on(self.editor.strips().gutter_shadow);
        crate::motion::refresh(&w, self.editor.durations(), self.editor.reduce_motion());
        // O aviso do editor vale também sem álbum: a lista de recentes ilegível aparece na tela
        // inicial (spec TI, regra 9).
        if let Some(notice) = self.editor.take_notice() {
            self.notice = notice;
        }
        w.set_notice(self.notice.clone().into());
        let Some(session) = self.editor.session() else {
            w.set_window_title("Lamina".into());
            w.set_project_title("".into());
            w.set_renaming(false);
            w.set_counter("".into());
            return;
        };
        let dirty = if session.is_dirty() { "• " } else { "" };
        let title = format!("{dirty}{}", session.title());
        w.set_window_title(format!("{title} — Lamina").into());
        w.set_project_title(title.into());
        w.set_album_name(session.title().into());
        w.set_counter(session.counter().into());
        // O álbum em branco: a lâmina vazia no formato e no fundo do álbum (spec BR).
        w.set_empty_album(self.editor.is_blank());
        let project = session.project();
        let canvas = project.spec.canvas_rect();
        let aspect = canvas.height.get() / canvas.width.get().max(1.0);
        w.set_blank_aspect(crate::convert::to_f32(aspect));
        w.set_blank_color(crate::convert::color(project.look.background));
        w.set_can_build_album(self.editor.available(Command::BuildAlbum));
        w.set_counter_warning(session.page_warning().unwrap_or_default().into());
        w.set_page_meter(session.page_meter().map_or_else(Default::default, |m| {
            crate::PageMeterData {
                text: m.text.into(),
                short: m.short.into(),
            }
        }));
        // "Lâminas · 4 lâminas (8 páginas)": o nome fica no cabeçalho, aqui vai só o resumo.
        let selected = self.editor.selection().len();
        let [spreads, timeline, _] = strip_titles(session)
            .map(|t| t.split_once(" · ").map_or(t.clone(), |(_, s)| s.to_owned()));
        w.set_strip_spreads(spreads.into());
        w.set_strip_timeline(timeline.into());
        // O painel de fotos conta a seleção (spec PF, regra 12).
        let photos = photos_title(session, selected);
        let photos = photos
            .split_once(" · ")
            .map_or(photos.clone(), |(_, s)| s.to_owned());
        w.set_strip_photos(photos.into());
        let index = session.current();
        let warnings: Vec<String> = session
            .warnings_for(index)
            .iter()
            .map(ToString::to_string)
            .collect();
        let project = session.project_shared();
        let count = session.spread_count();
        let frames = project.spreads.get(index).map_or(0, |s| s.frames.len());
        w.set_info_album(album_info(session.album()).into());
        let manual = project.spreads.get(index).is_some_and(|s| s.manual);
        let line = if session.on_cover() {
            "Capa".to_owned()
        } else {
            lamina_editor::naming::spread_line(index, count, frames, manual)
        };
        w.set_info_spread(line.into());
        w.set_info_warnings(warnings.join("\n").into());

        crate::cover::refresh(self);
        crate::roll_connect::refresh(self);
        crate::crop::refresh_spread(self);
        if let Some(notice) = self.editor.take_notice() {
            self.notice = notice;
            w.set_notice(self.notice.clone().into());
        }
        crate::band::refresh(self);
        crate::look::refresh(self);
        crate::history::refresh(self);
        crate::frame_panel::refresh(self);
        crate::details::refresh(self);
    }
}
