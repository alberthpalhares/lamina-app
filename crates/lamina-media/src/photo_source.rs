//! Fontes de fotos para a renderização (spec RD, seção 3): entregam os pixels de cada foto já
//! girados pelo EXIF. Moram aqui, e não na CLI, para a interface do Marco 2 usar as mesmas
//! (auditoria do Marco 1, M4).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use image::RgbaImage;
use lamina_core::{Photo, PhotoId};

use lamina_core::ColorSpace;

use crate::thumbnail::{SCREEN_SIZE, THUMB_SIZE};
use crate::{load_oriented, load_oriented_in, to_rgba};

/// Resultado de uma fonte de fotos: a imagem compartilhada, ou o motivo em texto.
pub type Loaded = Result<Arc<RgbaImage>, String>;

/// Sempre o arquivo original, em sRGB: para a impressão em sRGB.
pub fn original_source() -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    original_source_in(ColorSpace::Srgb)
}

/// Sempre o arquivo original, convertido para o espaço de cor da saída (spec CT, regra 2).
pub fn original_source_in(space: ColorSpace) -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    move |photo: &Photo, _: (u32, u32)| {
        load_oriented_in(&photo.path, space)
            .map(Arc::new)
            .map_err(|e| e.to_string())
    }
}

/// A miniatura do catálogo quando a foto inteira precisa de no máximo o tamanho dela (prévias);
/// senão, o arquivo original.
pub fn thumbnail_source(
    thumbs: HashMap<PhotoId, PathBuf>,
) -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    cached_source(thumbs, HashMap::new())
}

/// Como [`thumbnail_source`], tentando a imagem de tela antes do original quando a miniatura não
/// basta: um recorte fechado não abre a foto inteira (spec LV, regra 11).
pub fn cached_source(
    thumbs: HashMap<PhotoId, PathBuf>,
    screens: HashMap<PhotoId, PathBuf>,
) -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    move |photo: &Photo, needed: (u32, u32)| match read_copy(
        &thumbs,
        &screens,
        photo,
        needed,
        SCREEN_SIZE,
    ) {
        Some(copy) => Ok(copy),
        None => read_original(photo),
    }
}

/// Até quantos pixels a aprovação do cliente aceita a cópia de tela: 1,5 vez o tamanho dela. Acima
/// disso a foto sairia mole demais (spec AC, regra 17).
const APPROVAL_SCREEN_LIMIT: u32 = 2400;

/// A fonte da aprovação do cliente (spec AC, regras 17 e 18): a miniatura e a cópia de tela sempre
/// que bastam, e os originais um de cada vez, porque a prévia lê todos os quadros juntos.
pub fn approval_source(
    thumbs: HashMap<PhotoId, PathBuf>,
    screens: HashMap<PhotoId, PathBuf>,
) -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync {
    let originals = one_at_a_time(|photo: &Photo, _: (u32, u32)| read_original(photo));
    move |photo: &Photo, needed: (u32, u32)| match read_copy(
        &thumbs,
        &screens,
        photo,
        needed,
        APPROVAL_SCREEN_LIMIT,
    ) {
        Some(copy) => Ok(copy),
        None => originals(photo, needed),
    }
}

/// A mesma fonte, atendendo um pedido de cada vez (spec AC, regra 18).
pub fn one_at_a_time<F>(source: F) -> impl Fn(&Photo, (u32, u32)) -> Loaded + Sync
where
    F: Fn(&Photo, (u32, u32)) -> Loaded + Sync,
{
    let turn = Mutex::new(());
    move |photo: &Photo, needed: (u32, u32)| {
        let _turn = turn.lock().unwrap_or_else(PoisonError::into_inner);
        source(photo, needed)
    }
}

