//! A foto escolhida para ver (spec VF, beta 9.1): a prévia da lateral direita e a foto ampliada
//! no centro da tela. A foto é a última clicada na grade — ou a do quadro selecionado na lâmina,
//! com o recorte dele; arrastar na prévia reenquadra a foto no quadro (conferência do dono,
//! 2026-11-04). As setas navegam pela lista da aba atual; Espaço abre e fecha. Ver não é edição;
//! reenquadrar é ("Reenquadrar foto", como no modo recorte).

use lamina_core::{FrameContent, PhotoId};

use super::Editor;

/// O estado da foto à vista: a escolhida e se a ampliada está aberta.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PhotoView {
    /// A última foto clicada (ou navegada) na aba atual.
    pub chosen: Option<PhotoId>,
    /// A foto ampliada no centro está aberta (com a foto da prévia).
    pub open: bool,
    /// O quadro da lâmina atual de onde a foto veio (arrastar na prévia reenquadrar nele).
    pub frame: Option<usize>,
}

impl Editor {
    /// O estado da prévia e da ampliada, para a tela.
    pub fn photo_view(&self) -> PhotoView {
        PhotoView {
            chosen: self.photo_view.chosen,
            open: self.photo_view.open,
            frame: self.photo_view.frame,
        }
    }

    /// A foto clicada na grade (simples, Ctrl ou Shift): a prévia passa a ser ela (spec VF,
    /// regra 1) e o painel da direita aparece se estava oculto. Quem chama também mexe na
    /// seleção (`click_photo`).
    pub fn view_photo_clicked(&mut self, shown: &[PhotoId], index: usize) {
        self.photo_view.chosen = shown.get(index).copied();
        self.photo_view.frame = None;
        self.photo_view.open = false;
        if self.photo_view.chosen.is_some() && !self.strips.strips.panel {
            self.panel_need
                .set(Some(super::panel_need::PanelNeed::PhotoView));
        }
    }

    /// Clicou num quadro com foto da lâmina: a prévia passa a ser a foto dele, com o recorte
    /// (conferência do dono, 2026-11-04). Quadro vazio ou de texto: a prévia da grade continua.
    pub fn view_frame_photo(&mut self, frame: usize) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let photo = s
            .project()
            .spreads
            .get(s.current())
            .and_then(|sp| sp.frames.get(frame))
            .and_then(|f| match &f.content {
                FrameContent::Photo { photo, .. } => Some(*photo),
                _ => None,
            });
        if let Some(photo) = photo {
            self.photo_view.chosen = Some(photo);
            self.photo_view.frame = Some(frame);
            self.photo_view.open = false;
            if !self.strips.strips.panel {
                self.panel_need
                    .set(Some(super::panel_need::PanelNeed::PhotoView));
            }
        }
    }

    /// A fração visível da foto na prévia (0..1 da imagem), com o recorte do quadro de onde ela
    /// veio; `None` sem recorte (foto inteira) ou sem quadro.
    pub fn preview_clip(&self) -> Option<(f64, f64, f64, f64)> {
        let frame = self.photo_view.frame?;
        let s = self.session.as_ref()?;
        let project = s.project();
        let f = project.spreads.get(s.current())?.frames.get(frame)?;
        let FrameContent::Photo { photo, crop } = &f.content else {
            return None;
        };
        let p = project.photo(*photo).ok()?;
        let rect = project.frame_rect(f);
        let aspect = rect.width.get() / rect.height.get();
        let crop = self
            .pending_crop
            .filter(|_| self.crop_frame == Some(frame))
            .unwrap_or(*crop);
        let region = crop.visible_region(f64::from(p.width_px), f64::from(p.height_px), aspect);
        Some((
            region.x / f64::from(p.width_px),
            region.y / f64::from(p.height_px),
            region.width / f64::from(p.width_px),
            region.height / f64::from(p.height_px),
        ))
    }

    /// Arrasto de `dx`, `dy` (frações da prévia) na prévia: reenquadra a foto no quadro de onde
    /// ela veio (provisório durante o arrasto, como o modo recorte; spec RC, regra 2).
    pub fn preview_drag(&mut self, dx: f64, dy: f64) -> bool {
        let Some(frame) = self.photo_view.frame else {
            return false;
        };
        // Mesmo caminho do modo recorte: o provisório em `pending_crop` com o quadro dele.
        if self.crop_frame != Some(frame) {
            self.commit_crop();
            self.crop_frame = Some(frame);
        }
        self.crop_drag(dx, dy)
    }

    /// Solta o arrasto da prévia: grava o reenquadramento como uma edição.
    pub fn preview_release(&mut self) -> bool {
        let Some(frame) = self.photo_view.frame else {
            return false;
        };
        if self.crop_frame != Some(frame) {
            return false;
        }
        let had = self.pending_crop.is_some();
        self.exit_crop();
        self.photo_view.frame = Some(frame);
        had
    }

    /// Espaço (fora do modo organizar): abre ou fecha a ampliada. `true` = mudou. Sem foto
    /// escolhida e com seleção de uma foto, vale ela (regra 2); sem nenhuma, `false`.
    pub fn toggle_photo_view(&mut self) -> bool {
        if self.photo_view.open {
            self.photo_view.open = false;
            return true;
        }
        if self.photo_view.chosen.is_none() {
            self.photo_view.chosen = self.selection.first().copied();
        }
        if self.photo_view.chosen.is_some() {
            self.photo_view.open = true;
            return true;
        }
        false
    }

    /// Fecha a ampliada (Esc ou clique fora).
    pub fn close_photo_view(&mut self) -> bool {
        let was = self.photo_view.open;
        self.photo_view.open = false;
        was
    }

    /// A prévia some (desmarcou tudo na grade, regra 1).
    pub fn clear_photo_selection(&mut self) {
        self.selection.clear();
        self.photo_view.chosen = None;
    }

    /// A foto escolhida, para os testes limparem só a escolha (regra 2).
    pub fn clear_photo_view_choice(&mut self) {
        self.photo_view.chosen = None;
    }

    /// As fotos selecionadas na grade, para conferir que a navegação não mexe nelas.
    pub fn photo_selection_ids(&self) -> Vec<PhotoId> {
        self.selection.clone()
    }

    /// ← e → com a ampliada aberta: a foto anterior ou seguinte da lista da aba atual,
    /// circulando (regra 4). A navegação troca a foto escolhida (prévia e ampliada, a mesma
    /// foto). `true` = mudou.
    pub fn step_photo_view(&mut self, forward: bool) -> bool {
        if !self.photo_view.open {
            return false;
        }
        let shown = self.shown_photos();
        let Some(current) = self.photo_view.chosen else {
            return false;
        };
        let Some(at) = shown.iter().position(|p| *p == current) else {
            return false;
        };
        if shown.len() < 2 {
            return false;
        }
        let next = if forward {
            (at + 1) % shown.len()
        } else {
            at.checked_sub(1).unwrap_or(shown.len() - 1)
        };
        self.photo_view.chosen = Some(shown[next]);
        self.photo_view.frame = None;
        true
    }
}
