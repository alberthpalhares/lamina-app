//! A capa pela janela (spec CA, regras 5 a 7, 12, 18 e 19): abrir, criar, medir, apagar, soltar fotos
//! e ligar o guia. Com a capa aberta, a sessão fala da projeção, e o resto do editor vale igual.

use lamina_core::{
    Cover, CoverGuide, CoverMeasures, Crop, FrameContent, PhotoId, Project, RelRect,
};

use super::Editor;
use crate::cover_form::CoverForm;
use crate::edits::spread_photos;
use crate::{Command, EditorError};

/// Comandos só de lâmina: ficam desativados com a capa aberta (regra 5).
pub(super) const SPREAD_ONLY: [Command; 20] = [
    Command::ToggleLayoutBand,
    Command::Mirror,
    Command::BlankPage,
    Command::NewSpread,
    Command::NewBlankSpread,
    Command::NewBlankSpreadBefore,
    Command::NewSpreadFromSelection,
    Command::BuildFromSelection,
    Command::DeleteSpread,
    Command::MoveSpreadLeft,
    Command::MoveSpreadRight,
    Command::ClearAllPhotos,
    Command::DeleteAllSpreads,
    Command::RemoveFromAlbum,
    Command::SaveModel,
    Command::Models,
    Command::NextLayout,
    Command::PreviousLayout,
    Command::Specification,
    Command::MountAlbum,
];

/// A área da capa onde uma foto foi solta (regra 19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverZone {
    Back,
    Spine,
    Front,
}

/// A área sob `fx`, em frações da largura da capa com a dobra.
pub fn zone_at(canvas: &Project, fx: f64) -> CoverZone {
    let canvas_rect = canvas.spec.canvas_rect();
    let trim = canvas.spec.trim_rect();
    let x = fx * canvas_rect.width.get() - trim.x.get();
    let half = canvas.spec.safe_margins().inner.get() - canvas.spec.safe_margins().outer.get();
    let middle = trim.width.get() / 2.0;
    if x < middle - half {
        CoverZone::Back
    } else if x > middle + half {
        CoverZone::Front
    } else {
        CoverZone::Spine
    }
}

/// O quadro que preenche a área: a face até a dobra; na lombada, a capa inteira.
fn zone_rect(canvas: &Project, zone: CoverZone) -> Result<RelRect, EditorError> {
    let trim = canvas.spec.trim_rect();
    let half = canvas.spec.safe_margins().inner.get() - canvas.spec.safe_margins().outer.get();
    let spine = 2.0 * half / trim.width.get();
    let face = (1.0 - spine) / 2.0;
    let rect = match zone {
        CoverZone::Back => RelRect::new(0.0, 0.0, face, 1.0),
        CoverZone::Front => RelRect::new(face + spine, 0.0, face, 1.0),
        CoverZone::Spine => RelRect::new(0.0, 0.0, 1.0, 1.0),
    };
    Ok(rect?)
}

/// Põe a foto na área, tirando as fotos que a área cobre: o resto da capa fica.
pub(crate) fn fill_zone(
    canvas: &mut Project,
    zone: CoverZone,
    photo: PhotoId,
) -> Result<(), EditorError> {
    let rect = zone_rect(canvas, zone)?;
    let spread = canvas.spreads.first_mut().ok_or(EditorError::NoFrame)?;
    spread.frames.retain(|f| {
        matches!(f.content, FrameContent::Text(_))
            || f.rect.x() + f.rect.width() <= rect.x() + 1e-6
            || f.rect.x() >= rect.x() + rect.width() - 1e-6
    });
    let id = spread.id;
    let content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    canvas.add_frame(id, rect, content)?;
    // Os textos ficam por cima da foto nova.
    let spread = canvas.spreads.first_mut().ok_or(EditorError::NoFrame)?;
    spread
        .frames
        .sort_by_key(|f| matches!(f.content, FrameContent::Text(_)));
    spread.manual = true;
    Ok(())
}

/// O arranjo próprio da capa (regra 6): o motor das lâminas não conhece a lombada e punha foto em cima
/// dela. A frente leva a metade das fotos, arredondada para cima, e o verso, o resto, numa grade em
/// cada face, com o espaço entre fotos do álbum. Os textos ficam.
pub(crate) fn arrange(canvas: &mut Project, photos: &[PhotoId]) -> Result<(), EditorError> {
    let trim = canvas.spec.trim_rect();
    let (width, height) = (trim.width.get(), trim.height.get());
    let gap = canvas.gap.get();
    let front = zone_rect(canvas, CoverZone::Front)?;
    let back = zone_rect(canvas, CoverZone::Back)?;
    let on_front = photos.len().div_ceil(2);
    let spread = canvas.spreads.first_mut().ok_or(EditorError::NoFrame)?;
    spread
        .frames
        .retain(|f| matches!(f.content, FrameContent::Text(_)));
    spread.manual = true;
    let id = spread.id;
    let groups = [(front, &photos[..on_front]), (back, &photos[on_front..])];
    for (face, group) in groups {
        for (photo, rect) in group
            .iter()
            .zip(grid(face, group.len(), gap / width, gap / height))
        {
            let content = FrameContent::Photo {
                photo: *photo,
                crop: Crop::default(),
            };
            canvas.add_frame(id, rect?, content)?;
        }
    }
    let spread = canvas.spreads.first_mut().ok_or(EditorError::NoFrame)?;
    spread
        .frames
        .sort_by_key(|f| matches!(f.content, FrameContent::Text(_)));
    Ok(())
}

