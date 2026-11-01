//! Os comandos do modo organizar no editor (spec EG): quando valem e o que fazem. O Esc fica em
//! camadas: fecha o recorte, depois desmarca, depois sai do modo.

use super::Editor;
use crate::Command;

/// O deslocamento, em mm, das setas do modo (1 mm; com Shift, 10 mm).
pub(super) fn nudge_of(command: Command) -> Option<(f64, f64)> {
    let d = match command {
        Command::NudgeLeft => (-1.0, 0.0),
        Command::NudgeRight => (1.0, 0.0),
        Command::NudgeUp => (0.0, -1.0),
        Command::NudgeDown => (0.0, 1.0),
        Command::NudgeLeftMore => (-10.0, 0.0),
        Command::NudgeRightMore => (10.0, 0.0),
        Command::NudgeUpMore => (0.0, -10.0),
        Command::NudgeDownMore => (0.0, 10.0),
        _ => return None,
    };
    Some(d)
}

impl Editor {
    /// Disponibilidade dos comandos do modo (e do Esc dentro dele); `None` = não é deles.
    pub(super) fn organize_available(&self, command: Command) -> Option<bool> {
        let on = self.organizing();
        let ok = match command {
            Command::Organize => true,
            Command::SelectAllFrames => on,
            Command::ViewZoomIn | Command::ViewZoomOut | Command::ViewFit => on,
            Command::Loupe | Command::ActualSize => on,
            Command::Deselect if on => true,
            c if nudge_of(c).is_some() => on && !self.selected_frames().is_empty(),
            _ => return None,
        };
        Some(ok)
    }

    /// Roda os comandos do modo; `None` = não é deles. `had_crop`: havia recorte aberto antes do
    /// comando (o Esc só fecha o recorte).
    pub(super) fn organize_run(&mut self, command: Command, had_crop: bool) -> Option<bool> {
        let on = self.organizing();
        let done = match command {
            Command::Organize => self.set_organizing(!on),
            Command::SelectAllFrames => self.select_all_frames(),
            Command::ViewZoomIn => self.zoom_step(true, None, true).is_some(),
            Command::ViewZoomOut => self.zoom_step(false, None, true).is_some(),
            Command::ViewFit => self.zoom_fit().is_some(),
            Command::ActualSize => self.zoom_real().is_some(),
            Command::Loupe => self.toggle_loupe(),
            Command::Deselect if on => {
                had_crop || self.clear_frames() || self.set_organizing(false)
            }
            c => {
                let (dx, dy) = nudge_of(c)?;
                self.nudge_group(dx, dy)
            }
        };
        Some(done)
    }
}

impl Editor {
    /// Clique fora dos quadros: desmarca (no modo, todos os quadros).
    pub fn click_nothing(&mut self) {
        if self.organizing() {
            self.clear_frames();
        } else {
            self.select_frame(None);
        }
    }

    /// O título do painel do modo: "Organizar a lâmina 3" ou "Organizar a capa" (vazio fora do modo;
    /// spec DG, regra 7).
    pub fn organize_title(&self) -> String {
        match (&self.session, self.organizing()) {
            (Some(s), true) if s.on_cover() => "Organizar a capa".to_owned(),
            (Some(s), true) => format!("Organizar a lâmina {}", s.current() + 1),
            _ => String::new(),
        }
    }
}
