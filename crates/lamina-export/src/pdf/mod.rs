//! PDF de impressão (spec PD): um PDF/X-4 do álbum, uma página por lâmina (ou por página),
//! com a mesma imagem do JPEG de impressão, caixas de corte e de sangria, perfil de saída e,
//! se pedido, marcas de corte.

mod file;
mod meta;
pub(crate) mod profile;

use std::path::PathBuf;

use lamina_core::{Project, Rect as MmRect};
use pdf_writer::types::ColorSpaceOperand;
use pdf_writer::{Chunk, Content, Filter, Name, Rect, Ref};

use crate::jobs::SpreadJob;
use crate::write::{self, Ctx};
use crate::{ExportError, ExportOptions, jpeg};
use file::PdfFile;

/// Margem a mais de cada lado para as marcas de corte (regra 2), em mm.
const MARKS_MARGIN_MM: f64 = 10.0;
/// As marcas começam esta distância fora da sangria (regra 6), em mm.
const MARKS_GAP_MM: f64 = 3.0;
/// Espessura das marcas, em pontos.
const MARK_WIDTH_PT: f32 = 0.25;

const CATALOG: Ref = Ref::new(1);
const PAGES: Ref = Ref::new(2);
const INFO: Ref = Ref::new(3);
const METADATA: Ref = Ref::new(4);
const OUTPUT_ICC: Ref = Ref::new(5);
const IMAGE_ICC: Ref = Ref::new(6);
/// Primeiro número livre: cada página usa três (página, conteúdo, imagem).
const FIRST_PAGE: i32 = 7;

/// `casamento.pdf`. Se um álbum antigo já veio com caractere que não serve de arquivo (ex.: `:`),
/// limpa o nome do arquivo na exportação para não quebrar em exFAT e no Windows.
pub fn file_name(project: &Project) -> String {
    file_name_for(&project.name)
}

pub(crate) fn file_name_for(name: &str) -> String {
    if lamina_core::validate_project_name(name).is_ok() {
        return format!("{name}.pdf");
    }
    format!("{}.pdf", crate::clean::safe_file_stem(name, "album"))
}

/// Uma página pronta: a imagem em JPEG e onde fica o corte.
struct Sheet {
    jpeg: Vec<u8>,
    size_px: (u32, u32),
    /// Tamanho da página com a sangria, e o corte dentro dela (origem no canto de cima), em mm.
    canvas: MmRect,
    trim: MmRect,
}

/// Grava o PDF em `folder`, lâmina a lâmina, e devolve o caminho dele.
pub fn write(
    ctx: &Ctx,
    jobs: &[SpreadJob],
    options: &ExportOptions,
    progress: &dyn Fn(usize, usize),
) -> Result<PathBuf, ExportError> {
    let path = ctx.folder.join(file_name(ctx.project));
    let mut pdf = PdfFile::create(ctx.folder, &path)?;
    let mut chunk = Chunk::new();
    let output = profile::output_profile(ctx.space).map_err(ExportError::Encode)?;
    chunk.icc_profile(OUTPUT_ICC, output).n(3);
    pdf.object(OUTPUT_ICC, &chunk)?;
    let mut chunk = Chunk::new();
    let image = profile::image_profile(ctx.space).map_err(ExportError::Encode)?;
    chunk.icc_profile(IMAGE_ICC, image).n(3);
    pdf.object(IMAGE_ICC, &chunk)?;

    let total: usize = jobs.iter().map(|j| j.names().len()).sum();
    let mut kids = Vec::with_capacity(total);
    let mut failure = None;
    crate::in_order::in_order(
        jobs,
        crate::export::at_once(),
        &|job| sheets(ctx, job),
        &mut |result| {
            let written = result.and_then(|sheets| {
                for sheet in &sheets {
                    let first = FIRST_PAGE + 3 * i32::try_from(kids.len()).unwrap_or(i32::MAX / 4);
                    let ids = (Ref::new(first), Ref::new(first + 1), Ref::new(first + 2));
                    write_sheet(&mut pdf, sheet, ids, options.crop_marks)?;
                    kids.push(ids.0);
                    progress(kids.len(), total);
                }
                Ok(())
            });
            if let (None, Err(e)) = (&failure, written) {
                failure = Some(e);
            }
            failure.is_none()
        },
    );
    if let Some(e) = failure {
        return Err(e);
    }
    let mut chunk = Chunk::new();
    chunk
        .pages(PAGES)
        .kids(kids.iter().copied())
        .count(i32::try_from(kids.len()).unwrap_or(i32::MAX));
    pdf.object(PAGES, &chunk)?;
    let title = ctx.project.name.clone();
    let digest = pdf.digest();
    pdf.object(METADATA, &meta::metadata(METADATA, &title, &digest))?;
    pdf.object(INFO, &meta::info(INFO, &title))?;
    pdf.object(
        CATALOG,
        &meta::catalog(
            CATALOG,
            PAGES,
            METADATA,
            OUTPUT_ICC,
            profile::output_condition(ctx.space),
        ),
    )?;
    pdf.finish(CATALOG, INFO)
}