/// `n` retângulos em grade dentro de `face`, com os vãos dados (em frações da capa).
fn grid(
    face: RelRect,
    n: usize,
    gap_x: f64,
    gap_y: f64,
) -> impl Iterator<Item = Result<RelRect, lamina_core::CoreError>> {
    let columns = (1..=n.max(1)).find(|c| c * c >= n).unwrap_or(1);
    let rows = n.div_ceil(columns).max(1);
    #[allow(clippy::cast_precision_loss)] // poucas fotos
    let (c, r) = (columns as f64, rows as f64);
    let w = (face.width() - gap_x * (c - 1.0)) / c;
    let h = (face.height() - gap_y * (r - 1.0)) / r;
    (0..n).map(move |i| {
        #[allow(clippy::cast_precision_loss)]
        let (col, row) = ((i % columns) as f64, (i / columns) as f64);
        RelRect::new(
            face.x() + col * (w + gap_x),
            face.y() + row * (h + gap_y),
            w,
            h,
        )
    })
}

/// Várias fotos numa face, em grade, tirando as fotos que a face tinha (regra 19).
fn fill_zone_with(
    canvas: &mut Project,
    zone: CoverZone,
    photos: &[PhotoId],
) -> Result<(), EditorError> {
    let face = zone_rect(canvas, zone)?;
    let trim = canvas.spec.trim_rect();
    let gaps = (
        canvas.gap.get() / trim.width.get(),
        canvas.gap.get() / trim.height.get(),
    );
    let spread = canvas.spreads.first_mut().ok_or(EditorError::NoFrame)?;
    spread.frames.retain(|f| {
        matches!(f.content, FrameContent::Text(_))
            || f.rect.x() + f.rect.width() <= face.x() + 1e-6
            || f.rect.x() >= face.x() + face.width() - 1e-6
    });
    spread.manual = true;
    let id = spread.id;
    for (photo, rect) in photos.iter().zip(grid(face, photos.len(), gaps.0, gaps.1)) {
        let content = FrameContent::Photo {
            photo: *photo,
            crop: Crop::default(),
        };
        canvas.add_frame(id, rect?, content)?;
    }
    let spread = canvas.spreads.first_mut().ok_or(EditorError::NoFrame)?;
    spread
        .frames
        .sort_by_key(|f| matches!(f.content, FrameContent::Text(_)));
    Ok(())
}

/// A capa nasce com as fotos: uma cobre a capa inteira; duas ou mais, no arranjo da capa (regra 6).
fn fill_new(project: &mut Project, photos: &[PhotoId]) -> Result<(), EditorError> {
    let Some(mut canvas) = project.cover_canvas()? else {
        return Ok(());
    };
    match photos {
        [] => return Ok(()),
        [photo] => fill_zone(&mut canvas, CoverZone::Spine, *photo)?,
        many => arrange(&mut canvas, many)?,
    }
    project.absorb_cover(canvas);
    Ok(())
}

/// O guia sobre a capa: o arquivo e onde ele fica (x, y, largura, altura, em frações da largura
/// da capa com a dobra).
#[derive(Debug, Clone, PartialEq)]
pub struct GuideOverlay {
    pub path: std::path::PathBuf,
    pub at: (f64, f64, f64, f64),
}

/// O que a tela desenha sobre a capa aberta (regra 8), em frações da largura da capa com a dobra.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverOverlay {
    pub spine_left: f64,
    pub spine_right: f64,
    pub trim_top: f64,
    pub guide: Option<GuideOverlay>,
}

impl Editor {
    /// As linhas da lombada e o guia, com a capa aberta.
    pub fn cover_overlay(&self) -> Option<CoverOverlay> {
        let session = self.session.as_ref()?;
        if !session.on_cover() {
            return None;
        }
        let spec = &session.project().spec;
        let canvas = spec.canvas_rect();
        let trim = spec.trim_rect();
        let width = canvas.width.get();
        let half = spec.safe_margins().inner.get() - spec.safe_margins().outer.get();
        let middle = trim.x.get() + trim.width.get() / 2.0;
        let cover = session.album().cover.as_ref()?;
        let guide = cover
            .guide
            .as_ref()
            .filter(|_| !self.guide_hidden)
            .map(|g| {
                let area = if g.includes_wrap { canvas } else { trim };
                GuideOverlay {
                    path: g.path.clone(),
                    at: (
                        area.x.get() / width,
                        area.y.get() / width,
                        area.width.get() / width,
                        area.height.get() / width,
                    ),
                }
            });
        Some(CoverOverlay {
            spine_left: (middle - half) / width,
            spine_right: (middle + half) / width,
            trim_top: trim.y.get() / width,
            guide,
        })
    }

