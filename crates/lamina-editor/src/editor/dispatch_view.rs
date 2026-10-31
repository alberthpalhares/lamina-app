//! O registro de comandos no editor, parte de vista: navegar entre as lâminas, as faixas, o
//! painel, a faixa de layouts, as guias, o movimento e o zoom. Só mudam o que a janela mostra,
//! nunca o álbum (a janela redesenha só a lâmina atual).

use super::Editor;
use crate::{Command, Strip};

impl Editor {
    /// O comando de vista pode rodar agora? `None` = não é de vista.
    pub(super) fn view_available(&self, command: Command) -> Option<bool> {
        let s = self.session.as_ref()?;
        let last = s.album().spreads.len().saturating_sub(1);
        let ok = match command {
            // Com a capa aberta, → e End andam para as lâminas, Home vai para a lâmina 1 (spec CA,
            // regra 10); na lâmina 1, ← vai para a capa.
            // Num álbum sem lâminas, eles voltam ao álbum.
            Command::NextSpread | Command::LastSpread | Command::FirstSpread if s.on_cover() => {
                true
            }
            Command::PreviousSpread if s.on_cover() => false,
            Command::PreviousSpread if s.current() == 0 && s.album().cover.is_some() => true,
            Command::NextSpread | Command::LastSpread => s.current() < last,
            Command::PreviousSpread | Command::FirstSpread => s.current() > 0,
            Command::ToggleGuides | Command::ToggleReduceMotion | Command::TogglePanel => true,
            Command::ToggleGutterShadow => true,
            Command::ToggleSpreads | Command::ToggleTimeline | Command::TogglePhotos => true,
            Command::ToggleStrips => true,
            Command::AdjustPhoto => self.photo_adjust().is_some(),
            Command::EditText => self.selected_text().is_some(),
            // A calibração do tamanho real abre um diálogo da janela, em qualquer modo (ZR).
            Command::CalibrateSize => true,
            // A faixa de layouts precisa de uma lâmina (spec FL, regra 10).
            Command::ToggleLayoutBand => s.spread_count() > 0,
            _ => return None,
        };
        Some(ok)
    }

    /// Roda os comandos de vista; `None` = não é de vista.
    pub(super) fn view_run(&mut self, command: Command) -> Option<bool> {
        let done = match command {
            Command::ToggleGuides => {
                self.guides = !self.guides;
                true
            }
            Command::ToggleSpreads => self.toggle_strip(Strip::Spreads),
            Command::ToggleTimeline => self.toggle_strip(Strip::Timeline),
            Command::TogglePhotos => self.toggle_strip(Strip::Photos),
            Command::ToggleStrips => self.toggle_bottom(),
            Command::TogglePanel => self.toggle_panel(),
            Command::AdjustPhoto => {
                self.wants_details = true;
                self.ask_panel_for_selected();
                true
            }
            Command::EditText => self.show_text(super::TextFocus::Cursor),
            Command::ToggleReduceMotion => self.toggle_reduce_motion(),
            Command::ToggleGutterShadow => self.toggle_gutter_shadow(),
            Command::ToggleLayoutBand => {
                // Os quadros da prévia são outros: abrir desmarca o quadro (spec FL, regra 2).
                self.selected_frame = None;
                self.toggle_band()
            }
            Command::NextSpread
            | Command::PreviousSpread
            | Command::FirstSpread
            | Command::LastSpread => {
                self.selected_frame = None;
                self.navigate(command)
            }
            _ => return None,
        };
        Some(done)
    }

    fn navigate(&mut self, command: Command) -> bool {
        let forward = command != Command::PreviousSpread;
        if forward
            && self.on_cover()
            && self
                .session
                .as_ref()
                .is_some_and(|s| s.album().spreads.is_empty())
        {
            self.leave_cover();
            return true;
        }
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        if command == Command::PreviousSpread && s.current() == 0 && !s.on_cover() {
            return s.go_to_cover();
        }
        let target = match command {
            Command::NextSpread if s.on_cover() => 0,
            Command::NextSpread => s.current() + 1,
            Command::PreviousSpread => s.current().saturating_sub(1),
            Command::FirstSpread => 0,
            Command::LastSpread => s.album().spreads.len().saturating_sub(1),
            _ => return false,
        };
        s.go_to(target)
    }
}
