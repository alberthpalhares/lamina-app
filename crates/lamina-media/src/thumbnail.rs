//! Miniaturas das fotos, já na orientação certa, guardadas como JPEG no cache.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use tempfile::NamedTempFile;

use crate::MediaError;
use crate::derive::{derive_file, open, read_pixels, reduced, write_derived};

/// Lado maior das miniaturas, em pixels: nítido na lista e na visão geral das lâminas.
pub const THUMB_SIZE: u32 = 320;

/// Decodifica a foto, reduz para caber em `max_side`, converte para sRGB e aplica a rotação EXIF
/// (reduzir primeiro: spec TR, regra 3).
pub fn make_thumbnail(
    path: &Path,
    exif_orientation: u8,
    max_side: u32,
) -> Result<DynamicImage, MediaError> {
    let bytes = std::fs::read(path).map_err(MediaError::io(path))?;
    let mut decoder = ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(MediaError::io(path))?
        .into_decoder()
        .map_err(MediaError::image(path))?;
    let icc = decoder.icc_profile().ok().flatten();
    drop(decoder);
    let full = read_pixels(path, bytes, max_side)?;
    Ok(reduced(full, icc.as_deref(), exif_orientation, max_side))
}

/// Lado maior da imagem de tela: a lâmina grande na janela (spec JP, regra 6).
pub const SCREEN_SIZE: u32 = 1600;
/// Qualidade do JPEG da imagem de tela: só para mostrar, nunca para imprimir.
const SCREEN_QUALITY: u8 = 90;

/// Pastas do cache onde ficam as cópias reduzidas de cada foto.
#[derive(Debug, Clone, Copy)]
pub struct DerivedDirs<'a> {
    pub thumbs: &'a Path,
    /// Imagens de tela; `None` quando só a miniatura interessa (linha de comando).
    pub screens: Option<&'a Path>,
}

/// Versão das cópias reduzidas: a 2 converte as fotos com perfil próprio para sRGB (spec CT,
/// regra 10); a 3 sai do JPEG reduzido na leitura (spec LV, regra 9). As cópias de outra versão são
/// ignoradas e refeitas.
const DERIVED_VERSION: u32 = 3;

/// Arquivo da cópia de uma foto (miniatura ou imagem de tela) dentro de `dir`.
pub fn derived_path(dir: &Path, fingerprint: &str) -> PathBuf {
    dir.join(format!("{fingerprint}.v{DERIVED_VERSION}.jpg"))
}

/// Gera (se ainda não existir) a miniatura em `dir/<impressão digital>.jpg` e devolve o caminho.
pub fn ensure_thumbnail(
    path: &Path,
    fingerprint: &str,
    exif_orientation: u8,
    dir: &Path,
) -> Result<PathBuf, MediaError> {
    let dirs = DerivedDirs {
        thumbs: dir,
        screens: None,
    };
    ensure_derived(path, fingerprint, exif_orientation, dirs)
}

/// Gera o que faltar entre a miniatura e a imagem de tela, lendo e decodificando a foto uma vez
/// só, e devolve o caminho da miniatura. `fingerprint` e `exif_orientation` são os do catálogo.
pub fn ensure_derived(
    path: &Path,
    fingerprint: &str,
    exif_orientation: u8,
    dirs: DerivedDirs<'_>,
) -> Result<PathBuf, MediaError> {
    let thumb = derived_path(dirs.thumbs, fingerprint);
    let screen_missing = dirs
        .screens
        .is_some_and(|d| !derived_path(d, fingerprint).exists());
    if thumb.exists() && !screen_missing {
        return Ok(thumb);
    }
    let bytes = std::fs::read(path).map_err(MediaError::io(path))?;
    let mut opened = open(path, &bytes)?;
    fingerprint.clone_into(&mut opened.fingerprint);
    opened.metadata.exif_orientation = exif_orientation;
    write_derived(path, bytes, &opened, dirs)
}

/// Como [`ensure_derived`] para uma foto do projeto: a orientação vem do EXIF e a impressão
/// digital, se o projeto não tiver, do próprio arquivo, sem lê-lo de novo (spec TR, regra 2).
pub fn ensure_derived_for(
    path: &Path,
    fingerprint: Option<&str>,
    dirs: DerivedDirs<'_>,
) -> Result<PathBuf, MediaError> {
    if let Some(fp) = fingerprint {
        let thumb = derived_path(dirs.thumbs, fp);
        let screen_ok = dirs.screens.is_none_or(|d| derived_path(d, fp).exists());
        if thumb.exists() && screen_ok {
            return Ok(thumb);
        }
    }
    derive_file(path, fingerprint, dirs)
}

/// Grava em `target` de uma vez: o arquivo só aparece inteiro. Um JPEG cortado abre sem erro, com o
/// resto em cinza, e quem lê as imagens só confere se o arquivo existe (a faixa cinza nas lâminas
/// vizinhas). Dois gravando a mesma foto: fica o primeiro que terminar, e os dois são iguais.
fn write_whole(
    target: &Path,
    encode: impl FnOnce(&mut BufWriter<&mut File>) -> image::ImageResult<()>,
) -> Result<(), MediaError> {
    let dir = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(MediaError::io(dir))?;
    let mut file = NamedTempFile::new_in(dir).map_err(MediaError::io(dir))?;
    let mut writer = BufWriter::new(file.as_file_mut());
    encode(&mut writer).map_err(MediaError::image(target))?;
    writer.flush().map_err(MediaError::io(target))?;
    drop(writer);
    match file.persist(target) {
        Ok(_) => Ok(()),
        Err(_) if target.exists() => Ok(()),
        Err(failure) => Err(MediaError::io(target)(failure.error)),
    }
}

