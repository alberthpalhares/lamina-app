//! O registro de comandos no editor: quando cada comando está disponível e o que ele faz. Os de
//! vista (navegar, faixas, painel, guias e zoom) ficam em `dispatch_view.rs`.

use super::{Editor, adjust, crop, organize_cmd};
use crate::suggestions::Part;
use crate::{Command, Session};

impl Editor {
    /// O comando pode rodar agora? (menu e teclado obedecem; regra 10 do JP)
    pub fn available(&self, command: Command) -> bool {
        // Durante uma tarefa longa, só o Sobre e a ajuda de sugestões e registros (spec SR,
        // regra 2): é quando algo trava que o testador quer relatar.
        if self.busy {
            return matches!(
                command,
                Command::About | Command::SendFeedback | Command::OpenLogs
            );
        }
        // Com a capa aberta, os comandos só de lâmina ficam desativados (spec CA, regra 5).
        if self.on_cover() && super::cover::SPREAD_ONLY.contains(&command) {
            return false;
        }
        let Some(s) = &self.session else {
            return matches!(
                command,
                Command::NewAlbum
                    | Command::Open
                    | Command::OpenExample
                    | Command::ToggleReduceMotion
                    | Command::About
                    | Command::Presets
                    | Command::Models
                    | Command::CheckUpdates
                    | Command::ToggleAutoUpdates
                    | Command::ToggleBeta
                    | Command::SendFeedback
                    | Command::OpenLogs
                    | Command::ShowShortcuts
                    | Command::ShowNews
                    | Command::Quit
            ) || (command == Command::ClearRecents && self.recents.has_unpinned());
        };
        // Atalhos por modo (spec EG, regra 2) e os comandos do modo organizar.
        if !command.in_mode(self.mode()) {
            return false;
        }
        if let Some(ok) = self
            .organize_available(command)
            .or_else(|| self.arrange_available(command))
            .or_else(|| self.view_available(command))
        {
            return ok;
        }
        let last = s.spread_count().saturating_sub(1);
        match command {
            Command::NewAlbum | Command::Open | Command::Save | Command::SaveAs => true,
            Command::OpenExample | Command::CloseAlbum | Command::RenameAlbum => true,
            Command::Quit => true,
            Command::ClearRecents => self.recents.has_unpinned(),
            Command::Export | Command::Specification | Command::Presets => true,
            Command::MountAlbum | Command::ImportPhotos | Command::ImportFolder => true,
            Command::FindMissing => !s.missing().is_empty(),
            Command::LocatePhoto => self
                .selected_frame()
                .and_then(|f| self.missing_photo_at(f))
                .is_some(),
            Command::About | Command::ToggleReordered => true,
            Command::Undo => s.can_undo(),
            Command::Redo => s.can_redo(),
            Command::NextLayout | Command::PreviousLayout => self.can_step(),
            Command::Mirror => self.can_mirror(),
            Command::Favorite | Command::Reject => self.panel(Part::Spread).in_use.is_some(),
            Command::ClearFrame if self.organizing() => !self.selected_frames().is_empty(),
            Command::ClearFrame => self.selected_content().is_some(),
            Command::RemoveFromAlbum => !self.selected_frame_photos().is_empty(),
            Command::NewText | Command::NewPhotoFrame | Command::Models => true,
            Command::CheckUpdates | Command::ToggleAutoUpdates | Command::ToggleBeta => true,
            Command::SendFeedback | Command::OpenLogs => true,
            // A lista de atalhos vale em qualquer momento (spec AT, regra 7).
            Command::ShowShortcuts => true,
            // As novidades valem em qualquer momento (spec NV, regra 5).
            Command::ShowNews => true,
            Command::SaveModel => self.can_save_model(),
            Command::BringForward | Command::SendBackward => self.can_reorder(),
            Command::Deselect => {
                self.band.open
                    || self.selected_frame.is_some()
                    || !self.selection.is_empty()
                    || self.crop_frame.is_some()
            }
            // No álbum em branco, Enter com mais de 24 fotos responde com o aviso (spec BR).
            Command::BuildFromSelection if self.is_blank() => !self.selection.is_empty(),
            Command::BuildFromSelection | Command::NewSpreadFromSelection => {
                !self.selection.is_empty() && self.photo_selection().is_ok()
            }
            Command::NewSpread | Command::NewBlankSpread | Command::NewBlankSpreadBefore => true,
            Command::BlankPage => self.can_toggle_blank(),
            Command::CreateCover => !self.has_cover(),
            Command::GoToCover => self.has_cover() && !self.on_cover(),
            Command::CoverMeasures | Command::DeleteCover => self.has_cover(),
            Command::ToggleCoverGuide => {
                self.on_cover() && self.cover().is_some_and(|c| c.guide.is_some())
            }
            Command::DeleteSpread => s.spread_count() > 1,
            Command::ClearAllPhotos => self.clear_counts().0 > 0,
            Command::DeleteAllSpreads => s.spread_count() > 0,
            Command::MoveSpreadLeft => s.current() > 0,
            Command::MoveSpreadRight => s.current() < last,
            Command::CropMode => self.can_clear(),
            Command::ZoomIn | Command::ZoomOut | Command::ResetCrop => self.crop_frame.is_some(),
            Command::CopyFrames | Command::DuplicateFrames => !self.selected_frames().is_empty(),
            Command::PasteFrames => self.can_paste(),
            Command::TurnLeft
            | Command::TurnRight
            | Command::TurnLeftMore
            | Command::TurnRightMore => self.crop_frame.is_some(),
            // "Montar o álbum…" abre uma janela (spec BR, regra 12): pode com foto não usada.
            Command::BuildAlbum => self.can_build_album(),
            Command::SelectAllPhotos => !self.shown_photos().is_empty(),
            // Os do modo organizar e os de vista já responderam acima.
            Command::Organize
            | Command::SelectAllFrames
            | Command::NudgeLeft
            | Command::NudgeRight
            | Command::NudgeUp
            | Command::NudgeDown
            | Command::NudgeLeftMore
            | Command::NudgeRightMore
            | Command::NudgeUpMore
            | Command::NudgeDownMore
            | Command::ViewZoomIn
            | Command::ViewZoomOut
            | Command::ViewFit
            | Command::AlignLeft
            | Command::AlignCenter
            | Command::AlignRight
            | Command::AlignTop
            | Command::AlignMiddle
            | Command::AlignBottom
            | Command::EqualWidth
            | Command::EqualHeight
            | Command::EqualSize
            | Command::DistributeAcross
            | Command::DistributeDown
            | Command::NextSpread
            | Command::PreviousSpread
            | Command::FirstSpread
            | Command::LastSpread
            | Command::ToggleGuides
            | Command::ToggleSpreads
            | Command::ToggleStrips
            | Command::ToggleTimeline
            | Command::TogglePhotos
            | Command::TogglePanel
            | Command::ToggleGutterShadow
            | Command::ToggleReduceMotion
            | Command::AdjustPhoto
            | Command::EditText
            | Command::ToggleLayoutBand
            | Command::Loupe
            | Command::ActualSize
            | Command::CalibrateSize => false,
        }
    }

