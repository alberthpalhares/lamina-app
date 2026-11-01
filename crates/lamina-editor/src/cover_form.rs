//! A caixa "Medidas da capa" (spec CA, seção 3 e regras 17 e 18): os campos em texto, as três abas,
//! o resumo, os avisos e o desenho da capa aberta. Só contas; a janela mostra.

use std::path::{Path, PathBuf};

use image::RgbaImage;

use lamina_core::{
    CoreError, CoverGuide, CoverMeasures, CoverOrigin, MM_PER_INCH, Mm, Project, Spine,
};

/// As abas da caixa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoverTab {
    #[default]
    Album,
    Total,
    Guide,
}

/// O arquivo-guia lido: tamanho em pixels e o DPI, se o arquivo disser.
#[derive(Debug, Clone, PartialEq)]
pub struct GuideFile {
    pub path: PathBuf,
    pub size_px: (u32, u32),
    pub dpi: Option<f64>,
}

/// Um retângulo do desenho, em frações da largura e da altura da capa com a dobra.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DrawBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// O desenho ao vivo da caixa.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CoverDrawing {
    /// Largura ÷ altura da capa com a dobra.
    pub aspect: f64,
    pub back: DrawBox,
    pub spine: DrawBox,
    pub front: DrawBox,
}

/// Os campos da caixa, em centímetros (a espessura da folha, em milímetros).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoverForm {
    pub tab: CoverTab,
    pub face_width: String,
    pub face_height: String,
    /// `true` = lombada calculada pelo álbum; `false` = medida fixa em `spine`.
    pub spine_auto: bool,
    pub sheet_mm: String,
    pub spine: String,
    pub wrap: String,
    pub safe: String,
    pub total_width: String,
    pub total_height: String,
    pub includes_wrap: bool,
    pub guide: Option<GuideFile>,
    /// Quanto vale a largura total do guia, quando o arquivo não diz o DPI (regra 18).
    pub guide_width: String,
    /// As páginas do álbum, para a lombada calculada e o resumo.
    pages: usize,
    /// A lombada fixa que a capa já tem e as páginas de quando foi fixada: reconfirmar a caixa
    /// sem mudar a lombada não apaga o aviso do álbum que cresceu (regra 14).
    fixed: Option<(f64, usize)>,
    page_width: f64,
}

/// Lê o arquivo-guia: o tamanho em pixels e o DPI, sem decodificar a imagem inteira (regra 18).
pub fn guide_file(path: &Path) -> Result<GuideFile, String> {
    let known = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|e| matches!(e.as_str(), "jpg" | "jpeg" | "png" | "tif" | "tiff"));
    let wrong = || {
        "O guia precisa ser uma imagem JPEG, PNG ou TIFF. Exporte o guia do laboratório em um desses formatos.".to_owned()
    };
    if !known {
        return Err(wrong());
    }
    let size_px = image::image_dimensions(path).map_err(|_| wrong())?;
    Ok(GuideFile {
        path: path.to_path_buf(),
        size_px,
        dpi: lamina_media::image_dpi(path),
    })
}

/// O maior lado do guia na memória: basta para a tela (regra 18).
const GUIDE_SIDE: u32 = 1600;

/// O guia reduzido para a tela; `None` se o arquivo sumiu ou não abre.
pub fn guide_picture(path: &Path) -> Option<RgbaImage> {
    let image = image::open(path).ok()?;
    Some(image.thumbnail(GUIDE_SIDE, GUIDE_SIDE).to_rgba8())
}

/// A proporção (altura ÷ largura) do item da capa na faixa: da capa, ou da que seria criada.
pub fn cover_aspect(project: &Project) -> f64 {
    let measures = project
        .cover
        .as_ref()
        .map_or_else(|| CoverMeasures::defaults(&project.spec), |c| c.measures);
    let width = measures.open_width(project.page_count()).get() + 2.0 * measures.wrap.get();
    (measures.face_height.get() + 2.0 * measures.wrap.get()) / width
}