/// Grava a miniatura (JPEG na qualidade padrão do codificador, como sempre).
pub(crate) fn save_thumb(image: &DynamicImage, target: &Path) -> Result<(), MediaError> {
    write_whole(target, |writer| {
        image.to_rgb8().write_to(writer, ImageFormat::Jpeg)
    })
}

/// Grava a imagem de tela (até [`SCREEN_SIZE`] no lado maior; foto menor fica do tamanho dela).
pub(crate) fn save_screen(image: &DynamicImage, target: &Path) -> Result<(), MediaError> {
    write_whole(target, |writer| {
        let encoder = JpegEncoder::new_with_quality(writer, SCREEN_QUALITY);
        image.to_rgb8().write_with_encoder(encoder)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::write_jpeg;

    /// Uma foto de ruído: difícil de comprimir, para a gravação levar vários blocos.
    fn noisy(width: u32, height: u32) -> DynamicImage {
        let mut state = 0x02f6_e2b1_u32;
        let image = image::RgbImage::from_fn(width, height, |_, _| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let [r, g, b, _] = state.to_le_bytes();
            image::Rgb([r, g, b])
        });
        DynamicImage::ImageRgb8(image)
    }

    /// Lê `target` sem parar enquanto `save` grava, e conta as leituras que acharam o arquivo
    /// pela metade (um JPEG termina em `FF D9`).
    fn partial_reads(target: &Path, save: impl FnOnce()) -> usize {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (flag, path) = (stop.clone(), target.to_path_buf());
        let reader = std::thread::spawn(move || {
            let mut partial = 0;
            while !flag.load(std::sync::atomic::Ordering::SeqCst) {
                if let Ok(bytes) = std::fs::read(&path)
                    && !bytes.ends_with(&[0xFF, 0xD9])
                {
                    partial += 1;
                }
            }
            partial
        });
        save();
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        reader.join().unwrap()
    }

    #[test]
    fn a_imagem_de_tela_nunca_aparece_pela_metade() {
        let dir = tempfile::tempdir().unwrap();
        let image = noisy(2400, 1600);
        for i in 0..3 {
            let target = dir.path().join(format!("telas/{i}.jpg"));
            let partial = partial_reads(&target, || save_screen(&image, &target).unwrap());
            assert_eq!(partial, 0, "leu o arquivo pela metade {partial} vezes");
            assert_eq!(image::open(&target).unwrap().width(), 2400);
        }
        let sobras = std::fs::read_dir(dir.path().join("telas")).unwrap().count();
        assert_eq!(sobras, 3, "ficou arquivo temporário na pasta");
    }

    #[test]
    fn a_miniatura_nunca_aparece_pela_metade() {
        let dir = tempfile::tempdir().unwrap();
        let image = noisy(2400, 1600);
        let target = dir.path().join("a.jpg");
        let partial = partial_reads(&target, || save_thumb(&image, &target).unwrap());
        assert_eq!(partial, 0, "leu o arquivo pela metade {partial} vezes");
    }

    #[test]
    fn miniatura_cabe_no_tamanho_maximo_e_mantem_a_proporcao() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 900, 600, None);
        let thumb = make_thumbnail(&path, 1, 300).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (300, 200));
    }

    #[test]
    fn rotacao_exif_e_aplicada() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 900, 600, None);
        let thumb = make_thumbnail(&path, 6, 300).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (200, 300));
    }

    #[test]
    fn miniatura_existente_nao_e_refeita() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 90, 60, None);
        let cache = dir.path().join("cache");
        let first = ensure_thumbnail(&path, "abc", 1, &cache).unwrap();
        let modified = std::fs::metadata(&first).unwrap().modified().unwrap();
        let second = ensure_thumbnail(&path, "abc", 1, &cache).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            std::fs::metadata(&second).unwrap().modified().unwrap(),
            modified
        );
    }

    #[test]
    fn imagem_de_tela_tem_1600_no_lado_maior_e_sai_com_a_miniatura() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 3000, 2000, None);
        let (thumbs, screens) = (dir.path().join("m"), dir.path().join("t"));
        let dirs = DerivedDirs {
            thumbs: &thumbs,
            screens: Some(&screens),
        };
        let thumb = ensure_derived(&path, "abc", 6, dirs).unwrap();
        let screen = image::open(derived_path(&screens, "abc")).unwrap();
        assert_eq!((screen.width(), screen.height()), (1067, 1600));
        assert_eq!(image::open(thumb).unwrap().height(), THUMB_SIZE);
    }

    #[test]
    fn foto_pequena_nao_e_ampliada_na_imagem_de_tela() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_jpeg(dir.path(), "a.jpg", 900, 600, None);
        let screens = dir.path().join("t");
        let dirs = DerivedDirs {
            thumbs: dir.path(),
            screens: Some(&screens),
        };
        ensure_derived(&path, "abc", 1, dirs).unwrap();
        let screen = image::open(derived_path(&screens, "abc")).unwrap();
        assert_eq!((screen.width(), screen.height()), (900, 600));
    }
}
