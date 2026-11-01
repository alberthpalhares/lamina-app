//! A capa fotográfica (spec CA): medidas, lombada e a projeção como uma lâmina de outro tamanho.
//!
//! O conteúdo é uma lâmina comum, com quadros relativos ao corte da capa aberta. Para desenhar e
//! editar, a capa vira um projeto de uma lâmina só, com uma especificação equivalente: a lombada
//! vira margem interna, e a dobra, sangria. Nem a geometria nem o desenho precisam saber de capa.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Project, Spread};
use crate::{AlbumSpec, CoreError, DesignMode, Margins, Mm, RelRect};

/// Espessura padrão de cada folha: lay-flat (regra 2).
pub const DEFAULT_SHEET: Mm = Mm(1.0);
/// A parte que vira para dentro da capa dura, por padrão.
pub const DEFAULT_WRAP: Mm = Mm(15.0);
/// Zona segura padrão da capa.
pub const DEFAULT_COVER_SAFE: Mm = Mm(10.0);
/// A face padrão: a página mais 3 mm na largura e 3 mm em cima e embaixo.
const FACE_EXTRA_WIDTH: f64 = 3.0;
const FACE_EXTRA_HEIGHT: f64 = 6.0;
/// Limites das medidas (seção 6): fora deles, a caixa recusa e o arquivo não abre.
const FACE_MIN: f64 = 50.0;
const FACE_MAX: f64 = 1000.0;
const SPINE_MAX: f64 = 100.0;

/// A largura da lombada: calculada pelas páginas do álbum ou fixa (regra 2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum Spine {
    #[serde(rename = "calculada")]
    Auto {
        #[serde(rename = "folha")]
        sheet: Mm,
    },
    #[serde(rename = "fixa")]
    Fixed {
        mm: Mm,
        /// As páginas do álbum na hora de fixar: o aviso compara com elas (regra 14).
        #[serde(rename = "paginas")]
        pages: usize,
    },
}

/// De onde vieram as medidas: a caixa reabre nessa aba, com o que foi digitado (correção C1).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub enum CoverOrigin {
    #[default]
    #[serde(rename = "album")]
    Album,
    #[serde(rename = "total")]
    Total {
        #[serde(rename = "largura")]
        width: Mm,
        #[serde(rename = "altura")]
        height: Mm,
        #[serde(rename = "inclui_dobra")]
        includes_wrap: bool,
    },
    #[serde(rename = "guia")]
    Guide,
}

/// As medidas da capa. O resto do código só lê as faces, a lombada e a dobra.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CoverMeasures {
    #[serde(rename = "face_largura")]
    pub face_width: Mm,
    #[serde(rename = "face_altura")]
    pub face_height: Mm,
    #[serde(rename = "lombada")]
    pub spine: Spine,
    #[serde(rename = "dobra")]
    pub wrap: Mm,
    #[serde(rename = "zona_segura")]
    pub safe: Mm,
    #[serde(rename = "origem", default)]
    pub origin: CoverOrigin,
}

/// A imagem do laboratório por cima da capa, só na tela (regra 18).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CoverGuide {
    #[serde(rename = "caminho")]
    pub path: PathBuf,
    #[serde(rename = "inclui_dobra")]
    pub includes_wrap: bool,
    #[serde(rename = "visivel")]
    pub visible: bool,
}

/// A capa do álbum: fica fora de `Project::spreads` e da numeração (regras 1 e 10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cover {
    #[serde(rename = "medidas")]
    pub measures: CoverMeasures,
    #[serde(rename = "conteudo")]
    pub content: Spread,
    #[serde(rename = "guia", default, skip_serializing_if = "Option::is_none")]
    pub guide: Option<CoverGuide>,
}

fn invalid(field: &'static str, value: f64) -> CoreError {
    CoreError::InvalidCover { field, value }
}

impl CoverMeasures {
    /// As medidas que a caixa abre preenchidas (seção 3).
    pub fn defaults(album: &AlbumSpec) -> Self {
        CoverMeasures {
            face_width: Mm(album.page_width().get() + FACE_EXTRA_WIDTH),
            face_height: Mm(album.page_height().get() + FACE_EXTRA_HEIGHT),
            spine: Spine::Auto {
                sheet: DEFAULT_SHEET,
            },
            wrap: DEFAULT_WRAP,
            safe: DEFAULT_COVER_SAFE,
            origin: CoverOrigin::Album,
        }
    }

