//! A faixa de layouts (spec FL): aberta ou fechada, a opção sob o mouse e a prévia na lâmina
//! grande, feita pela mesma função que o clique usa. Nada disso vai para o arquivo nem para o
//! histórico (regras 4 e 14).

use lamina_core::Project;
use lamina_layout::LayoutCandidate;

use super::{Editor, Panel};
use crate::motion::{PanelKey, panel_key};
use crate::suggestions::{Part, Selection, apply};
use crate::text::{ALL_REJECTED, option_title};
use crate::{Command, EditorError};

/// A frase da linha das abas (spec AL, regra 20).
const FREQUENT_RISE: &str = "Os layouts que você usa com frequência sobem na lista.";

/// O que a janela precisa saber da faixa: se está aberta e a opção da prévia.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Band {
    pub open: bool,
    pub hover: Option<usize>,
}

/// Um cartão da faixa (seção 4): "3. Linhas" e a estrela do favorito. A nota só ordena (spec AL,
/// regra 20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub title: String,
    pub favorite: bool,
}

/// O estado guardado: a prévia vale só para a lista em que foi escolhida (regra 9).
#[derive(Debug, Default)]
pub(super) struct BandState {
    pub(super) open: bool,
    hover: Option<(usize, PanelKey)>,
}

impl Panel {
    /// Os cartões da faixa, na ordem das opções.
    pub fn cards(&self) -> Vec<Card> {
        self.options
            .iter()
            .enumerate()
            .map(|(i, c)| Card {
                title: format!("{}. {}", i + 1, option_title(&c.family)),
                favorite: self.favorite.get(i).copied().unwrap_or(false),
            })
            .collect()
    }

    /// O aviso da faixa (seção 6): sem fotos, seleção grande demais ou todas rejeitadas.
    pub fn notice(&self) -> String {
        match (&self.unavailable, self.all_rejected) {
            (Some(reason), _) => reason.to_string(),
            (None, true) => ALL_REJECTED.to_owned(),
            (None, false) => String::new(),
        }
    }

    /// A frase da faixa quando não há aviso: por que a ordem muda com o uso (spec AL, regra 20).
    pub fn hint(&self) -> &'static str {
        if self.notice().is_empty() {
            FREQUENT_RISE
        } else {
            ""
        }
    }
}

/// O que o clique na opção `c` faz na lâmina `index` (spec FL, regra 3): com fotos do navegador
/// (`chosen`), a montagem do "Montar lâmina", que tira todos os quadros antes; sem elas, `apply`.
/// A prévia na lâmina e as miniaturas passam por aqui também.
pub(super) fn apply_choice(
    p: &mut Project,
    index: usize,
    (sel, c): (&Selection, &LayoutCandidate),
    chosen: bool,
) -> Result<(), EditorError> {
    if chosen {
        // Álbum sem lâminas: a seleção monta a primeira (spec TR, regra 10).
        if p.spreads.is_empty() {
            p.add_spread();
        }
        p.spreads
            .get_mut(index)
            .ok_or(EditorError::NoFrame)?
            .frames
            .clear();
    }
    apply(p, index, sel, c)?;
    Ok(())
}

/// As teclas do menu Layout, e a próxima lâmina e a anterior (→ ←, Page Down e Page Up): com a
/// faixa aberta, limpam a prévia e a deixam aberta, com as opções da lâmina mostrada (regra 8).
/// Home e End fecham a faixa, como antes: o pedido do dono foi só das setas.
fn keeps_band(command: Command) -> bool {
    matches!(
        command,
        Command::NextLayout
            | Command::PreviousLayout
            | Command::NextSpread
            | Command::PreviousSpread
            | Command::Mirror
            | Command::Favorite
            | Command::Reject
            | Command::ToggleReordered
    )
}

impl Editor {
    /// A faixa agora. A prévia some quando a lista de cartões mudou desde que foi pedida.
    pub fn band(&self) -> Band {
        Band {
            open: self.band.open,
            hover: self.hovered().map(|(i, _)| i),
        }
    }

    /// A opção da prévia e o painel dela, se a lista ainda é a mesma.
    fn hovered(&self) -> Option<(usize, Panel)> {
        let (i, key) = self.band.hover.as_ref()?;
        let panel = self.panel(Part::Spread);
        (panel_key(&panel, self.session.as_ref()?) == *key).then_some((*i, panel))
    }