    /// Roda os comandos que não precisam de diálogo. `true` = algo mudou. Os de arquivo pedem um
    /// caminho à janela e chamam [`Editor::open`] e a sessão.
    pub fn run(&mut self, command: Command) -> bool {
        // A lista de atalhos e as novidades são abertas pela janela e não mexem em nada: nem o
        // recorte, nem o modo organizar, nem a seleção (specs AT, regra 7, e NV, regra 5).
        // Renomear abre o campo da janela, que chama `rename_album` (spec NA).
        if !self.available(command)
            || matches!(
                command,
                Command::ShowShortcuts | Command::ShowNews | Command::RenameAlbum
            )
        {
            return false;
        }
        // A faixa de layouts (spec FL, regras 2 e 8): o Esc só a fecha; antes dos outros comandos
        // ela fecha, ou só limpa a prévia nas teclas do menu Layout.
        if command == Command::Deselect && self.close_band() {
            return true;
        }
        self.close_band_for(command);
        // Um comando no meio do arrasto de um quadro descarta o gesto: soltar depois gravaria o
        // quadro na lâmina errada (auditoria do Marco 3, M2). E encerra a emenda do histórico:
        // digitar depois de um comando é uma edição nova (spec TX, regra 10).
        self.gesture = None;
        self.group = None;
        // Setas seguidas no mesmo grupo viram uma edição só (spec EG, regra 10).
        let nudging =
            organize_cmd::nudge_of(command).is_some() || adjust::turn_of(command).is_some();
        if let Some(s) = self.session.as_mut()
            && !nudging
        {
            s.end_merge();
        }
        let had_crop = self.crop_frame.is_some();
        // Qualquer outro comando fecha o modo recorte, gravando o gesto (spec RC, regra 1).
        let crop_command = matches!(
            command,
            Command::CropMode | Command::ZoomIn | Command::ZoomOut | Command::ResetCrop
        ) || adjust::turn_of(command).is_some();
        if !crop_command {
            self.exit_crop();
        }
        if let Some(done) = self
            .organize_run(command, had_crop)
            .or_else(|| self.arrange_run(command))
            .or_else(|| self.view_run(command))
        {
            return done;
        }
        match command {
            Command::CropMode => self.selected_frame.is_some_and(|f| self.enter_crop(f)),
            Command::ZoomIn => self.crop_zoom(crop::ZOOM_STEP) && self.commit_crop(),
            Command::ZoomOut => self.crop_zoom(1.0 / crop::ZOOM_STEP) && self.commit_crop(),
            Command::ResetCrop => self.crop_reset() && self.commit_crop(),
            Command::ToggleReordered => {
                self.reordered = !self.reordered;
                true
            }
            Command::Undo => self.session.as_mut().and_then(Session::undo).is_some(),
            Command::Redo => self.session.as_mut().and_then(Session::redo).is_some(),
            Command::NextLayout => self.step(true),
            Command::PreviousLayout => self.step(false),
            Command::Mirror => self.mirror(),
            Command::Favorite => self.favorite(),
            Command::Reject => self.reject(),
            Command::ClearFrame if self.organizing() => self.delete_group(),
            Command::ClearFrame => self.delete_selected(),
            // Pela IA e pelo atalho de comando, sem a pergunta; a janela pergunta antes (TF-B11).
            Command::RemoveFromAlbum => {
                let photos = self.selected_frame_photos();
                self.remove_from_album(&photos)
            }
            Command::CopyFrames => self.copy_frames(),
            Command::PasteFrames => self.paste_frames(),
            Command::DuplicateFrames => self.duplicate_frames(),
            c if adjust::turn_of(c).is_some() => {
                adjust::turn_of(c).is_some_and(|t| self.adjust(adjust::AdjustChange::Turn(t)))
            }
            Command::NewText => self.new_text(None),
            Command::NewPhotoFrame => self.new_photo_frame(None),
            Command::BringForward => self.reorder_selected(true),
            Command::SendBackward => self.reorder_selected(false),
            Command::Deselect => {
                self.selected_frame = None;
                self.selection.clear();
                true
            }
            Command::SelectAllPhotos => self.select_all_photos(),
            Command::BuildFromSelection => self.build_from_selection(0),
            Command::NewSpreadFromSelection => self.new_spread_from_selection(),
            Command::NewSpread => self.new_spread(),
            Command::NewBlankSpread => self.new_blank_spread(false),
            Command::NewBlankSpreadBefore => self.new_blank_spread(true),
            Command::BlankPage => self.toggle_blank_page(),
            Command::GoToCover => self.go_to_cover(),
            Command::DeleteCover => self.delete_cover(),
            Command::ToggleCoverGuide => self.toggle_cover_guide(),
            // A janela abre a caixa, que chama `create_cover` ou `set_cover_measures`.
            Command::CreateCover | Command::CoverMeasures => false,
            Command::DeleteSpread => self.delete_current(),
            Command::ClearAllPhotos => self.clear_all_photos(),
            Command::DeleteAllSpreads => self.delete_all_spreads(),
            Command::MoveSpreadLeft => self.move_by(-1),
            Command::MoveSpreadRight => self.move_by(1),
            // Os de diálogo chegam aqui pela janela: desmarcam o quadro e não mudam nada.
            _ => {
                self.selected_frame = None;
                false
            }
        }
    }

    /// Clique num passo do painel de histórico (spec HI, regra 2). Um gesto de recorte em
    /// andamento é gravado antes.
    pub fn go_to_step(&mut self, step: usize) -> bool {
        self.exit_crop();
        self.gesture = None;
        self.selected_frame = None;
        self.session.as_mut().is_some_and(|s| s.go_to_step(step))
    }
}
