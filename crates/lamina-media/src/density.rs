//! O DPI gravado num arquivo de imagem, sem decodificar a imagem (spec CA, regra 18): o arquivo-guia
//! do laboratório diz o tamanho total por aqui.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use exif::{In, Tag, Value};

/// Polegadas por metro, para a densidade do PNG.
const INCHES_PER_METER: f64 = 39.3701;

/// O DPI de um JPEG (JFIF), PNG (`pHYs`) ou TIFF; `None` se o arquivo não diz.
pub fn image_dpi(path: &Path) -> Option<f64> {
    let mut head = [0u8; 256];
    let mut file = File::open(path).ok()?;
    let read = file.read(&mut head).ok()?;
    let head = &head[..read];
    if head.starts_with(&[0xFF, 0xD8]) {
        return jfif_dpi(head).or_else(|| exif_dpi(path));
    }
    if head.starts_with(b"\x89PNG") {
        return png_dpi(path);
    }
    exif_dpi(path)
}

fn jfif_dpi(head: &[u8]) -> Option<f64> {
    let at = head.windows(5).position(|w| w == b"JFIF\0")?;
    let unit = *head.get(at + 7)?;
    let x = u16::from_be_bytes([*head.get(at + 8)?, *head.get(at + 9)?]);
    match (unit, x) {
        (1, x) if x > 1 => Some(f64::from(x)),
        (2, x) if x > 1 => Some(f64::from(x) * 2.54),
        _ => None,
    }
}

fn png_dpi(path: &Path) -> Option<f64> {
    let mut reader = BufReader::new(File::open(path).ok()?);
    let mut signature = [0u8; 8];
    reader.read_exact(&mut signature).ok()?;
    loop {
        let mut header = [0u8; 8];
        reader.read_exact(&mut header).ok()?;
        let size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let kind = &header[4..8];
        if kind == b"IDAT" || kind == b"IEND" {
            return None;
        }
        if kind == b"pHYs" {
            let mut data = [0u8; 9];
            reader.read_exact(&mut data).ok()?;
            let per_meter = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            return (data[8] == 1 && per_meter > 0)
                .then(|| (f64::from(per_meter) / INCHES_PER_METER * 100.0).round() / 100.0);
        }
        std::io::copy(
            &mut (&mut reader).take(u64::from(size) + 4),
            &mut std::io::sink(),
        )
        .ok()?;
    }
}

fn exif_dpi(path: &Path) -> Option<f64> {
    let exif = exif::Reader::new()
        .read_from_container(&mut BufReader::new(File::open(path).ok()?))
        .ok()?;
    let field = exif.get_field(Tag::XResolution, In::PRIMARY)?;
    let Value::Rational(values) = &field.value else {
        return None;
    };
    let value = values.first()?.to_f64();
    let centimeters = exif
        .get_field(Tag::ResolutionUnit, In::PRIMARY)
        .and_then(|u| u.value.get_uint(0))
        == Some(3);
    let dpi = if centimeters { value * 2.54 } else { value };
    (dpi > 1.0).then_some(dpi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jfif_com_densidade_em_polegadas() {
        let mut head = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
        head.extend_from_slice(b"JFIF\0");
        head.extend_from_slice(&[1, 1, 1, 0x01, 0x2C, 0x01, 0x2C, 0, 0]);
        assert_eq!(jfif_dpi(&head), Some(300.0));
    }

    #[test]
    fn png_sem_densidade_nao_diz_dpi_e_com_densidade_diz() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("guia.png");
        image::RgbImage::new(4, 4).save(&path).unwrap();
        assert_eq!(image_dpi(&path), None);

        // O mesmo arquivo, com o pedaço pHYs de 300 DPI (11811 pontos por metro) depois do IHDR.
        let bytes = std::fs::read(&path).unwrap();
        let mut with = bytes[..33].to_vec();
        with.extend_from_slice(&9u32.to_be_bytes());
        with.extend_from_slice(b"pHYs");
        with.extend_from_slice(&11811u32.to_be_bytes());
        with.extend_from_slice(&11811u32.to_be_bytes());
        with.push(1);
        with.extend_from_slice(&[0, 0, 0, 0]);
        with.extend_from_slice(&bytes[33..]);
        let dense = dir.path().join("denso.png");
        std::fs::write(&dense, with).unwrap();
        let dpi = image_dpi(&dense).unwrap();
        assert!((dpi - 300.0).abs() < 0.1, "{dpi}");
    }

    #[test]
    fn arquivo_que_nao_existe_nao_tem_dpi() {
        assert_eq!(image_dpi(Path::new("nao-existe.tif")), None);
    }
}
