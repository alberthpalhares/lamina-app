//! O modo organizar a lâmina (spec EG): entrar e sair, a seleção de vários quadros (guardada pelos
//! ids, para sobreviver ao desfazer) e o laço; as setas e o zoom ficam em `organize_moves.rs`. O
//! modo, a seleção e o zoom não vão para o arquivo (regra 3).

use lamina_core::{FrameId, SpreadId};

use super::Editor;
use crate::commands::Mode;
use crate::view::{Area, spread_view};

/// A ferramenta da paleta (spec ZR, regra 2): mora no editor, para a tecla Z e as dicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaletteTool {
    #[default]
    Select,
    Frame,
    Text,
    Hand,
    Magnifier,
}

impl PaletteTool {
    pub const ALL: [PaletteTool; 5] = [
        PaletteTool::Select,
        PaletteTool::Frame,
        PaletteTool::Text,
        PaletteTool::Hand,
        PaletteTool::Magnifier,
    ];

    /// A posição na paleta (0 a 4), a mesma da janela.
    pub fn index(self) -> i32 {
        match self {
            PaletteTool::Select => 0,
            PaletteTool::Frame => 1,
            PaletteTool::Text => 2,
            PaletteTool::Hand => 3,
            PaletteTool::Magnifier => 4,
        }
    }

    /// A ferramenta da posição `i`; fora de 0 a 4, Selecionar.
    pub fn from_index(i: i32) -> PaletteTool {
        PaletteTool::ALL
            .into_iter()
            .find(|t| t.index() == i)
            .unwrap_or_default()
    }
}

/// O estado do modo organizar.
#[derive(Debug, Default)]
pub(super) struct Organizing {
    on: bool,
    /// A lâmina da seleção e os quadros, na ordem dos cliques (o primeiro é a referência).
    spread: Option<SpreadId>,
    ids: Vec<FrameId>,
    /// O zoom, a vista (pixels a partir do centro de Ajustar), a duração da última mudança e a
    /// ferramenta (spec ZR).
    pub(super) zoom: super::organize_zoom::Zoom,
    pub(super) pan: (f64, f64),
    pub(super) ms: u32,
    pub(super) tool: PaletteTool,
}

/// O que o duplo clique na lâmina fez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleClick {
    /// Fora dos quadros: entrou no modo organizar.
    Organize,
    /// Numa foto: abriu o recorte.
    Crop,
    /// Num texto: o cursor vai para o campo do texto.
    Text,
    /// Numa foto ausente: "Localizar esta foto…" (spec RF, regra 8).
    Locate(lamina_core::PhotoId),
    Nothing,
}

/// O painel da direita (EG-B23): as abas de sempre ou o painel do modo organizar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePanel {
    Tabs,
    Organize,
}

impl Editor {
    pub fn organizing(&self) -> bool {
        self.organize.on
    }

    pub fn mode(&self) -> Mode {
        if self.organize.on {
            Mode::Organize
        } else {
            Mode::Normal
        }
    }

    pub fn side_panel(&self) -> SidePanel {
        if self.organize.on {
            SidePanel::Organize
        } else {
            SidePanel::Tabs
        }
    }

    /// Entra ou sai do modo. Sair desmarca os quadros e volta ao zoom "Ajustar"; a tela anima a
    /// saída (spec ZR, regra 12), o editor sai na hora.
    pub(super) fn set_organizing(&mut self, on: bool) -> bool {
        if self.session.is_none() || self.organize.on == on {
            return false;
        }
        self.organize = Organizing {
            on,
            ms: if on { 0 } else { self.durations().navigate },
            ..Organizing::default()
        };
        // Com o painel escondido, ele aparece enquanto o modo estiver aberto (spec JE, regra 5).
        self.ask_panel_for_mode(on);
        if on {
            // O modo abre a aba Detalhes (spec AF, regra 10) e fecha a faixa de layouts (spec FL,
            // regra 10).
            self.wants_details = true;
            self.close_band();
        } else {
            self.selected_frame = None;
        }
        true
    }

    /// Duplo clique na lâmina grande: fora dos quadros (`None`) entra no modo; numa foto abre o
    /// recorte; num texto, o campo do texto (spec EG, regra de entrada; EG-B2).
    pub fn double_click(&mut self, frame: Option<usize>) -> DoubleClick {
        let Some(frame) = frame else {
            self.set_organizing(true);
            return if self.organize.on {
                DoubleClick::Organize
            } else {
                DoubleClick::Nothing
            };
        };
        self.select_frame(Some(frame));
        if let Some(photo) = self.missing_photo_at(frame) {
            return DoubleClick::Locate(photo);
        }
        // É no painel que se digita o texto (spec JE, regra 5); no modo organizar, sai dele.
        if self.show_text(super::TextFocus::Cursor) {
            DoubleClick::Text
        } else if self.enter_crop(frame) {
            DoubleClick::Crop
        } else {
            DoubleClick::Nothing
        }
    }

    /// Os ids dos quadros da lâmina atual, na ordem dela.
    fn current_ids(&self) -> Option<(SpreadId, Vec<FrameId>)> {
        let s = self.session.as_ref()?;
        let spread = s.project().spreads.get(s.current())?;
        Some((spread.id, spread.frames.iter().map(|f| f.id).collect()))
    }

