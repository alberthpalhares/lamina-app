//! O painel temporário (spec JE, regras 5 e 6): com o painel da direita escondido, ele aparece
//! sozinho enquanto for preciso ("Ajustar a foto…", escrever um texto, o modo organizar), sem
//! mudar a escolha guardada em `janela.json`.

use lamina_core::{FrameId, SpreadId};

use super::Editor;

/// O que pediu o painel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelNeed {
    /// "Ajustar a foto…", T ou duplo clique num texto: enquanto aquele quadro continuar
    /// selecionado, naquela lâmina.
    Frame { spread: SpreadId, frame: FrameId },
    /// O modo organizar, enquanto ele estiver aberto.
    Organize,
    /// Uma foto à vista (prévia da lateral, spec VF, regra 1): enquanto houver foto escolhida.
    PhotoView,
}

/// Como a aba Quadro abre para o texto selecionado: só a aba (um clique no texto) ou a aba com o
/// cursor no campo (T, duplo clique, "Editar texto", a ferramenta Texto). Correção TX de
/// 2026-10-28.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFocus {
    Tab,
    Cursor,
}

impl Editor {
    /// O painel da direita aparece: a escolha guardada ou um pedido em curso (regra 5). Um pedido
    /// que acabou (o quadro desmarcado ou trocado, outra lâmina, fora do modo, o álbum fechado)
    /// é esquecido aqui e não volta.
    pub fn panel_shown(&self) -> bool {
        if self.strips.strips.panel {
            return true;
        }
        let Some(need) = self.panel_need.get() else {
            return false;
        };
        let alive = self.need_alive(need);
        if !alive {
            self.panel_need.set(None);
        }
        alive
    }

    fn need_alive(&self, need: PanelNeed) -> bool {
        match need {
            PanelNeed::Organize => self.organizing(),
            PanelNeed::Frame { spread, frame } => {
                self.selected_frame_key() == Some((spread, frame)) && !self.organizing()
            }
            PanelNeed::PhotoView => self.photo_view.chosen.is_some(),
        }
    }

    /// A lâmina atual e o quadro selecionado nela.
    fn selected_frame_key(&self) -> Option<(SpreadId, FrameId)> {
        let s = self.session.as_ref()?;
        let spread = s.project().spreads.get(s.current())?;
        let frame = spread.frames.get(self.selected_frame?)?;
        Some((spread.id, frame.id))
    }

    /// Pede o painel para o quadro selecionado. No modo organizar, o pedido do modo continua.
    pub(super) fn ask_panel_for_selected(&mut self) {
        if self.organizing() {
            return;
        }
        if let Some((spread, frame)) = self.selected_frame_key() {
            self.panel_need
                .set(Some(PanelNeed::Frame { spread, frame }));
        }
    }

    /// Leva ao campo do texto selecionado: sai do modo organizar (lá não há campo) com o texto
    /// ainda selecionado, pede o painel e a aba Quadro. `false` = não há texto selecionado.
    pub(super) fn show_text(&mut self, focus: TextFocus) -> bool {
        if self.selected_text().is_none() {
            return false;
        }
        if self.organizing() {
            let frame = self.selected_frame;
            self.set_organizing(false);
            self.selected_frame = frame;
        }
        self.ask_panel_for_selected();
        self.text_focus = Some(focus);
        true
    }

    /// A janela abre a aba Quadro (e põe o cursor no texto)? Ler limpa o pedido.
    pub fn take_text_focus(&mut self) -> Option<TextFocus> {
        self.text_focus.take()
    }

    /// Entrar no modo organizar pede o painel; sair encerra o pedido.
    pub(super) fn ask_panel_for_mode(&mut self, on: bool) {
        self.panel_need.set(on.then_some(PanelNeed::Organize));
    }

    /// O botão Painel e Alt+4 alternam o que se vê (regra 6). Com o painel aparecendo por um
    /// pedido, escondem e encerram o pedido, sem gravar; senão, mudam a escolha e gravam.
    pub(super) fn toggle_panel(&mut self) -> bool {
        let shown = self.panel_shown();
        self.panel_need.set(None);
        if shown && !self.strips.strips.panel {
            return true;
        }
        self.strips.strips.panel = !shown;
        self.save_strips();
        true
    }
}