    /// Abre ou fecha (L, o botão e o menu; regra 2). Só abre num álbum com lâminas.
    pub fn toggle_band(&mut self) -> bool {
        if self.band.open {
            return self.close_band();
        }
        if self.session.as_ref().is_none_or(|s| s.spread_count() == 0) {
            return false;
        }
        self.band = BandState {
            open: true,
            hover: None,
        };
        true
    }

    /// Fecha e desfaz a prévia (clique fora, Esc, L, o botão; regra 7). `true` se estava aberta.
    pub fn close_band(&mut self) -> bool {
        std::mem::take(&mut self.band).open
    }

    /// O mouse parou num cartão (`Some`) ou saiu da faixa (`None`; regras 3 e 5). `true` se a
    /// visão da lâmina mudou.
    pub fn hover_option(&mut self, option: Option<usize>) -> bool {
        let before = self.band().hover;
        let chosen = option.filter(|_| self.band.open).and_then(|i| {
            let panel = self.panel(Part::Spread);
            let session = self.session.as_ref()?;
            (i < panel.options.len()).then(|| (i, panel_key(&panel, session)))
        });
        let after = chosen.as_ref().map(|(i, _)| *i);
        self.band.hover = chosen;
        before != after
    }

    /// Antes de um comando (regra 8): as teclas do menu Layout e a próxima lâmina e a anterior
    /// limpam a prévia e deixam a faixa aberta; L e o Esc ficam com o próprio comando; qualquer outro fecha a faixa antes de
    /// rodar. `true` se a faixa fechou.
    pub fn close_band_for(&mut self, command: Command) -> bool {
        match command {
            Command::ToggleLayoutBand | Command::Deselect => false,
            c if keeps_band(c) => {
                self.band.hover = None;
                false
            }
            _ => self.close_band(),
        }
    }

    /// Limpa a prévia e deixa a faixa como está (clique num cartão, ☆ e ×; regras 6 e 9).
    pub(super) fn clear_hover(&mut self) {
        self.band.hover = None;
    }

    /// A lâmina atual como ficaria com a opção `i` do painel, igual ao que o clique deixa
    /// (miniaturas da faixa e prévia na lâmina; spec FL, regra 3).
    pub fn preview(&self, panel: &Panel, i: usize) -> Option<Project> {
        let session = self.session.as_ref()?;
        let (sel, c) = (panel.selection.as_ref()?, panel.options.get(i)?);
        let mut project = session.project().clone();
        let chosen = !self.selection.is_empty();
        apply_choice(&mut project, session.current(), (sel, c), chosen).ok()?;
        Some(project)
    }

    /// As miniaturas da faixa: `show(i, álbum)` recebe o álbum com a opção `i` aplicada à
    /// lâmina atual, numa cópia só para todas (com 200 opções, copiar o álbum de 600 fotos a
    /// cada uma levava 300 ms; regra 13). Os quadros são os do clique; só os ids podem mudar.
    pub fn for_each_preview(&self, panel: &Panel, mut show: impl FnMut(usize, &Project)) {
        let (Some(session), Some(sel)) = (self.session.as_ref(), panel.selection.as_ref()) else {
            return;
        };
        let (index, chosen) = (session.current(), !self.selection.is_empty());
        let mut scratch = session.project().clone();
        let original = scratch.spreads.get(index).cloned();
        for (i, c) in panel.options.iter().enumerate() {
            if let (Some(spread), Some(o)) = (scratch.spreads.get_mut(index), &original) {
                spread.clone_from(o);
            }
            // Uma opção que não aplica mostra a lâmina como está: cada cartão continua na posição
            // da opção dele.
            if apply_choice(&mut scratch, index, (sel, c), chosen).is_err()
                && let (Some(spread), Some(o)) = (scratch.spreads.get_mut(index), &original)
            {
                spread.clone_from(o);
            }
            show(i, &scratch);
        }
    }

    /// O álbum com a opção da prévia aplicada à lâmina atual, numa cópia (regra 3).
    pub(super) fn band_preview(&self) -> Option<Project> {
        let (i, panel) = self.hovered().filter(|_| self.band.open)?;
        self.preview(&panel, i)
    }
}
