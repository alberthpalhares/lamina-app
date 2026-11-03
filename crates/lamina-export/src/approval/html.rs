//! O álbum animado em HTML (spec AH): um arquivo único e fechado. As imagens saem da imagem de 2048 px
//! do JPG, reduzidas e sem a faixa, e a página vem de `viewer.html`, sem biblioteca nenhuma.

use std::fmt::Write as _;

use base64::Engine;
use image::RgbaImage;
use image::imageops::{FilterType, crop_imm, resize};
use lamina_core::{Book, Face, PageSide, Project};
use serde_json::{Value, json};

use super::draw::{WEB_SIDE, px};
use super::plan::Job;
use super::{ApprovalHtml, compose, web_jpeg};
use crate::ExportError;

const VIEWER: &str = include_str!("viewer.html");
/// O maior lado do corte da lâmina inteira, no HTML (regra 3).
const HTML_SIDE: u32 = 1600;
const FILE_SUFFIX: &str = "-aprovacao.html";
/// Sem nome que sirva de arquivo, o HTML se chama assim; o cliente continua vendo o nome digitado.
const FILE_FALLBACK: &str = "album";
/// Nomes que o Windows reserva, com ou sem extensão.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Uma imagem do HTML, já codificada.
pub(super) struct Image {
    id: &'static str,
    number: Option<usize>,
    alt: String,
    size: (u32, u32),
    jpeg: Vec<u8>,
}

/// As imagens que saem de uma imagem desenhada: a lâmina inteira, ou as duas faces da capa. A
/// marca-d'água entra depois da redução (regra 3).
pub(super) fn images_of(
    project: &Project,
    job: &Job,
    clean: &RgbaImage,
    mark: Option<&str>,
) -> Result<Vec<Image>, ExportError> {
    let Some(number) = job.number else {
        return cover_images(project, clean, mark);
    };
    let (width, height) = clean.dimensions();
    let size = (reduced(width), reduced(height));
    let small = resize(clean, size.0, size.1, FilterType::CatmullRom);
    Ok(vec![finish(
        "lamina",
        Some(number),
        format!("Lâmina {number}"),
        small,
        mark,
    )?])
}

fn reduced(side: u32) -> u32 {
    (side * HTML_SIDE + WEB_SIDE / 2) / WEB_SIDE
}

/// A frente e o verso, recortados do corte da capa, na mesma escala das lâminas (regra 4).
fn cover_images(
    project: &Project,
    clean: &RgbaImage,
    mark: Option<&str>,
) -> Result<Vec<Image>, ExportError> {
    let cover = Book::from_project(project).cover;
    let trim = project.spec.trim_rect();
    let album_scale = f64::from(HTML_SIDE) / trim.width.get().max(trim.height.get());
    let size = (
        px(cover.face_width.get() * album_scale),
        px(cover.face_height.get() * album_scale),
    );
    let open = cover.front.right().get();
    let scale = f64::from(clean.width()) / open;
    let face = |rect: lamina_core::Rect| {
        let x = px(rect.x.get() * scale).min(clean.width());
        let width = px(rect.width.get() * scale).min(clean.width() - x);
        let cut = crop_imm(clean, x, 0, width, clean.height()).to_image();
        resize(&cut, size.0, size.1, FilterType::CatmullRom)
    };
    Ok(vec![
        finish(
            "capa-frente",
            None,
            "Capa".to_owned(),
            face(cover.front),
            mark,
        )?,
        finish(
            "capa-verso",
            None,
            "Contracapa".to_owned(),
            face(cover.back),
            mark,
        )?,
    ])
}

fn finish(
    id: &'static str,
    number: Option<usize>,
    alt: String,
    image: RgbaImage,
    mark: Option<&str>,
) -> Result<Image, ExportError> {
    let size = image.dimensions();
    let jpeg = web_jpeg(compose::compose(image, None, mark))?;
    Ok(Image {
        id,
        number,
        alt,
        size,
        jpeg,
    })
}

/// A página inteira. A contracapa vai por último na lista, depois das lâminas (regra 2).
pub(super) fn page(
    project: &Project,
    html: &ApprovalHtml,
    code: &str,
    mut images: Vec<Image>,
) -> Result<String, ExportError> {
    images.sort_by_key(|image| match (image.id, image.number) {
        ("capa-frente", _) => 0,
        (_, Some(number)) => number,
        _ => usize::MAX,
    });
    let data = data_json(project, html, code)?;
    let name = escaped(&html.name);
    let links = Links::new(html, code);
    Ok(fill(
        VIEWER,
        &[
            ("__TITULO__", name.clone()),
            ("__NOME__", name),
            ("__CODIGO__", escaped(code)),
            ("__FIGURAS__", figures(&images)),
            ("__WA_APROVAR__", links.approve),
            ("__WA_PEDIR__", links.ask),
            ("__MAIL_PEDIR__", links.mail),
            ("__DADOS__", data),
        ],
    ))
}