/// Renderiza a lâmina e codifica cada página dela em JPEG (a mesma imagem do JPEG exportado).
fn sheets(ctx: &Ctx, job: &SpreadJob) -> Result<Vec<Sheet>, ExportError> {
    let spec = &ctx.project.spec;
    let dpi = spec.dpi().get();
    write::images(ctx, job)?
        .into_iter()
        .zip(job.sides())
        .map(|(image, side)| {
            let size_px = image.dimensions();
            let (canvas, trim) = match side {
                None => (spec.canvas_rect(), spec.trim_rect()),
                Some(side) => (spec.page_canvas_rect(side), spec.page_rect(side)),
            };
            let jpeg =
                jpeg::encode(image, ctx.quality, dpi, ctx.space).map_err(ExportError::Encode)?;
            Ok(Sheet {
                jpeg,
                size_px,
                canvas,
                trim: relative(trim, canvas),
            })
        })
        .collect()
}

/// `inner` com a origem no canto de cima de `outer`.
fn relative(inner: MmRect, outer: MmRect) -> MmRect {
    MmRect::new(
        inner.x - outer.x,
        inner.y - outer.y,
        inner.width,
        inner.height,
    )
}

fn pt(mm: f64) -> f32 {
    // Medidas de página (no máximo alguns metros): cabem num f32 sem perda que importe.
    #[allow(clippy::cast_possible_truncation)]
    let v = (mm * 72.0 / 25.4) as f32;
    v
}

/// Página, conteúdo e imagem de uma folha. As medidas do PDF crescem de baixo para cima.
fn write_sheet(
    pdf: &mut PdfFile,
    sheet: &Sheet,
    (page, content, image): (Ref, Ref, Ref),
    marks: bool,
) -> Result<(), ExportError> {
    let margin = if marks { MARKS_MARGIN_MM } else { 0.0 };
    let (w, h) = (sheet.canvas.width.get(), sheet.canvas.height.get());
    let media = Rect::new(0.0, 0.0, pt(w + 2.0 * margin), pt(h + 2.0 * margin));
    let bleed = Rect::new(pt(margin), pt(margin), pt(margin + w), pt(margin + h));
    let t = sheet.trim;
    let trim = Rect::new(
        pt(margin + t.x.get()),
        pt(margin + h - t.y.get() - t.height.get()),
        pt(margin + t.x.get() + t.width.get()),
        pt(margin + h - t.y.get()),
    );

    let mut chunk = Chunk::new();
    let (wpx, hpx) = sheet.size_px;
    let mut xobject = chunk.image_xobject(image, &sheet.jpeg);
    xobject.filter(Filter::DctDecode);
    xobject.width(i32::try_from(wpx).unwrap_or(i32::MAX));
    xobject.height(i32::try_from(hpx).unwrap_or(i32::MAX));
    xobject.bits_per_component(8);
    xobject.color_space().icc_based(IMAGE_ICC);
    drop(xobject);
    pdf.object(image, &chunk)?;

    let mut c = Content::new();
    c.save_state();
    c.transform([
        bleed.x2 - bleed.x1,
        0.0,
        0.0,
        bleed.y2 - bleed.y1,
        bleed.x1,
        bleed.y1,
    ]);
    c.x_object(Name(b"Im0"));
    c.restore_state();
    if marks {
        draw_marks(&mut c, bleed, trim);
    }
    let mut chunk = Chunk::new();
    chunk.stream(content, &c.finish());
    pdf.object(content, &chunk)?;

    let mut chunk = Chunk::new();
    let mut p = chunk.page(page);
    p.parent(PAGES)
        .media_box(media)
        .bleed_box(bleed)
        .trim_box(trim)
        .contents(content);
    let mut resources = p.resources();
    resources.x_objects().pair(Name(b"Im0"), image);
    resources
        .color_spaces()
        .insert(Name(b"CS0"))
        .start::<pdf_writer::writers::ColorSpace>()
        .icc_based(IMAGE_ICC);
    drop(resources);
    drop(p);
    pdf.object(page, &chunk)
}

/// Marcas de corte (regra 6): nos quatro cantos, alinhadas ao corte, de 3 mm fora da sangria até
/// a borda da página, em preto (sRGB).
fn draw_marks(c: &mut Content, bleed: Rect, trim: Rect) {
    let gap = pt(MARKS_GAP_MM);
    c.set_stroke_color_space(ColorSpaceOperand::Named(Name(b"CS0")));
    c.set_stroke_color([0.0, 0.0, 0.0]);
    c.set_line_width(MARK_WIDTH_PT);
    let (left, right) = (bleed.x1 - gap, bleed.x2 + gap);
    let (bottom, top) = (bleed.y1 - gap, bleed.y2 + gap);
    let (max_x, max_y) = (bleed.x2 + bleed.x1, bleed.y2 + bleed.y1);
    for x in [trim.x1, trim.x2] {
        line(c, (x, bottom), (x, 0.0));
        line(c, (x, top), (x, max_y));
    }
    for y in [trim.y1, trim.y2] {
        line(c, (left, y), (0.0, y));
        line(c, (right, y), (max_x, y));
    }
}

fn line(c: &mut Content, from: (f32, f32), to: (f32, f32)) {
    c.move_to(from.0, from.1);
    c.line_to(to.0, to.1);
    c.stroke();
}
