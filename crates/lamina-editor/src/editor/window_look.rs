//! As preferências da janela no editor (spec AJ): mudar, gravar e voltar ao padrão. Nada aqui entra
//! no histórico nem marca o álbum como alterado (regra 6).

use super::Editor;
use crate::motion::Durations;
use crate::window_look::{
    BackdropChoice, MotionChoice, TEXT_SIZE_DEFAULT, ThemeChoice, WindowLook, text_size,
};

impl Editor {
    /// As quatro escolhas de agora.
    pub fn window_look(&self) -> WindowLook {
        let strips = &self.strips.strips;
        WindowLook {
            theme: strips.theme,
            backdrop: strips.backdrop,
            text_size: strips.text_size,
            motion: strips.motion_choice(),
        }
    }

    pub fn set_theme_choice(&mut self, choice: ThemeChoice) {
        self.strips.strips.theme = choice;
        self.save_strips();
    }

    pub fn set_backdrop_choice(&mut self, choice: BackdropChoice) {
        self.strips.strips.backdrop = choice;
        self.save_strips();
    }

    /// O tamanho do texto, em porcentagem; um valor fora da lista vai para o válido mais próximo.
    pub fn set_text_size(&mut self, percent: u32) {
        self.strips.strips.text_size = text_size(f64::from(percent));
        self.save_strips();
    }

    pub fn set_motion_choice(&mut self, choice: MotionChoice) {
        self.strips.strips.set_motion(choice);
        self.save_strips();
    }

    /// "Voltar ao padrão" (regra 5): as quatro escolhas, de uma vez só.
    pub fn reset_window_look(&mut self) {
        let strips = &mut self.strips.strips;
        strips.theme = ThemeChoice::default();
        strips.backdrop = BackdropChoice::default();
        strips.text_size = TEXT_SIZE_DEFAULT;
        strips.set_motion(MotionChoice::default());
        self.save_strips();
    }

    /// A resposta do sistema sobre o movimento (regra 18). Não grava nada.
    pub fn set_system_motion(&mut self, reduces: Option<bool>) {
        self.system_motion = reduces;
    }

    /// A última resposta do sistema sobre o movimento.
    pub fn system_motion(&self) -> Option<bool> {
        self.system_motion
    }

    /// O que vale agora, com a escolha já resolvida (regra 19).
    pub fn reduce_motion(&self) -> bool {
        self.strips
            .strips
            .motion_choice()
            .reduces(self.system_motion)
    }

    /// As durações dos movimentos agora (zero sem movimento).
    pub fn durations(&self) -> Durations {
        Durations::new(self.reduce_motion())
    }

    /// `ver.reduzir-movimento` (regra 7) grava nos dois campos; se não gravar, vale nesta sessão.
    pub(super) fn toggle_reduce_motion(&mut self) -> bool {
        let next = self.strips.strips.motion_choice().toggled();
        self.set_motion_choice(next);
        true
    }
}