/// A miniatura ou a cópia de tela, se ela basta e pode ser lida.
fn read_copy(
    thumbs: &HashMap<PhotoId, PathBuf>,
    screens: &HashMap<PhotoId, PathBuf>,
    photo: &Photo,
    needed: (u32, u32),
    screen_limit: u32,
) -> Option<Arc<RgbaImage>> {
    let side = needed.0.max(needed.1);
    let copy = if side <= THUMB_SIZE {
        thumbs.get(&photo.id)
    } else if side <= screen_limit {
        screens.get(&photo.id)
    } else {
        None
    };
    let image = image::open(copy?).ok()?;
    Some(Arc::new(to_rgba(image)))
}

fn read_original(photo: &Photo) -> Loaded {
    load_oriented(&photo.path)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use lamina_core::{AlbumSpec, Dpi, Mm, Project};

    use super::*;

    /// Grava uma foto 60 × 40 e uma "miniatura" 30 × 20 de outra cor, e devolve o projeto.
    fn setup(dir: &std::path::Path) -> (Project, HashMap<PhotoId, PathBuf>) {
        let (photo, thumb) = (dir.join("foto.png"), dir.join("mini.png"));
        RgbImage::from_pixel(60, 40, Rgb([200, 0, 0]))
            .save(&photo)
            .unwrap();
        RgbImage::from_pixel(30, 20, Rgb([0, 0, 200]))
            .save(&thumb)
            .unwrap();
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        let id = project.add_photo(&photo, 60, 40);
        (project, HashMap::from([(id, thumb)]))
    }

    #[test]
    fn miniatura_so_quando_basta() {
        let dir = tempfile::tempdir().unwrap();
        let (project, thumbs) = setup(dir.path());
        let photo = &project.photos[0];
        let source = thumbnail_source(thumbs);
        assert_eq!(
            source(photo, (100, 80)).unwrap().dimensions(),
            (30, 20),
            "miniatura"
        );
        let big = source(photo, (1000, 800)).unwrap();
        assert_eq!(big.dimensions(), (60, 40), "original");
    }

    #[test]
    fn original_sempre_le_o_arquivo() {
        let dir = tempfile::tempdir().unwrap();
        let (project, _) = setup(dir.path());
        let image = original_source()(&project.photos[0], (1, 1)).unwrap();
        assert_eq!(image.dimensions(), (60, 40));
    }

    #[test]
    fn ac_b23_de_onde_vem_a_foto() {
        let dir = tempfile::tempdir().unwrap();
        let (project, thumbs) = setup(dir.path());
        let screen = dir.path().join("tela.png");
        RgbImage::from_pixel(45, 30, Rgb([0, 200, 0]))
            .save(&screen)
            .unwrap();
        let photo = &project.photos[0];
        let screens = HashMap::from([(photo.id, screen)]);
        let source = approval_source(thumbs, screens);
        let size = |side: u32| source(photo, (side, side / 2)).unwrap().dimensions();
        assert_eq!(size(320), (30, 20), "miniatura");
        assert_eq!(size(321), (45, 30), "cópia");
        assert_eq!(size(2400), (45, 30), "cópia");
        assert_eq!(size(2401), (60, 40), "original");
    }

    #[test]
    fn ac_b24_um_original_de_cada_vez() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (now, most) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let reader = |_: &Photo, _: (u32, u32)| {
            most.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
            now.fetch_sub(1, Ordering::SeqCst);
            Ok(Arc::new(RgbaImage::new(1, 1)))
        };
        let source = one_at_a_time(reader);
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_photo("a.jpg", 60, 40);
        let photo = &project.photos[0];
        std::thread::scope(|scope| {
            for _ in 0..6 {
                scope.spawn(|| source(photo, (1, 1)).unwrap());
            }
        });
        assert_eq!(most.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn arquivo_que_sumiu_vira_motivo_em_texto() {
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        project.add_photo("nao-existe.jpg", 60, 40);
        let err = original_source()(&project.photos[0], (1, 1)).unwrap_err();
        assert!(err.contains("nao-existe.jpg"), "{err}");
    }
}