    /// Clique num quadro na lâmina (spec GJ): na edição comum e no modo organizar, Ctrl ou Shift
    /// soma ou tira e monta a seleção de quadros; sem eles, só seleciona um. Com foto no quadro,
    /// a prévia da lateral passa a ser ela (spec VF, conferência do dono 2026-11-04).
    pub fn frame_click(&mut self, frame: usize, ctrl: bool, shift: bool) {
        self.view_frame_photo(frame);
        self.organize_click(frame, ctrl, shift);
    }

    /// Clique num quadro. No modo organizar (EG) ou na edição comum (spec GJ), Ctrl ou Shift soma
    /// ou tira e monta a seleção de quadros; sem eles, só seleciona um.
    pub fn organize_click(&mut self, frame: usize, ctrl: bool, shift: bool) {
        let Some((spread, ids)) = self.current_ids() else {
            return;
        };
        let Some(&id) = ids.get(frame) else {
            return;
        };
        if !self.organize.on && !ctrl && !shift {
            self.organize.ids.clear();
            self.select_frame(Some(frame));
            return;
        }
        if self.organize.spread != Some(spread) {
            self.organize.ids.clear();
            self.organize.spread = Some(spread);
        }
        if ctrl || shift {
            if let Some(at) = self.organize.ids.iter().position(|i| *i == id) {
                self.organize.ids.remove(at);
            } else {
                self.organize.ids.push(id);
            }
        } else {
            self.organize.ids = vec![id];
        }
        self.sync_reference();
    }

    /// Seleciona os quadros `ids` da lâmina atual (os colados). Fora do modo, o primeiro deles.
    pub(super) fn select_ids(&mut self, ids: Vec<FrameId>) {
        self.organize.spread = self.current_ids().map(|(s, _)| s);
        self.organize.ids = ids;
        self.sync_reference();
    }

    /// O quadro de referência (o primeiro clicado) vira o quadro selecionado de sempre.
    fn sync_reference(&mut self) {
        let reference = self.organize.ids.first().copied();
        self.selected_frame = self
            .current_ids()
            .and_then(|(_, ids)| reference.and_then(|r| ids.iter().position(|i| *i == r)));
    }

    /// Os quadros selecionados da lâmina atual, na ordem da lâmina. Fora do modo organizar, o
    /// quadro selecionado de sempre — ou a seleção múltipla da edição comum, quando ela tem 2 ou
    /// mais (spec GJ).
    pub fn selected_frames(&self) -> Vec<usize> {
        if !self.organize.on
            && let Some((spread, ids)) = self.current_ids()
            && self.organize.spread == Some(spread)
            && self.organize.ids.len() > 1
            && ids.iter().any(|i| self.organize.ids.contains(i))
        {
            let sel: Vec<usize> = (0..ids.len())
                .filter(|i| self.organize.ids.contains(&ids[*i]))
                .collect();
            return sel;
        }
        if !self.organize.on {
            return self.selected_frame.into_iter().collect();
        }
        let Some((spread, ids)) = self.current_ids() else {
            return Vec::new();
        };
        if self.organize.spread != Some(spread) {
            return Vec::new();
        }
        (0..ids.len())
            .filter(|i| self.organize.ids.contains(&ids[*i]))
            .collect()
    }

    /// "1 quadro", "3 quadros" (nada selecionado: vazio).
    pub fn selection_text(&self) -> String {
        match self.selected_frames().len() {
            0 => String::new(),
            1 => "1 quadro".to_owned(),
            n => format!("{n} quadros"),
        }
    }

    pub(super) fn select_all_frames(&mut self) -> bool {
        let Some((spread, ids)) = self.current_ids() else {
            return false;
        };
        self.organize.spread = Some(spread);
        self.organize.ids = ids;
        self.sync_reference();
        !self.organize.ids.is_empty()
    }

    /// Desmarca os quadros (modo organizar ou edição comum); `true` se havia algum.
    pub fn deselect_frames(&mut self) -> bool {
        self.clear_frames()
    }

    /// Desmarca os quadros do modo; `true` se havia algum.
    pub(super) fn clear_frames(&mut self) -> bool {
        let had = !self.selected_frames().is_empty();
        self.organize.ids.clear();
        self.selected_frame = None;
        had
    }

    /// O laço de `a` a `b` (frações da largura da lâmina): pega os quadros inteiros dentro dele;
    /// com `add`, soma à seleção.
    pub fn lasso(&mut self, a: (f64, f64), b: (f64, f64), add: bool) {
        let (Some((spread, ids)), Some(s)) = (self.current_ids(), self.session.as_ref()) else {
            return;
        };
        let canvas = s.project().spec.canvas_rect();
        let area = Area {
            width: 1.0,
            height: canvas.height.get() / canvas.width.get(),
            margin: 0.0,
        };
        let Some(view) = spread_view(s.project(), s.current(), area, &[]) else {
            return;
        };
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let inside = view.frames.iter().enumerate().filter(|(_, f)| {
            let r = f.rect;
            r.x >= x0 && r.y >= y0 && r.x + r.width <= x1 && r.y + r.height <= y1
        });
        let caught: Vec<FrameId> = inside.map(|(i, _)| ids[i]).collect();
        if !add || self.organize.spread != Some(spread) {
            self.organize.ids.clear();
        }
        self.organize.spread = Some(spread);
        for id in caught {
            if !self.organize.ids.contains(&id) {
                self.organize.ids.push(id);
            }
        }
        self.sync_reference();
    }
}