    /// O atalho do rolo na lâmina 1 (spec CA, seção 4).
    pub fn roll_pill(&self) -> crate::motion::RollPill {
        use crate::motion::RollPill;
        let Some(s) = self.session.as_ref() else {
            return RollPill::None;
        };
        if s.on_cover() || s.current() != 0 || !crate::motion::roll_shown(self) {
            return RollPill::None;
        }
        if s.album().cover.is_some() {
            RollPill::GoToCover
        } else {
            RollPill::CreateCover
        }
    }

    /// A capa está aberta?
    pub fn on_cover(&self) -> bool {
        self.session.as_ref().is_some_and(crate::Session::on_cover)
    }

    /// O álbum tem capa?
    pub fn has_cover(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| s.album().cover.is_some())
    }

    /// A capa do álbum.
    pub fn cover(&self) -> Option<&Cover> {
        self.session.as_ref()?.album().cover.as_ref()
    }

    /// Abre a capa; sem capa, nada muda.
    pub fn go_to_cover(&mut self) -> bool {
        self.exit_crop();
        self.gesture = None;
        self.selected_frame = None;
        let done = self
            .session
            .as_mut()
            .is_some_and(crate::Session::go_to_cover);
        self.settle_visit();
        done
    }

    /// Sai da capa, antes do que só vale no álbum (montar, limpar, tirar foto do álbum).
    pub(crate) fn leave_cover(&mut self) {
        if self.on_cover() {
            self.exit_crop();
            self.gesture = None;
            self.selected_frame = None;
        }
        if let Some(s) = self.session.as_mut() {
            s.leave_cover();
        }
    }

    /// A caixa "Medidas da capa", como ela abre agora.
    pub fn cover_form(&self) -> Option<CoverForm> {
        Some(CoverForm::of(self.session.as_ref()?.album()))
    }

    /// Cria a capa e a abre; com fotos, ela nasce com elas, numa edição só (regra 6).
    pub fn create_cover(
        &mut self,
        measures: CoverMeasures,
        guide: Option<CoverGuide>,
        photos: &[PhotoId],
    ) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let done = session.edit_album("Criar a capa", |p| {
            p.create_cover(measures)?;
            if let Some(cover) = p.cover.as_mut() {
                cover.guide = guide;
            }
            fill_new(p, photos)
        });
        match done {
            Ok(()) => self.go_to_cover(),
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }

    /// Muda as medidas: os quadros acompanham na mesma edição (regra 7).
    pub fn set_cover_measures(
        &mut self,
        measures: CoverMeasures,
        guide: Option<CoverGuide>,
    ) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let done = session.edit_album("Medidas da capa", |p| {
            measures.validate()?;
            let cover = p.cover.as_mut().ok_or(EditorError::NoFrame)?;
            cover.measures = measures;
            // Medir de novo em outra aba não apaga o guia que já estava na capa.
            if guide.is_some() {
                cover.guide = guide;
            }
            Ok::<(), EditorError>(())
        });
        match done {
            Ok(()) => true,
            Err(e) => {
                self.report(&e);
                false
            }
        }
    }

    /// Apaga a capa; Ctrl+Z traz de volta (regra 12).
    pub(super) fn delete_cover(&mut self) -> bool {
        self.selected_frame = None;
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        session
            .edit_album("Apagar a capa", |p| {
                p.cover = None;
                Ok::<(), EditorError>(())
            })
            .is_ok()
    }

    /// Mostra ou esconde o guia: é só da vista, fora do histórico (regra 18).
    pub(super) fn toggle_cover_guide(&mut self) -> bool {
        self.guide_hidden = !self.guide_hidden;
        true
    }

    fn cover_zone(&self, fx: f64) -> Option<CoverZone> {
        let session = self.session.as_ref()?;
        Some(zone_at(session.project(), fx))
    }

    /// Fotos soltas na capa aberta, fora dos quadros, com o ponteiro em `fx` (regra 19).
    pub fn drop_on_cover(&mut self, fx: f64, photos: &[PhotoId]) -> bool {
        if !self.on_cover() {
            return false;
        }
        let done = match photos {
            [] => false,
            [photo] => self.change("Colocar foto", |p, _| {
                let zone = zone_at(p, fx);
                fill_zone(p, zone, *photo)
            }),
            // Numa face, as fotos novas ficam nela, e a outra face não muda.
            many if !matches!(self.cover_zone(fx), Some(CoverZone::Spine)) => {
                self.change("Colocar fotos", |p, _| {
                    let zone = zone_at(p, fx);
                    fill_zone_with(p, zone, many)
                })
            }
            many => self.change("Colocar fotos", |p, i| {
                let mut all = spread_photos(p, i);
                let fresh: Vec<PhotoId> =
                    many.iter().copied().filter(|m| !all.contains(m)).collect();
                all.extend(fresh);
                arrange(p, &all)
            }),
        };
        if done {
            self.selection.clear();
        }
        done
    }
}