    /// As medidas a partir da medida total do laboratório (regra 17).
    pub fn from_total(
        (width, height): (Mm, Mm),
        (spine, wrap, safe): (Mm, Mm, Mm),
        includes_wrap: bool,
        pages: usize,
    ) -> Result<Self, CoreError> {
        let fold = if includes_wrap { wrap.get() } else { 0.0 };
        let face_width = (width.get() - spine.get() - 2.0 * fold) / 2.0;
        let face_height = height.get() - 2.0 * fold;
        if face_width <= 0.0 || face_height <= 0.0 {
            return Err(invalid("total", width.get()));
        }
        let measures = CoverMeasures {
            face_width: Mm(face_width),
            face_height: Mm(face_height),
            spine: Spine::Fixed { mm: spine, pages },
            wrap,
            safe,
            origin: CoverOrigin::Total {
                width,
                height,
                includes_wrap,
            },
        };
        measures.validate()?;
        Ok(measures)
    }

    /// A largura da lombada num álbum de `pages` páginas (regra 2).
    pub fn spine_width(&self, pages: usize) -> Mm {
        match self.spine {
            Spine::Auto { sheet } => Mm(sheet.get() * pages.div_ceil(2) as f64),
            Spine::Fixed { mm, .. } => mm,
        }
    }

    /// Confere os limites da seção 6; a face pode ser menor que a página (é só um aviso).
    pub fn validate(&self) -> Result<(), CoreError> {
        let in_range = |v: f64, min: f64, max: f64| v.is_finite() && v >= min && v <= max;
        let (width, height) = (self.face_width.get(), self.face_height.get());
        if !in_range(width, FACE_MIN, FACE_MAX) {
            return Err(invalid("face", width));
        }
        if !in_range(height, FACE_MIN, FACE_MAX) {
            return Err(invalid("face", height));
        }
        let spine = match self.spine {
            Spine::Auto { sheet } => sheet.get(),
            Spine::Fixed { mm, .. } => mm.get(),
        };
        if !in_range(spine, 0.0, SPINE_MAX) {
            return Err(invalid("lombada", spine));
        }
        if !in_range(self.wrap.get(), 0.0, SPINE_MAX) {
            return Err(invalid("dobra", self.wrap.get()));
        }
        let limit = width.min(height) / 2.0;
        let safe = self.safe.get();
        if !safe.is_finite() || safe < 0.0 || safe >= limit {
            return Err(invalid("zona_segura", safe));
        }
        Ok(())
    }

    /// A especificação equivalente (regra 3): o corte é a capa aberta, e o canvas inclui a dobra.
    pub fn spec(&self, album: &AlbumSpec, pages: usize) -> Result<AlbumSpec, CoreError> {
        let half = self.spine_width(pages).get() / 2.0;
        AlbumSpec::new(
            Mm(self.face_width.get() + half),
            self.face_height,
            album.dpi(),
        )?
        .with_bleed_margins(Margins::new(self.wrap, self.wrap, Mm::ZERO))?
        .with_safe_margins(Margins::new(
            self.safe,
            self.safe,
            Mm(half + self.safe.get()),
        ))
        .map(|spec| spec.with_mode(DesignMode::Spread))
    }

    /// A largura total da capa aberta, sem a dobra.
    pub fn open_width(&self, pages: usize) -> Mm {
        Mm(2.0 * self.face_width.get() + self.spine_width(pages).get())
    }
}

impl Project {
    /// As páginas impressas do álbum: a capa não conta (regra 10).
    pub fn page_count(&self) -> usize {
        self.spec.edges().page_count(self.spreads.len())
    }

    /// As lâminas e a capa: o que olha o álbum inteiro (fotos usadas, fontes) passa por aqui.
    pub fn all_content(&self) -> impl Iterator<Item = &Spread> {
        self.spreads
            .iter()
            .chain(self.cover.as_ref().map(|c| &c.content))
    }

    /// A largura atual da lombada, com capa.
    pub fn cover_spine(&self) -> Option<Mm> {
        let cover = self.cover.as_ref()?;
        Some(cover.measures.spine_width(self.page_count()))
    }

    /// O projeto que se desenha e se edita na capa: uma lâmina só, com a especificação da capa, sem
    /// copiar as lâminas do álbum (regra 4). `None` = o álbum não tem capa.
    pub fn cover_canvas(&self) -> Result<Option<Project>, CoreError> {
        let Some(cover) = self.cover.as_ref() else {
            return Ok(None);
        };
        Ok(Some(Project {
            schema_version: self.schema_version,
            name: self.name.clone(),
            spec: cover.measures.spec(&self.spec, self.page_count())?,
            gap: self.gap,
            photos: self.photos.clone(),
            spreads: vec![cover.content.clone()],
            cover: None,
            page_limits: self.page_limits,
            export: self.export,
            mounting: self.mounting,
            look: self.look,
            next_id: self.next_id,
        }))
    }

