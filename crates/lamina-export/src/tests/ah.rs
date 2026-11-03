//! Cenários da spec `aprovacao-cliente-html` (AH), o arquivo: fechado, sem nada privado, lista sem
//! script, imagens, dados do livro, contato e mesmos bytes. O resto está em `ah_more.rs`.

use base64::Engine;
use lamina_core::{AlbumEdge, Book, Face, PageSide};
use serde_json::{Value, json};

use super::ac::{generate, with_cover};
use super::ex::{Album, solid, spec};
use crate::{ApprovalHtml, ApprovalOptions};

/// O script e a regra de segurança são iguais em todo arquivo; os dados mudam.
pub fn html_only(name: &str) -> ApprovalOptions {
    ApprovalOptions {
        jpg: false,
        html: Some(ApprovalHtml {
            name: name.to_owned(),
            whatsapp: None,
            email: None,
        }),
        ..ApprovalOptions::default()
    }
}

pub fn page(album: &Album, options: &ApprovalOptions) -> String {
    let dir = tempfile::tempdir().unwrap();
    let report = generate(album, dir.path(), options).unwrap();
    let file = report.files.last().unwrap();
    std::fs::read_to_string(file).unwrap()
}

/// A página sem o conteúdo das imagens: base64 pode conter qualquer palavra por acaso.
pub fn without_data(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = rest.find("data:image/jpeg;base64,") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find('"').unwrap();
        out.push_str("DADOS");
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn without_scripts(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = rest.find("<script") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find("</script>").unwrap();
        rest = &tail[end + "</script>".len()..];
    }
    out.push_str(rest);
    out
}

/// O JSON da regra 6.
fn data_of(html: &str) -> Value {
    let open = "<script type=\"application/json\" id=\"dados\">";
    let at = html.find(open).unwrap() + open.len();
    let end = at + html[at..].find("</script>").unwrap();
    serde_json::from_str(&html[at..end]).unwrap()
}

/// As medidas da imagem de `id`, decodificada do arquivo.
fn size_of(html: &str, id: &str) -> (u32, u32) {
    let at = html.find(&format!("id=\"{id}\"")).unwrap();
    let tail = &html[at..];
    let prefix = "data:image/jpeg;base64,";
    let start = tail.find(prefix).unwrap() + prefix.len();
    let end = start + tail[start..].find('"').unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&tail[start..end])
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap();
    (image.width(), image.height())
}

fn three() -> Album {
    let mut album = Album::new(spec(), 3);
    let photo = album.photo("f.jpg", solid(1600, 1000, [120, 90, 60]));
    album.photo_frame(0, photo, [0.1, 0.1, 0.5, 0.6]);
    album
}

#[test]
fn ah_b1_arquivo_fechado() {
    let mut album = three();
    with_cover(&mut album);
    let mut options = html_only("Ana");
    if let Some(html) = &mut options.html {
        html.whatsapp = Some("5511999999999".to_owned());
        html.email = Some("estudio@exemplo.com".to_owned());
    }
    let html = without_data(&page(&album, &options));
    assert!(html.contains(
        "default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'"
    ));
    for word in ["atob", "Blob", "fetch", "XMLHttpRequest"] {
        assert!(!html.contains(word), "{word}");
    }
    // Todo endereço é o do WhatsApp; o e-mail vai por `mailto:`, que não abre nada sozinho.
    let mut rest = html.as_str();
    while let Some(at) = rest.find("://") {
        assert!(rest[..at].ends_with("https"), "{}", &rest[at - 10..at + 10]);
        assert!(
            rest[at..].starts_with("://wa.me/"),
            "{}",
            &rest[at..at + 20]
        );
        rest = &rest[at + 3..];
    }
    assert!(!html.contains("src=\"http"));
}

