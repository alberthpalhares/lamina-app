//! Sugestões de layout da lâmina atual e as edições que trocam ou espelham o layout (spec TL,
//! regras 1 a 5). Funções puras sobre o projeto: o [`crate::Editor`] guarda o cache e chama.

use lamina_core::{ActivePages, CoreError, Frame, FrameContent, Mm, PhotoId, Project, RelRect};
use lamina_layout::{LayoutCandidate, LayoutError, LayoutLists, LayoutOptions, layout_options};

pub use crate::applying::{apply, keep_center};

/// Folga para dizer que dois quadros estão no mesmo lugar: 0,1 mm (regra 2).
const SAME_MM: f64 = 0.1;
/// Folga para dizer que um quadro passa da lombada (fração da lâmina).
const SPINE_EPS: f64 = 1e-6;

/// Que parte da lâmina troca de layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    Spread,
    Left,
    Right,
}

/// Qual das duas listas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum List {
    #[default]
    InOrder,
    Reordered,
}

/// As fotos de uma parte da lâmina, na ordem dos quadros, e as páginas ativas para o motor.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub photos: Vec<PhotoId>,
    pub aspects: Vec<f64>,
    pub active: ActivePages,
    /// A lâmina inteira: a troca tira também os quadros vazios (spec FQ, regra 4).
    pub whole: bool,
    /// Espaço entre fotos da lâmina (spec AP).
    pub gap: Mm,
}

/// Por que uma parte não pode trocar de layout (as mensagens da seção 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    NoPhotos,
    PageWithoutPhotos(Part),
    CrossesSpine,
    EdgeSpread,
    /// Seleção de fotos maior do que a lâmina comporta (spec FQ, regra 5).
    TooMany(usize),
    /// A lâmina não tem página com conteúdo (spec PB, regra 7).
    BlankSpread,
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let page = |p: &Part| {
            if *p == Part::Left {
                "esquerda"
            } else {
                "direita"
            }
        };
        match self {
            Unavailable::NoPhotos => write!(f, "Esta lâmina não tem fotos."),
            Unavailable::PageWithoutPhotos(p) => write!(f, "A página {} não tem fotos.", page(p)),
            Unavailable::CrossesSpine => write!(
                f,
                "Esta lâmina tem foto sobre a lombada: troque o layout da lâmina inteira."
            ),
            Unavailable::EdgeSpread => write!(f, "Lâmina de ponta: não dá para espelhar."),
            Unavailable::TooMany(n) => write!(f, "Selecione até {n} fotos para uma lâmina."),
            Unavailable::BlankSpread => write!(f, "{}", crate::blank::BLANK_SPREAD),
        }
    }
}

pub(crate) fn crosses_spine(r: &RelRect) -> bool {
    r.x() < 0.5 - SPINE_EPS && r.x() + r.width() > 0.5 + SPINE_EPS
}

pub(crate) fn on_part(r: &RelRect, part: Part) -> bool {
    match part {
        Part::Spread => true,
        Part::Left => r.x() + r.width() <= 0.5 + SPINE_EPS,
        Part::Right => r.x() >= 0.5 - SPINE_EPS,
    }
}

pub(crate) fn photo_of(frame: &Frame) -> Option<PhotoId> {
    match &frame.content {
        FrameContent::Photo { photo, .. } => Some(*photo),
        _ => None,
    }
}

/// As fotos da `part` da lâmina `index`, ou por que ela não pode trocar de layout.
pub fn selection(project: &Project, index: usize, part: Part) -> Result<Selection, Unavailable> {
    let spread = project.spreads.get(index).ok_or(Unavailable::NoPhotos)?;
    let photo_frames: Vec<&Frame> = spread
        .frames
        .iter()
        .filter(|f| photo_of(f).is_some())
        .collect();
    if photo_frames.is_empty() {
        return Err(Unavailable::NoPhotos);
    }
    if part != Part::Spread && photo_frames.iter().any(|f| crosses_spine(&f.rect)) {
        return Err(Unavailable::CrossesSpine);
    }
    let photos: Vec<PhotoId> = photo_frames
        .iter()
        .filter(|f| on_part(&f.rect, part))
        .filter_map(|f| photo_of(f))
        .collect();
    if photos.is_empty() {
        return Err(Unavailable::PageWithoutPhotos(part));
    }
    let aspects = photos
        .iter()
        .filter_map(|id| {
            project
                .photo(*id)
                .ok()
                .map(lamina_core::Photo::aspect_ratio)
        })
        .collect();
    let active = match part {
        Part::Spread => project.content_pages(index),
        Part::Left => ActivePages {
            left: true,
            right: false,
        },
        Part::Right => ActivePages {
            left: false,
            right: true,
        },
    };
    Ok(Selection {
        photos,
        aspects,
        active,
        whole: part == Part::Spread,
        gap: project.spread_gap(spread),
    })
}

/// As duas listas do motor para a seleção, com o espaço entre fotos da lâmina (regra 1; spec
/// AP, regra 1).
pub fn lists(project: &Project, sel: &Selection) -> Result<LayoutLists, LayoutError> {
    let options = LayoutOptions {
        gap: sel.gap,
        mounting: project.mounting,
        ..LayoutOptions::default()
    };
    layout_options(&project.spec, sel.active, &sel.aspects, &options)
}

/// O candidato descreve exatamente os quadros atuais das fotos da seleção? (regra 2)
pub fn is_in_use(project: &Project, index: usize, sel: &Selection, c: &LayoutCandidate) -> bool {
    let Some(spread) = project.spreads.get(index) else {
        return false;
    };
    let width = project.spec.trim_rect().width.get();
    let height = project.spec.trim_rect().height.get();
    let close = |a: &RelRect, b: &RelRect| {
        (a.x() - b.x()).abs() * width <= SAME_MM
            && (a.y() - b.y()).abs() * height <= SAME_MM
            && (a.width() - b.width()).abs() * width <= SAME_MM
            && (a.height() - b.height()).abs() * height <= SAME_MM
    };
    sel.photos.iter().zip(&c.frames).all(|(photo, rect)| {
        spread
            .frames
            .iter()
            .any(|f| photo_of(f) == Some(*photo) && close(&f.rect, rect))
    })
}

/// Espelha a lâmina: cada quadro vai para a posição espelhada, com a mesma foto (regra 5).
pub fn mirror(project: &mut Project, index: usize) -> Result<(), CoreError> {
    let Some(spread) = project.spreads.get_mut(index) else {
        return Ok(());
    };
    for frame in &mut spread.frames {
        let r = frame.rect;
        // Quadro até a borda: o arredondamento pode dar uma posição um fio abaixo de zero.
        let x = (1.0 - r.x() - r.width()).max(0.0);
        frame.rect = RelRect::new(x, r.y(), r.width(), r.height())?;
    }
    // A marca vai junto com o conteúdo (spec PB, regra 9).
    std::mem::swap(&mut spread.blank.left, &mut spread.blank.right);
    Ok(())
}

/// A lâmina pode ser espelhada? Não nas pontas com página simples.
pub fn can_mirror(project: &Project, index: usize) -> Result<(), Unavailable> {
    let active = project
        .spec
        .edges()
        .active_pages(index, project.spreads.len());
    if !(active.left && active.right) {
        return Err(Unavailable::EdgeSpread);
    }
    selection(project, index, Part::Spread).map(|_| ())
}
