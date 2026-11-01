//! Cenários da spec `pagina-de-novidades` (PV) sem janela: o leitor, o endereço do vídeo, as notas de
//! verdade, destaques, filtros, busca, a capa e o "Experimente agora".

use std::io::Cursor;
use std::path::Path;

use super::tl_common::{editor_at, three_spreads};
use crate::Editor;
use crate::updates::Version;
use crate::updates::news::{
    NewsFilter, NewsItem, NewsKind, NewsPage, NewsSystem, TryNow, after_update, animates,
    available_filters, cover, entries, from_help, highlights, no_results, parse, shown,
    shows_highlights, video_address,
};

const REAL: &str = include_str!("../../../lamina-app/dist/novidades.md");
const VIDEO: &str = "https://alberthpalhares.github.io/lamina-app/v/a.html";
/// Três quadros de 8×8, como no NV-B14.
const THREE: &[u8] = include_bytes!("dados/tres-quadros.png");

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

/// A página pela Ajuda, a partir do texto das notas, com a versão mais nova marcada.
fn page(text: &str) -> NewsPage {
    from_help(&parse(text), &v("9.9.9"))
}

fn labels(list: &[crate::updates::news::NewsEntry]) -> Vec<&str> {
    list.iter().map(|e| e.label.as_str()).collect()
}

fn one(comment: &str) -> NewsItem {
    let text = format!("## [0.1.0-beta.9] - 2026-11-12\n\n- **A.** Texto. {comment}\n");
    parse(&text).remove(0).items.remove(0)
}

#[test]
fn pv_b1_o_comentario_traz_tipo_video_destaque_e_sistema() {
    let first = one(&format!(
        "<!-- imagem: a.jpg; comando: ajuda.atalhos; tipo: novo; video: {VIDEO}; fixada: sim; so: mac -->"
    ));
    let other = one(&format!(
        "<!-- so: mac; fixada: sim; video: {VIDEO}; tipo: novo; comando: ajuda.atalhos; imagem: a.jpg -->"
    ));
    for item in [&first, &other] {
        assert_eq!(item.image.as_deref(), Some("a.jpg"));
        assert_eq!(item.command.as_deref(), Some("ajuda.atalhos"));
        assert_eq!(item.kind, Some(NewsKind::New));
        assert_eq!(item.video.as_deref(), Some(VIDEO));
        assert!(item.pinned);
        assert_eq!(item.only, Some(NewsSystem::Mac));
        assert_eq!(item.text, "Texto.");
    }
    assert_eq!(
        one("<!-- tipo: melhoria -->").kind,
        Some(NewsKind::Improvement)
    );
    assert_eq!(one("<!-- so: windows -->").only, Some(NewsSystem::Windows));
    assert_eq!(one("<!-- so: linux -->").only, Some(NewsSystem::Linux));
}

#[test]
fn pv_b2_item_antigo_le_como_antes() {
    let old = one("<!-- imagem: a.png; comando: b -->");
    assert_eq!(old.image.as_deref(), Some("a.png"));
    assert_eq!(old.command.as_deref(), Some("b"));
    assert_eq!(
        (old.kind, old.video.as_ref(), old.pinned, old.only),
        (None, None, false, None)
    );
    let none = one("");
    assert_eq!((none.image.as_ref(), none.command.as_ref()), (None, None));
    assert_eq!(
        (none.kind, none.video.as_ref(), none.pinned, none.only),
        (None, None, false, None)
    );
    let odd = one("<!-- tipo: raro; so: amiga; fixada: talvez; outro: x -->");
    assert_eq!((odd.kind, odd.pinned, odd.only), (None, false, None));
}

#[test]
fn pv_b3_so_o_endereco_do_site_vira_video() {
    let good = "https://alberthpalhares.github.io/lamina-app/v/janela-nova.html";
    assert_eq!(video_address(good), Some(good));
    for bad in [
        "http://alberthpalhares.github.io/lamina-app/v/janela-nova.html",
        "https://alberthpalhares.github.io/lamina-app/v/../x.html",
        "https://alberthpalhares.github.io/lamina-app/v/Janela.html",
        "https://alberthpalhares.github.io/lamina-app/v/.html",
        "https://exemplo.org/v/a.html",
        "file:///c:/a.html",
        "",
    ] {
        assert_eq!(video_address(bad), None, "{bad}");
    }
}

