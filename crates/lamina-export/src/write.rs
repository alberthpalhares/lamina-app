//! Renderizar uma lâmina, codificar e gravar com segurança (spec EX, regras 1, 2 e 6).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use image::RgbaImage;
use image::imageops::crop_imm;
use lamina_core::{ColorSpace, PageSide, Project};
use lamina_render::{Fill, PhotoSource, RenderTarget, render_spread_with};

use crate::jobs::{Output, SpreadJob};
use crate::once::ReadOnce;
use crate::{ExportError, ExportFormat, jpeg, tiff_out};

/// O que toda lâmina precisa para ser gravada.
pub struct Ctx<'a> {
    pub project: &'a Project,
    pub folder: &'a Path,
    pub photos: &'a dyn PhotoSource,
    pub quality: u8,
    /// Espaço de cor da saída (spec CT): a fonte de fotos já entrega as fotos nele.
    pub space: ColorSpace,
    /// JPEG ou TIFF (o PDF tem o próprio caminho).
    pub format: ExportFormat,
}

/// Renderiza a lâmina uma vez e grava os arquivos dela.
pub fn spread(ctx: &Ctx, job: &SpreadJob) -> Result<Vec<PathBuf>, ExportError> {
    let dpi = ctx.project.spec.dpi().get();
    images(ctx, job)?
        .into_iter()
        .zip(job.names())
        .map(|(image, name)| save(ctx, name, image, dpi))
        .collect()
}

/// As imagens de impressão da lâmina, uma por arquivo de saída e na mesma ordem: a lâmina
/// inteira, ou cada página com a própria sangria. O PDF usa as mesmas (spec PD, regra 3).
pub fn images(ctx: &Ctx, job: &SpreadJob) -> Result<Vec<RgbaImage>, ExportError> {
    match &job.output {
        Output::Pages(pages) if pages.is_empty() => Ok(Vec::new()),
        Output::Whole(_) => Ok(vec![render(ctx, job.spread, Fill::Pages)?]),
        Output::Pages(pages) => {
            // Páginas com fundos diferentes: cada uma é renderizada com o próprio fundo, para a
            // sangria da lombada ter a cor dela (spec AP, regra 3). Fundos iguais: uma vez só.
            let two = ctx.project.spreads.get(job.spread).is_some_and(|s| {
                s.page_background(PageSide::Left) != s.page_background(PageSide::Right)
            });
            let shared = if two {
                None
            } else {
                Some(render(ctx, job.spread, Fill::Pages)?)
            };
            pages
                .iter()
                .map(|(side, _)| match &shared {
                    Some(image) => Ok(page_of(ctx.project, image, *side)),
                    None => Ok(page_of(
                        ctx.project,
                        &render(ctx, job.spread, Fill::Side(*side))?,
                        *side,
                    )),
                })
                .collect()
        }
    }
}

/// Desenha a lâmina uma vez. A mesma foto em vários quadros é lida uma vez só, e cada original sai
/// da memória no último quadro; páginas com fundos diferentes leem de novo em cada desenho, para
/// não deixar todos os originais abertos entre um e outro (spec LV, regra 13).
fn render(ctx: &Ctx, spread: usize, fill: Fill) -> Result<RgbaImage, ExportError> {
    let frames = ctx
        .project
        .spreads
        .get(spread)
        .map_or(&[][..], |s| &s.frames);
    Ok(render_spread_with(
        ctx.project,
        spread,
        RenderTarget::Print,
        &ReadOnce::new(ctx.photos, frames),
        fill,
    )?)
}

fn save(ctx: &Ctx, name: &str, image: RgbaImage, dpi: u32) -> Result<PathBuf, ExportError> {
    let bytes = match ctx.format {
        ExportFormat::Tiff => {
            let profile = jpeg::embedded_profile(ctx.space).map_err(ExportError::Encode)?;
            tiff_out::encode(image, dpi, profile)
        }
        _ => jpeg::encode(image, ctx.quality, dpi, ctx.space),
    }
    .map_err(ExportError::Encode)?;
    let path = ctx.folder.join(name);
    write_atomic(ctx.folder, &path, &bytes)?;
    Ok(path)
}

/// A página com a própria sangria, recortada da imagem da lâmina. A largura sai da medida da
/// página (não da diferença de duas bordas arredondadas), então esquerda e direita têm sempre o
/// mesmo tamanho.
pub fn page_of(project: &Project, image: &RgbaImage, side: PageSide) -> RgbaImage {
    let canvas = project.spec.canvas_rect();
    let page = project.spec.page_canvas_rect(side);
    let scale = f64::from(image.width()) / canvas.width.get();
    let px = |mm: f64, max: u32| {
        // Preso entre 0 e o tamanho da imagem, então o cast não trunca.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (mm * scale).round().clamp(0.0, f64::from(max)) as u32;
        v
    };
    let (x, y) = (
        px(page.x.get(), image.width()),
        px(page.y.get(), image.height()),
    );
    let w = px(page.width.get(), image.width()).min(image.width().saturating_sub(x));
    let h = px(page.height.get(), image.height()).min(image.height().saturating_sub(y));
    crop_imm(image, x, y, w, h).to_image()
}

/// Grava num temporário de nome único na mesma pasta, força a ida ao disco e renomeia no fim.
/// Se algo falha, o temporário some sozinho.
pub(crate) fn write_atomic(folder: &Path, path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    let fail = |reason: String| ExportError::Write {
        path: path.display().to_string(),
        reason,
    };
    let mut tmp = tempfile::Builder::new()
        .prefix(".lamina-")
        .suffix(".tmp")
        .tempfile_in(folder)
        .map_err(|e| fail(e.to_string()))?;
    tmp.write_all(bytes).map_err(|e| fail(e.to_string()))?;
    tmp.as_file().sync_all().map_err(|e| fail(e.to_string()))?;
    tmp.persist(path).map_err(|e| fail(e.error.to_string()))?;
    Ok(())
}