#[test]
fn ah_b2_nada_de_privado_no_arquivo() {
    let mut album = Album::new(spec(), 1);
    album.project.name = "Ana Silva 2026".to_owned();
    let photo = album.photo(
        "C:\\Clientes\\Ana\\IMG_1234.jpg",
        solid(600, 400, [10, 20, 30]),
    );
    album.project.photos[0].camera = Some("Camera Secreta X".to_owned());
    album.photo_frame(0, photo, [0.1, 0.1, 0.5, 0.6]);
    let html = without_data(&page(&album, &html_only("Ana")));
    for word in ["Clientes", "IMG_1234", "Camera Secreta", "Ana Silva 2026"] {
        assert!(!html.contains(word), "{word}");
    }
    assert!(html.contains("Ana"));
}

#[test]
fn ah_b3_lista_sem_script() {
    let mut album = three();
    with_cover(&mut album);
    let html = without_scripts(&without_data(&page(&album, &html_only("Ana"))));
    for (id, alt) in [
        ("capa-frente", "Capa"),
        ("lamina-1", "Lâmina 1"),
        ("lamina-2", "Lâmina 2"),
        ("lamina-3", "Lâmina 3"),
        ("capa-verso", "Contracapa"),
    ] {
        assert!(html.contains(&format!("id=\"{id}\" alt=\"{alt}\"")), "{id}");
        assert!(
            html.contains(&format!("<figcaption>{alt}</figcaption>")),
            "{alt}"
        );
    }
    let first = html.find("capa-frente").unwrap();
    let last = html.find("capa-verso").unwrap();
    assert!(first < html.find("lamina-1").unwrap());
    assert!(
        last > html.find("lamina-3").unwrap(),
        "a contracapa por último"
    );
    assert!(html.contains("Aprovar pelo WhatsApp"));
    assert!(html.contains("Pedir alterações pelo WhatsApp"));
    assert!(html.contains("Responder por e-mail"));
    let approve = "Aprovo%20o%20%C3%A1lbum%20%22Ana%22%20%28prova%20";
    assert!(
        html.contains(&format!("https://wa.me/?text={approve}")),
        "aprovar"
    );
    let ask = "Revisei%20o%20%C3%A1lbum%20%22Ana%22";
    assert!(
        html.contains(&format!("https://wa.me/?text={ask}")),
        "pedir"
    );
    assert!(html.contains("&amp;body="), "o & escrito no HTML");
}

#[test]
fn ah_b4_tamanho_das_imagens() {
    let mut album = three();
    with_cover(&mut album);
    let html = page(&album, &html_only("Ana"));
    assert_eq!(size_of(&html, "lamina-1"), (1600, 800));
    assert_eq!(size_of(&html, "lamina-3"), (1600, 800));
    assert_eq!(size_of(&html, "capa-frente"), (808, 816));
    assert_eq!(size_of(&html, "capa-verso"), (808, 816));
    let images = html.matches("<img ").count();
    assert_eq!(images, 5);
    assert_eq!(html.matches("loading=\"lazy\"").count(), images);
    assert_eq!(html.matches("decoding=\"async\"").count(), images);
}

#[test]
fn ah_b5_pontas_de_pagina_simples() {
    let edges = spec().with_edges(AlbumEdge::SinglePage, AlbumEdge::SinglePage);
    let album = Album::new(edges, 3);
    let html = page(&album, &html_only("Ana"));
    assert_eq!(size_of(&html, "lamina-1"), (800, 800));
    assert_eq!(size_of(&html, "lamina-2"), (1600, 800));
    assert_eq!(size_of(&html, "lamina-3"), (800, 800));
    let data = data_of(&html);
    assert_eq!(data["guardas"]["inicio"], "neutra");
    assert_eq!(data["guardas"]["fim"], "neutra");
}

fn face_json(face: Face) -> Value {
    match face {
        Face::Blank => json!("neutra"),
        Face::Spread { index, side } => json!({
            "lamina": index + 1,
            "lado": if side == PageSide::Left { "esquerda" } else { "direita" },
        }),
    }
}