    /// Devolve ao projeto o que uma edição fez na projeção: o conteúdo, as fotos e os ids (regra 4).
    pub fn absorb_cover(&mut self, mut canvas: Project) {
        // Só a própria capa volta: uma lâmina criada na projeção por engano não toma o lugar dela.
        let same =
            |cover: &Cover| canvas.spreads.len() == 1 && canvas.spreads[0].id == cover.content.id;
        if !self.cover.as_ref().is_some_and(same) {
            return;
        }
        let (Some(cover), Some(content)) = (self.cover.as_mut(), canvas.spreads.pop()) else {
            return;
        };
        cover.content = content;
        // O que vale para o álbum todo volta também; a especificação e as lâminas, nunca.
        self.name = canvas.name;
        self.gap = canvas.gap;
        self.photos = canvas.photos;
        self.page_limits = canvas.page_limits;
        self.export = canvas.export;
        self.mounting = canvas.mounting;
        self.look = canvas.look;
        self.next_id = self.next_id.max(canvas.next_id);
    }

    /// Cria a capa vazia, com o fundo do álbum (regra 6).
    pub fn create_cover(&mut self, measures: CoverMeasures) -> Result<(), CoreError> {
        measures.validate()?;
        let id = self.add_spread();
        let content = self.spreads.pop().filter(|s| s.id == id);
        let Some(content) = content else {
            return Err(CoreError::UnknownSpread(id));
        };
        self.cover = Some(Cover {
            measures,
            content,
            guide: None,
        });
        Ok(())
    }

    /// Refaz os quadros da capa depois de a medida mudar: em milímetros a partir do centro da
    /// lombada, que cresce para os dois lados (regra 7). `before` e `before_spine` são os de antes.
    pub fn refit_cover(&mut self, before: &CoverMeasures, before_spine: Mm) {
        let pages = self.page_count();
        let Some(cover) = self.cover.as_mut() else {
            return;
        };
        let now = cover.measures;
        let map = Refit {
            old_half: before_spine.get() / 2.0,
            new_half: now.spine_width(pages).get() / 2.0,
            old_size: (
                before.open_width_with(before_spine),
                before.face_height.get(),
            ),
            new_size: (now.open_width(pages).get(), now.face_height.get()),
        };
        for frame in &mut cover.content.frames {
            frame.rect = map.apply(frame.rect);
        }
    }
}

impl CoverMeasures {
    fn open_width_with(&self, spine: Mm) -> f64 {
        2.0 * self.face_width.get() + spine.get()
    }
}

/// A conta da regra 7: cada borda guarda a distância até a lombada, e o que fica dentro dela guarda
/// a distância até o centro.
struct Refit {
    old_half: f64,
    new_half: f64,
    old_size: (f64, f64),
    new_size: (f64, f64),
}

impl Refit {
    fn edge_x(&self, x: f64) -> f64 {
        let from_center = x - self.old_size.0 / 2.0;
        // Uma borda em cima da linha da lombada anda com a linha: é a foto que preenche a face.
        let moved = if from_center.abs() < self.old_half - 1e-6 {
            from_center
        } else {
            from_center.signum() * (from_center.abs() - self.old_half + self.new_half)
        };
        moved + self.new_size.0 / 2.0
    }

    fn apply(&self, rect: RelRect) -> RelRect {
        let (old_w, old_h) = self.old_size;
        let (new_w, new_h) = self.new_size;
        // Quem encosta no corte continua encostado: é o que leva a foto até a dobra.
        const AT_CUT: f64 = 1e-6;
        let (x0, x1) = (rect.x(), rect.x() + rect.width());
        let (y0, y1) = (rect.y(), rect.y() + rect.height());
        let left = if x0 < AT_CUT {
            0.0
        } else {
            self.edge_x(x0 * old_w).clamp(0.0, new_w)
        };
        let right = if x1 > 1.0 - AT_CUT {
            new_w
        } else {
            self.edge_x(x1 * old_w).clamp(0.0, new_w)
        };
        let shift_y = (new_h - old_h) / 2.0;
        let top = if y0 < AT_CUT {
            0.0
        } else {
            (y0 * old_h + shift_y).clamp(0.0, new_h)
        };
        let bottom = if y1 > 1.0 - AT_CUT {
            new_h
        } else {
            (y1 * old_h + shift_y).clamp(0.0, new_h)
        };
        // Um quadro que saiu inteiro da capa nova fica como estava em frações: nunca some.
        if right - left <= 0.0 || bottom - top <= 0.0 {
            return rect;
        }
        RelRect::new(
            left / new_w,
            top / new_h,
            (right - left) / new_w,
            (bottom - top) / new_h,
        )
        .unwrap_or(rect)
    }
}