/// Os comentários do arquivo, sem as pontas: `imagem: x; comando: y`.
fn comments(text: &str) -> Vec<String> {
    text.split("<!--")
        .skip(1)
        .filter_map(|rest| rest.split_once("-->"))
        .map(|(inside, _)| inside.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn pv_b4_as_notas_de_verdade_seguem_as_regras() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sections = parse(&REAL.replace("## [Não lançado]", "## [9.9.9]"));
    assert!(!sections.is_empty());
    for section in &sections {
        let pinned = section.items.iter().filter(|i| i.pinned).count();
        assert!(pinned <= 3, "{}: {pinned} fixadas", section.version);
        for item in &section.items {
            if item.video.is_some() || item.pinned {
                assert!(item.image.is_some(), "{:?} sem imagem", item.title);
            }
            if let Some(video) = &item.video {
                let address = video_address(video);
                assert!(address.is_some(), "{video} não é do site do Lamina");
                let name = video.rsplit('/').next().unwrap();
                assert!(
                    root.join("site/v").join(name).is_file(),
                    "falta site/v/{name}"
                );
            }
        }
    }
    for comment in comments(REAL) {
        if comment.contains("imagem:") {
            assert!(
                comment.starts_with("imagem:"),
                "imagem fora do começo: {comment}"
            );
        }
    }
}

/// `novas` itens fixados na beta.9 e um na beta.8.
fn pinned_notes(fixed: usize) -> String {
    let items: String = (1..=4)
        .map(|n| {
            let mark = if n <= fixed {
                " <!-- imagem: a.png; fixada: sim -->"
            } else {
                ""
            };
            format!("- **Nove {n}.** Texto.{mark}\n")
        })
        .collect();
    format!(
        "## [0.1.0-beta.9] - 2026-11-12\n\n- **Sem marca.** Texto.\n{items}\n\
         ## [0.1.0-beta.8] - 2026-10-31\n\n- **Oito.** Texto. <!-- imagem: b.png; fixada: sim -->\n"
    )
}

#[test]
fn pv_b5_os_destaques_sao_so_da_versao_mais_nova_da_pagina() {
    let list = highlights(&page(&pinned_notes(4)));
    let found: Vec<(usize, &str)> = list.iter().map(|(i, e)| (*i, e.label.as_str())).collect();
    assert_eq!(found, [(1, "Nove 1"), (2, "Nove 2"), (3, "Nove 3")]);
    assert!(highlights(&page(&pinned_notes(0))).is_empty());
}

fn four_kinds() -> NewsPage {
    page(
        "## [0.1.0-beta.9] - 2026-11-12\n\n\
          - **Novo.** Um. <!-- tipo: novo -->\n\
          - **Melhor.** Dois. <!-- tipo: melhoria -->\n\
          - **Sem tipo.** Três.\n\
          - **Vídeo errado.** Quatro. <!-- video: https://exemplo.org/v/a.html -->\n",
    )
}

#[test]
fn pv_b6_so_aparecem_os_filtros_que_tem_novidade() {
    let p = four_kinds();
    for _ in 0..2 {
        assert_eq!(
            available_filters(&p),
            [NewsFilter::All, NewsFilter::New, NewsFilter::Improvements]
        );
        assert_eq!(shown(&p, NewsFilter::All, "").len(), 4);
        assert_eq!(labels(&shown(&p, NewsFilter::New, "")), ["Novo"]);
        assert_eq!(labels(&shown(&p, NewsFilter::Improvements, "")), ["Melhor"]);
        assert!(shown(&p, NewsFilter::Video, "").is_empty());
    }
    let with_video = page(&format!(
        "## [0.1.0-beta.9] - 2026-11-12\n\n- **V.** Um. <!-- imagem: a.jpg; video: {VIDEO} -->\n"
    ));
    assert_eq!(
        available_filters(&with_video),
        [NewsFilter::All, NewsFilter::Video]
    );
    assert_eq!(shown(&with_video, NewsFilter::Video, "").len(), 1);
    let plain = page("## [0.1.0-beta.8] - 2026-10-31\n\n- **A.** Um.\n- **B.** Dois.\n");
    assert_eq!(available_filters(&plain), [NewsFilter::All]);
}

#[test]
fn pv_b7_o_cabecalho_da_versao_vai_no_primeiro_item_que_aparece() {
    let p = page(
        "## [0.1.0-beta.9] - 2026-11-12\n\nAbertura.\n\n\
         - **Melhor.** Um. <!-- tipo: melhoria -->\n\
         - **Novo.** Dois. <!-- tipo: novo -->\n",
    );
    let all = shown(&p, NewsFilter::All, "");
    assert_eq!(all[1].heading, None);
    assert_eq!((all[0].index, all[1].index), (0, 1));
    let list = shown(&p, NewsFilter::New, "");
    assert_eq!(labels(&list), ["Novo"]);
    assert_eq!(list[0].index, 1, "o índice do item não muda com o filtro");
    assert!(list[0].heading.as_deref().unwrap().contains("0.1.0-beta.9"));
    assert_eq!(list[0].intro, "Abertura.");
    assert_eq!(list[0].date, "12/11/2026");
    assert_eq!(list[0].version, "0.1.0-beta.9");
}

fn photos() -> NewsPage {
    page(
        "## [0.1.0-beta.9] - 2026-11-12\n\n- **Fontes do computador.** Escolha a letra.\n\n\
          ## [0.1.0-beta.7] - 2026-10-29\n\n- **Fotos que mudaram de lugar.** O Lamina acha.\n",
    )
}

#[test]
fn pv_b8_a_busca_ignora_acento_e_maiusculas_e_pede_todas_as_palavras() {
    let p = photos();
    let second = ["Fotos que mudaram de lugar"];
    assert_eq!(labels(&shown(&p, NewsFilter::All, "fótos LUGAR")), second);
    assert!(shown(&p, NewsFilter::All, "fontes lugar").is_empty());
    assert_eq!(labels(&shown(&p, NewsFilter::All, "beta.7")), second);
    assert_eq!(shown(&p, NewsFilter::All, "  ").len(), 2);
    assert!(shown(&p, NewsFilter::All, "xyz").is_empty());
    assert_eq!(no_results("xyz"), "Nenhuma novidade com “xyz”.");
    assert_eq!(no_results("  xyz "), "Nenhuma novidade com “xyz”.");
}

#[test]
fn pv_b9_a_busca_vale_junto_com_o_filtro() {
    let p = page(
        "## [0.1.0-beta.9] - 2026-11-12\n\n\
         - **Novo.** Uma foto. <!-- tipo: novo -->\n\
         - **Melhor.** Outra foto. <!-- tipo: melhoria -->\n",
    );
    assert_eq!(
        labels(&shown(&p, NewsFilter::Improvements, "foto")),
        ["Melhor"]
    );
}

#[test]
fn pv_b10_depois_de_atualizar_so_o_que_falta_ver_o_botao_mostra_o_resto() {
    let notes: String = (5..=9)
        .map(|n| format!("## [0.1.0-beta.{n}] - 2026-10-2{n}\n\n- **Item {n}.** Texto.\n\n"))
        .collect();
    let sections = parse(&notes);
    let current = v("0.1.0-beta.9");
    let fresh = after_update(&sections, &v("0.1.0-beta.7"), &current).unwrap();
    assert_eq!(fresh.title, "Lamina atualizado para 0.1.0-beta.9");
    assert!(!fresh.subtitle.is_empty());
    let seen: Vec<String> = fresh
        .sections
        .iter()
        .map(|s| s.version.to_string())
        .collect();
    assert_eq!(seen, ["0.1.0-beta.9", "0.1.0-beta.8"]);
    assert!(!entries(&fresh)[0].current);

    let all = from_help(&sections, &current);
    assert_eq!(all.title, "O que há de novo no Lamina");
    assert_eq!(all.subtitle, "");
    assert_eq!(all.sections.len(), 5);
    assert!(entries(&all)[0].current, "Esta versão");
}

/// Um PNG de `w`×`h` da cor dada.
fn png(w: u32, h: u32) -> Vec<u8> {
    let picture = image::RgbImage::from_pixel(w, h, image::Rgb([9, 99, 199]));
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

fn jpeg(w: u32, h: u32) -> Vec<u8> {
    let picture = image::RgbImage::from_pixel(w, h, image::Rgb([9, 99, 199]));
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
        .unwrap();
    bytes
}

#[test]
fn pv_b11_a_capa_ocupa_pouca_memoria() {
    let big = cover(&png(1280, 720)).unwrap();
    assert_eq!((big.width, big.height), (640, 360));
    assert_eq!(big.rgba.len(), 640 * 360 * 4);
    let card = cover(&jpeg(640, 360)).unwrap();
    assert_eq!((card.width, card.height), (640, 360));
    let tall = cover(&png(720, 1280)).unwrap();
    assert!(tall.width <= 640 && tall.height <= 360);
    assert_eq!((tall.width, tall.height), (203, 360), "mantém a proporção");
    let small = cover(THREE).unwrap();
    assert_eq!((small.width, small.height), (8, 8), "não amplia");
    assert!(cover(b"nada").is_none());
}

#[test]
fn as_etiquetas_e_os_filtros_tem_os_nomes_da_spec() {
    assert_eq!(NewsKind::New.label(), "Novo recurso");
    assert_eq!(NewsKind::Improvement.label(), "Melhoria");
    assert_eq!(NewsSystem::Mac.label(), "Só no Mac");
    assert_eq!(NewsSystem::Windows.label(), "Só no Windows");
    assert_eq!(NewsSystem::Linux.label(), "Só no Linux");
    let names: Vec<&str> = [
        NewsFilter::All,
        NewsFilter::New,
        NewsFilter::Improvements,
        NewsFilter::Video,
    ]
    .map(NewsFilter::label)
    .to_vec();
    assert_eq!(names, ["Todas", "Novos recursos", "Melhorias", "Com vídeo"]);
}

#[test]
fn so_o_png_animado_conta_como_animacao() {
    assert!(animates(THREE));
    assert!(!animates(&png(8, 8)));
    assert!(!animates(&jpeg(8, 8)));
    assert!(!animates(b"nada"));
}

#[test]
fn pv_b16_os_destaques_somem_com_busca_ou_filtro() {
    assert!(shows_highlights(NewsFilter::All, ""));
    assert!(shows_highlights(NewsFilter::All, "   "));
    assert!(!shows_highlights(NewsFilter::All, "foto"));
    assert!(!shows_highlights(NewsFilter::New, ""));
    assert!(!shows_highlights(NewsFilter::Improvements, ""));
    assert!(!shows_highlights(NewsFilter::Video, " "));
}

fn other_system() -> NewsSystem {
    match NewsSystem::current() {
        NewsSystem::Mac => NewsSystem::Linux,
        _ => NewsSystem::Mac,
    }
}

#[test]
fn pv_b17_experimente_agora_diz_quando_nao_da() {
    let shortcuts = NewsItem {
        command: Some("ajuda.atalhos".to_owned()),
        ..NewsItem::default()
    };
    let organize = NewsItem {
        command: Some("lamina.organizar".to_owned()),
        ..NewsItem::default()
    };
    let no_command = NewsItem::default();
    let elsewhere = NewsItem {
        command: Some("ajuda.atalhos".to_owned()),
        only: Some(other_system()),
        ..NewsItem::default()
    };
    let unknown = NewsItem {
        command: Some("nao.existe".to_owned()),
        ..NewsItem::default()
    };
    let here = NewsItem {
        only: Some(NewsSystem::current()),
        ..shortcuts.clone()
    };

    let no_album = Editor::new();
    let with_album = editor_at(three_spreads(), 0);

    assert_eq!(no_album.try_now(&shortcuts), TryNow::Ready);
    assert_eq!(with_album.try_now(&shortcuts), TryNow::Ready);
    assert_eq!(no_album.try_now(&here), TryNow::Ready);
    assert_eq!(
        no_album.try_now(&organize),
        TryNow::Unavailable("Abra um álbum para experimentar.".to_owned())
    );
    assert_eq!(with_album.try_now(&organize), TryNow::Ready);
    for editor in [&no_album, &with_album] {
        assert_eq!(editor.try_now(&no_command), TryNow::Hidden);
        assert_eq!(editor.try_now(&elsewhere), TryNow::Hidden);
        assert_eq!(editor.try_now(&unknown), TryNow::Hidden);
    }
}

#[test]
fn pv_b17_com_album_e_comando_indisponivel_explica_sem_falar_em_abrir() {
    let mut busy = editor_at(three_spreads(), 0);
    busy.set_busy(true);
    let organize = NewsItem {
        command: Some("lamina.organizar".to_owned()),
        ..NewsItem::default()
    };
    assert_eq!(
        busy.try_now(&organize),
        TryNow::Unavailable("Não dá para experimentar agora.".to_owned())
    );
}