#[test]
fn ah_b6_dados_do_livro() {
    use AlbumEdge::{FullSpread, SinglePage};
    for (start, end) in [
        (FullSpread, FullSpread),
        (SinglePage, SinglePage),
        (SinglePage, FullSpread),
        (FullSpread, SinglePage),
    ] {
        let album = Album::new(spec().with_edges(start, end), 3);
        let data = data_of(&page(&album, &html_only("Ana")));
        let book = Book::from_project(&album.project);
        assert_eq!(data["laminas"], 3);
        assert_eq!(data["guardas"]["inicio"], face_json(book.inside_front));
        assert_eq!(data["guardas"]["fim"], face_json(book.inside_back));
        let leaves: Vec<Value> = book
            .leaves
            .iter()
            .map(|l| json!({"frente": face_json(l.front), "verso": face_json(l.back)}))
            .collect();
        assert_eq!(data["folhas"], json!(leaves));
        assert_eq!(data["medidas"]["pagina"], json!([300.0, 300.0]));
    }
}

#[test]
fn ah_b7_sem_capa_criada() {
    let html = page(&three(), &html_only("Ana"));
    let data = data_of(&html);
    assert_eq!(data["capa_lisa"], true);
    assert_eq!(data["medidas"]["face"], json!([303.0, 306.0]));
    assert!(html.contains("Capa ainda não criada"));
    assert!(!html.contains("id=\"capa-frente\""));
    let mut with = three();
    with_cover(&mut with);
    assert_eq!(data_of(&page(&with, &html_only("Ana")))["capa_lisa"], false);
}

fn with_phone(phone: &str) -> ApprovalOptions {
    let mut options = html_only("Ana");
    if let Some(html) = &mut options.html {
        html.whatsapp = Some(phone.to_owned());
    }
    options
}

#[test]
fn ah_b8_mesmos_bytes() {
    let album = three();
    let first = page(&album, &with_phone("5511999999999"));
    let second = page(&album, &with_phone("5511999999999"));
    let third = page(&album, &with_phone("5521888888888"));
    assert_eq!(first, second);
    assert_ne!(first, third);
    assert_eq!(first, third.replace("5521888888888", "5511999999999"));
    assert_eq!(data_of(&first)["codigo"], data_of(&third)["codigo"]);
}

#[test]
fn ah_b9_contato_no_html() {
    let mut options = with_phone("5511999999999");
    if let Some(html) = &mut options.html {
        html.email = Some("estudio@exemplo.com".to_owned());
    }
    let html = without_scripts(&without_data(&page(&three(), &options)));
    assert!(html.contains("https://wa.me/5511999999999?text="));
    assert!(html.contains("mailto:estudio@exemplo.com?"));
}

#[test]
fn ah_b10_sem_contato() {
    let html = without_scripts(&without_data(&page(&three(), &html_only("Ana"))));
    assert!(html.contains("https://wa.me/?text="));
    assert!(html.contains("mailto:?"));
}

#[test]
fn ah_b21_aviso_no_comeco_do_arquivo() {
    let html = page(&three(), &html_only("Ana"));
    let mut lines = html.lines();
    assert_eq!(lines.next(), Some("<!doctype html>"));
    assert_eq!(
        lines.next(),
        Some(
            "<!-- Este é o seu álbum. Se você está vendo este texto, baixe o arquivo e abra no navegador. -->"
        )
    );
}

/// Um nome com marcação não vira marcação: nem na página, nem saindo do bloco de dados.
#[test]
fn nome_com_marcacao_nao_quebra_a_pagina() {
    let name = "<b>Ana</b> & \"Pedro\"</script>";
    let html = without_data(&page(&three(), &html_only(name)));
    assert!(!html.contains("<b>Ana"));
    assert!(html.contains("&lt;b&gt;Ana&lt;/b&gt; &amp; &quot;Pedro&quot;"));
    assert_eq!(data_of(&html)["nome"], name);
}
