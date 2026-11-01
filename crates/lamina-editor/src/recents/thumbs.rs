//! A miniatura do cartão e a conferência dos arquivos que sumiram (spec TI, regras 4 e 5). As duas
//! rodam fora da linha da janela: a janela chama e espera a resposta por mensagem.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use image::{Rgba, RgbaImage};
use lamina_core::{FrameContent, Photo, PhotoId, Project};
use lamina_render::{PhotoSource, RenderTarget};

use super::{BIG_THUMB_END, RESUME_SIDE};
use crate::Cache;
use crate::updates::news::{Illustration, illustration};

/// 320 px no lado maior.
pub const THUMB_SIDE: u32 = 320;

/// O cinza de uma foto que não abre, na miniatura do cartão (correção de 2026-10-27).
pub const MISSING_GRAY: [u8; 4] = [90, 90, 90, 255];

/// A fonte das fotos da miniatura: a foto que não abre vira um retângulo cinza do tamanho pedido,
/// e o resto da lâmina sai normal (regra 5).
struct OrGray<S>(S);

impl<S: PhotoSource> PhotoSource for OrGray<S> {
    fn load(&self, photo: &Photo, needed: (u32, u32)) -> Result<Arc<RgbaImage>, String> {
        self.0.load(photo, needed).or_else(|_| {
            let (w, h) = (needed.0.max(1), needed.1.max(1));
            Ok(Arc::new(RgbaImage::from_pixel(w, h, Rgba(MISSING_GRAY))))
        })
    }
}

/// A imagem do álbum é a capa aberta (spec DI, regra 16)? Só com algo nela e medidas que fecham:
/// a capa ainda vazia não mostraria nada, e vale a primeira lâmina.
pub fn shows_cover(project: &Project) -> bool {
    project.cover.as_ref().is_some_and(|cover| {
        !cover.content.frames.is_empty()
            && cover
                .measures
                .spec(&project.spec, project.page_count())
                .is_ok()
    })
}

/// A miniatura do cartão: a imagem do álbum com 320 px no lado maior.
pub fn thumbnail_image(project: &Project, cache: &Cache) -> Result<Option<RgbaImage>, String> {
    spread_image(project, cache, THUMB_SIDE)
}

/// Desenha a imagem do álbum com o renderizador da prévia (as miniaturas das fotos, do cache,
/// bastam nesse tamanho), com `side` px no lado maior: a capa aberta, se `shows_cover`, ou a
/// primeira lâmina, seja qual for a lâmina em que o fotógrafo parou. `None` para o álbum sem o que
/// mostrar (spec BR): o cartão mostra o desenho do álbum em branco. `Err` com o motivo.
pub fn spread_image(
    project: &Project,
    cache: &Cache,
    side: u32,
) -> Result<Option<RgbaImage>, String> {
    let canvas = if shows_cover(project) {
        project.cover_canvas().map_err(|e| e.to_string())?
    } else {
        None
    };
    first_spread_image(canvas.as_ref().unwrap_or(project), cache, side)
}

fn first_spread_image(
    project: &Project,
    cache: &Cache,
    side: u32,
) -> Result<Option<RgbaImage>, String> {
    if project.spreads.is_empty() {
        return Ok(None);
    }
    let thumbs: HashMap<PhotoId, PathBuf> = project
        .photos
        .iter()
        .filter_map(|p| cache.thumbnail(p).map(|path| (p.id, path)))
        .collect();
    let screens: HashMap<PhotoId, PathBuf> = project
        .spreads
        .first()
        .map_or(&[][..], |s| &s.frames)
        .iter()
        .filter_map(|f| match &f.content {
            FrameContent::Photo { photo, .. } => project.photo(*photo).ok(),
            _ => None,
        })
        .filter_map(|p| cache.screen(p).map(|path| (p.id, path)))
        .collect();
    let source = OrGray(lamina_media::cached_source(thumbs, screens));
    // O renderizador acha o tamanho pela `preview_size`: `side` px no lado maior.
    let target_size = RenderTarget::Preview { max_side: side };
    lamina_render::render_spread(project, 0, target_size, &source)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Grava em `target` a miniatura do álbum. Álbum sem o que mostrar não tem miniatura: a que sobrou
/// de antes é apagada, para o cartão não mostrar lâminas que não existem mais.
pub fn write_thumbnail(project: &Project, cache: &Cache, target: &Path) -> Result<(), String> {
    write_image(project, cache, target, THUMB_SIDE)
}

/// Grava a miniatura grande do álbum em andamento e apaga a do álbum anterior: só existe uma (spec
/// DI, regra 14).
pub fn write_big_thumbnail(project: &Project, cache: &Cache, target: &Path) -> Result<(), String> {
    write_image(project, cache, target, RESUME_SIDE)?;
    let Some(folder) = target.parent() else {
        return Ok(());
    };
    for entry in std::fs::read_dir(folder)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let path = entry.path();
        let big = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with(BIG_THUMB_END));
        if big && path != target {
            // Se não der para apagar agora, a próxima gravação tenta de novo.
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

fn write_image(project: &Project, cache: &Cache, target: &Path, side: u32) -> Result<(), String> {
    let Some(image) = spread_image(project, cache, side)? else {
        return match std::fs::remove_file(target) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    };
    let folder = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let tmp = tempfile::Builder::new()
        .suffix(".png")
        .tempfile_in(folder)
        .map_err(|e| e.to_string())?;
    image
        .save_with_format(tmp.path(), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    tmp.persist(target).map_err(|e| e.error.to_string())?;
    Ok(())
}

/// Lê a miniatura do disco: a janela não pode guardar a imagem só pelo caminho, senão o cartão
/// continua com a miniatura de antes depois de salvar. `None` sem miniatura.
pub fn read_thumbnail(path: &Path) -> Option<Illustration> {
    illustration(&std::fs::read(path).ok()?)
}

/// As miniaturas já lidas, pelo caminho e pela data do arquivo (regra 5, correção de 2026-10-27):
/// montar a tela de novo (cada tecla da busca) só relê o PNG que mudou no disco.
#[derive(Debug, Default)]
pub struct ThumbCache {
    seen: HashMap<PathBuf, (SystemTime, Option<Illustration>)>,
    /// Quantas vezes um PNG foi lido do disco (para os testes).
    pub reads: usize,
}

impl ThumbCache {
    /// A miniatura em `path`: a guardada, se a data do arquivo não mudou; senão, lida de novo.
    pub fn get(&mut self, path: &Path) -> Option<Illustration> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
        if let Some((when, picture)) = self.seen.get(path)
            && *when == modified
        {
            return picture.clone();
        }
        self.reads += 1;
        let picture = read_thumbnail(path);
        self.seen
            .insert(path.to_path_buf(), (modified, picture.clone()));
        picture
    }
}

/// Os caminhos que não existem mais. Só confere se o arquivo existe, sem ler (regra 4); pode
/// demorar num disco de rede, por isso a janela chama fora da linha dela.
pub fn missing_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| !p.is_file()).cloned().collect()
}