/// Troca os marcadores de uma passada só: um nome escrito `__DADOS__` continua sendo só um nome.
fn fill(template: &str, values: &[(&str, String)]) -> String {
    let total: usize = values.iter().map(|(_, value)| value.len()).sum();
    let mut out = String::with_capacity(template.len() + total);
    let mut rest = template;
    loop {
        let next = values
            .iter()
            .filter_map(|(key, value)| rest.find(key).map(|at| (at, *key, value)))
            .min_by_key(|(at, _, _)| *at);
        let Some((at, key, value)) = next else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..at]);
        out.push_str(value);
        rest = &rest[at + key.len()..];
    }
}

fn figures(images: &[Image]) -> String {
    let mut out = String::new();
    let encoder = base64::engine::general_purpose::STANDARD;
    for image in images {
        let id = match image.number {
            Some(number) => format!("{}-{number}", image.id),
            None => image.id.to_owned(),
        };
        let _ = writeln!(
            out,
            "  <figure><img id=\"{id}\" alt=\"{alt}\" width=\"{w}\" height=\"{h}\" loading=\"lazy\" \
             decoding=\"async\" src=\"data:image/jpeg;base64,{data}\"><figcaption>{alt}</figcaption></figure>",
            alt = image.alt,
            w = image.size.0,
            h = image.size.1,
            data = encoder.encode(&image.jpeg),
        );
    }
    out
}

fn face_json(face: Face) -> Value {
    match face {
        Face::Blank => json!("neutra"),
        Face::Spread { index, side } => json!({
            "lamina": index + 1,
            "lado": match side {
                PageSide::Left => "esquerda",
                PageSide::Right => "direita",
            },
        }),
    }
}

/// Os dados do livro (regra 6), sem nada que feche o bloco: um `<` só entra como `<`.
fn data_json(project: &Project, html: &ApprovalHtml, code: &str) -> Result<String, ExportError> {
    let book = Book::from_project(project);
    let leaves: Vec<Value> = book
        .leaves
        .iter()
        .map(|leaf| json!({"frente": face_json(leaf.front), "verso": face_json(leaf.back)}))
        .collect();
    let data = json!({
        "nome": html.name,
        "codigo": code,
        "laminas": project.spreads.len(),
        "capa_lisa": book.cover.neutral,
        "guardas": {"inicio": face_json(book.inside_front), "fim": face_json(book.inside_back)},
        "folhas": leaves,
        "medidas": {
            "face": [book.cover.face_width.get(), book.cover.face_height.get()],
            "pagina": [book.page_width.get(), book.page_height.get()],
        },
        "whatsapp": html.whatsapp,
        "email": html.email,
    });
    let text = serde_json::to_string(&data).map_err(|e| ExportError::Encode(e.to_string()))?;
    Ok(text.replace('<', "\\u003c"))
}

/// Os três links fixos da lista, que funcionam sem script (regra 11).
struct Links {
    approve: String,
    ask: String,
    mail: String,
}

impl Links {
    fn new(html: &ApprovalHtml, code: &str) -> Links {
        let name = &html.name;
        let approve = format!("Aprovo o álbum \"{name}\" (prova {code}), sem alterações.");
        let ask = format!("Revisei o álbum \"{name}\" (prova {code}). Peço alteração em:");
        let subject = format!("Álbum {name}, prova {code}");
        let phone = html.whatsapp.as_deref().unwrap_or_default();
        let email = escaped(html.email.as_deref().unwrap_or_default());
        Links {
            approve: format!("https://wa.me/{phone}?text={}", percent(&approve)),
            ask: format!("https://wa.me/{phone}?text={}", percent(&ask)),
            mail: format!(
                "mailto:{email}?subject={}&amp;body={}",
                percent(&subject),
                percent(&ask)
            ),
        }
    }
}

/// UTF-8 com percent-encoding: só letras, dígitos e `-_.~` passam.
fn percent(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// O nome do arquivo (seção 6): o que o sistema recusa vira espaço, sem espaços repetidos nem espaço
/// ou ponto nas pontas.
pub fn html_file_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let words: Vec<&str> = clean.split_whitespace().collect();
    let joined = words.join(" ");
    let joined = joined.trim_end_matches('.');
    let stem = joined.split('.').next().unwrap_or_default();
    let reserved = RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem));
    if joined.is_empty() || reserved {
        return format!("{FILE_FALLBACK}{FILE_SUFFIX}");
    }
    format!("{joined}{FILE_SUFFIX}")
}