/// "30,3": o número da caixa, com uma casa quando precisa.
fn cm_text(mm: f64) -> String {
    let cm = (mm * 10.0).round() / 100.0;
    let text = format!("{cm:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.replace('.', ",")
}

/// Sempre com uma casa, para as frases: "62,1".
fn cm_line(mm: f64) -> String {
    format!("{:.1}", mm / 10.0).replace('.', ",")
}

/// O número digitado, com vírgula ou ponto.
fn number(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse::<f64>().ok()
}

fn cm(text: &str) -> Result<Mm, String> {
    number(text)
        .map(|v| Mm(v * 10.0))
        .ok_or_else(|| "Confira as medidas: use números, como 30,5.".to_owned())
}

/// A frase da seção 6 para cada medida fora dos limites.
fn invalid_text(error: &CoreError) -> String {
    let CoreError::InvalidCover { field, .. } = error else {
        return "Confira as medidas da capa.".to_owned();
    };
    match *field {
        "face" => "Confira as medidas: a face da capa vai de 5 a 100 cm.",
        "lombada" => "Confira as medidas: a lombada vai de 0 a 10 cm.",
        "dobra" => "Confira as medidas: a dobra vai de 0 a 10 cm.",
        "zona_segura" => "Confira as medidas: a zona segura precisa ser menor que metade da face.",
        "total" => "Confira as medidas: o total não comporta a lombada e as dobras.",
        _ => "Confira as medidas da capa.",
    }
    .to_owned()
}

impl CoverForm {
    /// A caixa de um álbum: com capa, na aba de origem e com o que foi digitado (CA-B27); sem capa,
    /// com as medidas já preenchidas (CA-B2).
    pub fn of(project: &Project) -> Self {
        let measures = project
            .cover
            .as_ref()
            .map_or_else(|| CoverMeasures::defaults(&project.spec), |c| c.measures);
        let pages = project.page_count();
        let sheet = match measures.spine {
            Spine::Auto { sheet } => sheet,
            Spine::Fixed { .. } => lamina_core::DEFAULT_SHEET,
        };
        let fixed = match (project.cover.is_some(), measures.spine) {
            (true, Spine::Fixed { mm, pages }) => Some((mm.get(), pages)),
            _ => None,
        };
        let mut form = CoverForm {
            fixed,
            tab: CoverTab::Album,
            face_width: cm_text(measures.face_width.get()),
            face_height: cm_text(measures.face_height.get()),
            spine_auto: matches!(measures.spine, Spine::Auto { .. }),
            sheet_mm: format!("{:.1}", sheet.get()).replace('.', ","),
            spine: cm_text(measures.spine_width(pages).get()),
            wrap: cm_text(measures.wrap.get()),
            safe: cm_text(measures.safe.get()),
            pages,
            page_width: project.spec.page_width().get(),
            ..CoverForm::default()
        };
        let open = measures.open_width(pages).get();
        form.total_width = cm_text(open);
        form.total_height = cm_text(measures.face_height.get());
        match measures.origin {
            CoverOrigin::Album => {}
            CoverOrigin::Total {
                width,
                height,
                includes_wrap,
            } => {
                form.tab = CoverTab::Total;
                form.total_width = cm_text(width.get());
                form.total_height = cm_text(height.get());
                form.includes_wrap = includes_wrap;
            }
            CoverOrigin::Guide => form.tab = CoverTab::Guide,
        }
        if let Some(guide) = project.cover.as_ref().and_then(|c| c.guide.as_ref()) {
            form.includes_wrap = guide.includes_wrap;
            form.guide = Some(GuideFile {
                path: guide.path.clone(),
                size_px: (0, 0),
                dpi: None,
            });
        }
        form
    }

    /// Escolhe o arquivo-guia: o total vem dos pixels e do DPI; sem DPI, a caixa pede a largura.
    pub fn set_guide(&mut self, file: GuideFile) {
        if let Some(dpi) = file.dpi.filter(|d| *d > 0.0) {
            let mm = |px: u32| f64::from(px) / dpi * MM_PER_INCH;
            self.total_width = cm_text(mm(file.size_px.0));
            self.total_height = cm_text(mm(file.size_px.1));
        }
        self.guide = Some(file);
    }

    /// Falta dizer a largura total do guia? (o arquivo não tem DPI)
    pub fn needs_guide_width(&self) -> bool {
        self.tab == CoverTab::Guide
            && self
                .guide
                .as_ref()
                .is_some_and(|g| g.dpi.is_none() && g.size_px.0 > 0)
    }

    /// Digitou a largura total do guia: a altura sai da proporção da imagem (regra 18).
    pub fn set_guide_width(&mut self, text: &str) {
        text.clone_into(&mut self.guide_width);
        let (Some(guide), Some(width)) = (self.guide.as_ref(), number(text)) else {
            return;
        };
        if guide.size_px.0 == 0 {
            return;
        }
        let ratio = f64::from(guide.size_px.1) / f64::from(guide.size_px.0);
        self.total_width = cm_text(width * 10.0);
        self.total_height = cm_text(width * 10.0 * ratio);
    }

    /// As medidas do que está na caixa, ou a frase da seção 6.
    pub fn measures(&self) -> Result<CoverMeasures, String> {
        let (wrap, safe) = (cm(&self.wrap)?, cm(&self.safe)?);
        let measures = match self.tab {
            CoverTab::Album => self.album_measures(wrap, safe)?,
            CoverTab::Total | CoverTab::Guide => {
                let total = (cm(&self.total_width)?, cm(&self.total_height)?);
                let spine = cm(&self.spine)?;
                let mut m = CoverMeasures::from_total(
                    total,
                    (spine, wrap, safe),
                    self.includes_wrap,
                    self.pages_for(spine),
                )
                .map_err(|e| invalid_text(&e))?;
                if self.tab == CoverTab::Guide {
                    m.origin = CoverOrigin::Guide;
                }
                m
            }
        };
        measures.validate().map_err(|e| invalid_text(&e))?;
        Ok(measures)
    }

    /// As páginas de uma lombada fixa: as guardadas, se a medida é a mesma; senão, as de agora.
    fn pages_for(&self, spine: Mm) -> usize {
        match self.fixed {
            Some((mm, pages)) if (mm - spine.get()).abs() < 0.05 => pages,
            _ => self.pages,
        }
    }

    fn album_measures(&self, wrap: Mm, safe: Mm) -> Result<CoverMeasures, String> {
        let spine = if self.spine_auto {
            let sheet = number(&self.sheet_mm)
                .ok_or_else(|| "Confira as medidas: use números, como 30,5.".to_owned())?;
            Spine::Auto { sheet: Mm(sheet) }
        } else {
            let mm = cm(&self.spine)?;
            Spine::Fixed {
                mm,
                pages: self.pages_for(mm),
            }
        };
        Ok(CoverMeasures {
            face_width: cm(&self.face_width)?,
            face_height: cm(&self.face_height)?,
            spine,
            wrap,
            safe,
            origin: CoverOrigin::Album,
        })
    }

    /// O guia a gravar na capa, se a aba é a do guia e há arquivo.
    pub fn guide_to_save(&self) -> Option<CoverGuide> {
        (self.tab == CoverTab::Guide)
            .then_some(self.guide.as_ref())
            .flatten()
            .map(|g| CoverGuide {
                path: g.path.clone(),
                includes_wrap: self.includes_wrap,
                visible: true,
            })
    }

    /// "Capa aberta: 62,1 × 30,6 cm, com lombada de 1,5 cm (15 folhas × 1,0 mm)".
    pub fn summary(&self) -> Option<String> {
        let m = self.measures().ok()?;
        let spine = m.spine_width(self.pages).get();
        let mut line = format!(
            "Capa aberta: {} × {} cm, com lombada de {} cm",
            cm_line(m.open_width(self.pages).get()),
            cm_line(m.face_height.get()),
            cm_line(spine)
        );
        if let Spine::Auto { sheet } = m.spine {
            let sheets = self.pages.div_ceil(2);
            let unit = if sheets == 1 { "folha" } else { "folhas" };
            let thick = format!("{:.1}", sheet.get()).replace('.', ",");
            line.push_str(&format!(" ({sheets} {unit} × {thick} mm)"));
        }
        Some(line)
    }

    /// Os avisos que não impedem criar: a face menor que a página e o guia de outra proporção.
    pub fn warnings(&self) -> Vec<String> {
        let Ok(m) = self.measures() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if m.face_width.get() < self.page_width {
            out.push(format!(
                "Capa: a face ({} cm) é menor que a página ({} cm).",
                cm_line(m.face_width.get()),
                cm_line(self.page_width)
            ));
        }
        if self.tab == CoverTab::Guide
            && let Some(guide) = self.guide.as_ref().filter(|g| g.size_px.1 > 0)
        {
            let fold = if self.includes_wrap {
                m.wrap.get()
            } else {
                0.0
            };
            let width = m.open_width(self.pages).get() + 2.0 * fold;
            let height = m.face_height.get() + 2.0 * fold;
            let cover = width / height;
            let image = f64::from(guide.size_px.0) / f64::from(guide.size_px.1);
            if ((image - cover) / cover).abs() > 0.01 {
                out.push("A proporção do guia é diferente da capa. Confira as medidas.".to_owned());
            }
        }
        out
    }

    /// O desenho ao vivo; sem medidas válidas, nenhum (a caixa mostra a frase do erro).
    pub fn drawing(&self) -> Option<CoverDrawing> {
        let m = self.measures().ok()?;
        let wrap = m.wrap.get();
        let spine = m.spine_width(self.pages).get();
        let face = m.face_width.get();
        let width = m.open_width(self.pages).get() + 2.0 * wrap;
        let height = m.face_height.get() + 2.0 * wrap;
        let part = |x: f64, w: f64| DrawBox {
            x: x / width,
            y: wrap / height,
            w: w / width,
            h: m.face_height.get() / height,
        };
        Some(CoverDrawing {
            aspect: width / height,
            back: part(wrap, face),
            spine: part(wrap + face, spine),
            front: part(wrap + face + spine, face),
        })
    }
}
