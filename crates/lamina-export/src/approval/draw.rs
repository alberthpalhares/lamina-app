//! O desenho das imagens para web (spec AC, regras 6, 7 e 9): o canvas numa escala tirada do corte, e
//! o recorte do que fica depois de cortar o álbum. O `RenderTarget::Preview` inclui a sangria, e por
//! isso o tamanho pedido a ele é o do canvas, não os 2048 px.

use image::RgbaImage;
use image::imageops::crop_imm;
use lamina_core::{PageSide, Project, Rect};
use lamina_render::{Fill, PhotoSource, RenderTarget, render_spread_with};

use crate::ExportError;

/// O maior lado do corte da lâmina inteira, em pixels (regra 6).
pub(crate) const WEB_SIDE: u32 = 2048;

/// Pixels por milímetro: os mesmos em todas as lâminas, inclusive nas de meia página.
fn web_scale(project: &Project) -> f64 {
    let trim = project.spec.trim_rect();
    f64::from(WEB_SIDE) / trim.width.get().max(trim.height.get())
}

/// Arredondado e preso entre 0 e o maior u32, então o cast não trunca.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn px(value: f64) -> u32 {
    value.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

/// O que aparece da lâmina `index` depois do corte: as duas páginas, a metade que existe numa ponta
/// de página simples, ou nada (uma lâmina só, com página simples nas duas pontas).
pub(crate) fn visible_region(project: &Project, index: usize) -> Option<Rect> {
    let spec = &project.spec;
    let active = spec.edges().active_pages(index, project.spreads.len());
    match (active.left, active.right) {
        (true, true) => Some(spec.trim_rect()),
        (true, false) => Some(spec.page_rect(PageSide::Left)),
        (false, true) => Some(spec.page_rect(PageSide::Right)),
        (false, false) => None,
    }
}

/// A lâmina `index` de `project` antes da faixa e da marca-d'água. A capa vem pela projeção: é a
/// lâmina 0 de `Project::cover_canvas`, cujo corte é a capa aberta sem a dobra.
pub fn web_image(
    project: &Project,
    index: usize,
    photos: &dyn PhotoSource,
) -> Result<RgbaImage, ExportError> {
    let spec = &project.spec;
    let scale = web_scale(project);
    let canvas = spec.canvas_rect();
    let max_side = px(canvas.width.get().max(canvas.height.get()) * scale);
    let target = RenderTarget::Preview { max_side };
    let drawn = render_spread_with(project, index, target, photos, Fill::Pages)?;
    let region = visible_region(project, index).unwrap_or_else(|| spec.trim_rect());
    let (x, y) = (
        px(region.x.get() * f64::from(drawn.width()) / canvas.width.get()),
        px(region.y.get() * f64::from(drawn.height()) / canvas.height.get()),
    );
    // A medida sai do corte, não do canvas arredondado: 600 mm dão sempre 2048 px.
    let width = px(region.width.get() * scale).min(drawn.width().saturating_sub(x));
    let height = px(region.height.get() * scale).min(drawn.height().saturating_sub(y));
    Ok(crop_imm(&drawn, x, y, width, height).to_image())
}
