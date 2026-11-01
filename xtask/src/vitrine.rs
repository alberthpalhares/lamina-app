//! A vitrine do repositório público (spec VP): os links de baixar do README seguem a versão, as
//! notas da versão mostram as ilustrações e a mídia do README e do site fica leve.

use lamina_editor::updates::news::video_address;

use crate::versao::REPO;

/// Onde mora a mídia do README e do site (`site/midia/`). As travas da mídia (VP-B5 e VP-B7)
/// são testes; a porta de verificação os roda.
#[cfg(test)]
pub const MEDIA_DIR: &str = "site/midia";

/// Limite de cada arquivo de mídia e da soma das imagens do README (regra 6).
#[cfg(test)]
pub const MAX_FILE: u64 = 4 * 1024 * 1024;
#[cfg(test)]
pub const MAX_README: u64 = 25 * 1024 * 1024;

/// O link de baixar do instalador de um sistema (`win` + `-Setup.exe`): o arquivo da própria tag
/// numa versão de teste, o `latest` numa estável (regra 2).
pub fn download(version: &str, system: &str, ending: &str) -> String {
    let base = format!("https://github.com/{REPO}/releases");
    if version.contains('-') {
        format!("{base}/download/v{version}/Lamina-{system}-beta{ending}")
    } else {
        format!("{base}/latest/download/Lamina-{system}{ending}")
    }
}

/// O README com os marcadores trocados pelos links e pela versão (regras 2 e 3).
pub fn fill(template: &str, version: &str) -> String {
    template
        .replace(
            "{{baixar_windows}}",
            &download(version, "win", "-Setup.exe"),
        )
        .replace("{{baixar_mac}}", &download(version, "osx", "-Setup.pkg"))
        .replace("{{baixar_linux}}", &download(version, "linux", ".AppImage"))
        .replace("{{versao}}", version)
}

/// As notas com cada `<!-- imagem: x.png … -->` trocado pela imagem da tag (regra 5) e, se o
/// comentário tiver um vídeo do site do Lamina, o link dele (spec PV, seção 4).
pub fn note_images(notes: &str, tag: &str) -> String {
    let mut out = String::with_capacity(notes.len());
    let mut rest = notes;
    while let Some(start) = rest.find("<!-- imagem:") {
        let Some(len) = rest[start..].find("-->") else {
            break;
        };
        out.push_str(&rest[..start]);
        let inner = &rest[start + "<!-- imagem:".len()..start + len];
        let file = inner.split(';').next().unwrap_or("").trim();
        out.push_str(&format!(
            "<br><img src=\"https://raw.githubusercontent.com/{REPO}/{tag}/crates/lamina-app/dist/novidades/{file}\" width=\"560\" alt=\"\">"
        ));
        if let Some(video) = video_of(inner) {
            out.push_str(&format!("<br><a href=\"{video}\">Assistir ao vídeo</a>"));
        }
        rest = &rest[start + len + "-->".len()..];
    }
    out.push_str(rest);
    out
}

/// O vídeo de um comentário, só se passar na mesma conferência do Lamina (spec PV, regra 8).
fn video_of(comment: &str) -> Option<&str> {
    comment
        .split(';')
        .filter_map(|field| field.split_once(':'))
        .find(|(key, _)| key.trim() == "video")
        .and_then(|(_, value)| video_address(value.trim()))
}

/// Os caminhos relativos que um Markdown cita: `](…)`, `src="…"` e `srcset="…"` (regra 4).
#[cfg(test)]
pub fn relative_links(text: &str) -> Vec<&str> {
    let mut links: Vec<&str> = text
        .split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .collect();
    for attr in ["src=\"", "srcset=\""] {
        links.extend(
            text.split(attr)
                .skip(1)
                .filter_map(|rest| rest.split('"').next()),
        );
    }
    links.retain(|l| !l.starts_with("http") && !l.starts_with('#') && !l.is_empty());
    links.sort_unstable();
    links.dedup();
    links
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::espelho::{CODE, PUBLIC_README, SLINT_BADGE, public_readme};
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }

    #[test]
    fn vp_b2_links_de_baixar_de_uma_versao_de_teste() {
        let readme = fill(PUBLIC_README, "0.1.0-beta.8");
        let tag = format!("https://github.com/{REPO}/releases/download/v0.1.0-beta.8");
        for file in [
            "Lamina-win-beta-Setup.exe",
            "Lamina-osx-beta-Setup.pkg",
            "Lamina-linux-beta.AppImage",
        ] {
            assert!(readme.contains(&format!("{tag}/{file}")), "{file}");
        }
        assert!(!readme.contains("releases/latest/download"));
    }

    #[test]
    fn vp_b3_links_de_baixar_da_versao_estavel() {
        let readme = fill(PUBLIC_README, "1.0.0");
        let latest = format!("https://github.com/{REPO}/releases/latest/download");
        for file in [
            "Lamina-win-Setup.exe",
            "Lamina-osx-Setup.pkg",
            "Lamina-linux.AppImage",
        ] {
            assert!(readme.contains(&format!("{latest}/{file}")), "{file}");
        }
        assert!(!readme.contains("-beta-Setup") && !readme.contains("-beta.AppImage"));
    }

    #[test]
    fn vp_b4_nenhum_marcador_sobra() {
        for version in ["0.1.0-beta.8", "1.0.0"] {
            let readme = public_readme(version);
            assert!(!readme.contains("{{"), "{version}");
            assert!(readme.ends_with(&format!("\n{SLINT_BADGE}\n")));
        }
    }

    #[test]
    fn vp_b5_toda_imagem_do_readme_existe_e_vai_ao_publico() {
        let readme = fill(PUBLIC_README, "1.0.0");
        let links = relative_links(&readme);
        assert!(links.iter().any(|l| l.starts_with(MEDIA_DIR)), "{links:?}");
        // O `LICENSE` só existe no público, e o espelho o mantém lá.
        for link in links.into_iter().filter(|l| *l != "LICENSE") {
            assert!(root().join(link).exists(), "{link} não existe");
            let top = link.split('/').next().unwrap_or("");
            assert!(CODE.contains(&top), "{link} não vai para o público");
        }
        // O site pede as versões ao mesmo repositório (regra 7).
        let site = std::fs::read_to_string(root().join("site/app.js")).unwrap();
        assert!(site.contains(&format!("const REPO = '{REPO}';")));
    }

    #[test]
    fn vp_b6_notas_com_imagem() {
        let notes =
            "- **Zoom.** Texto. <!-- imagem: zoom.png; comando: ver.zoom -->\n- **B.** Só texto.";
        let out = note_images(notes, "v1.0.0");
        let url = format!(
            "https://raw.githubusercontent.com/{REPO}/v1.0.0/crates/lamina-app/dist/novidades/zoom.png"
        );
        assert!(out.contains(&format!("<img src=\"{url}\"")), "{out}");
        assert!(
            !out.contains("<!--") && out.ends_with("- **B.** Só texto."),
            "{out}"
        );
    }

    #[test]
    fn pv_b14_a_pagina_da_versao_leva_o_link_do_video() {
        let video = "https://alberthpalhares.github.io/lamina-app/v/a.html";
        let notes = format!(
            "## [1.0.0] - 2026-11-12\n\n- **A.** Texto. <!-- imagem: a.jpg; tipo: novo; video: {video} -->\n\
             - **B.** Só imagem. <!-- imagem: b.png -->\n\
             - **C.** Vídeo de fora. <!-- imagem: c.png; video: https://exemplo.org/v/c.html -->\n"
        );
        let out = note_images(&notes, "v1.0.0");
        let base = format!(
            "https://raw.githubusercontent.com/{REPO}/v1.0.0/crates/lamina-app/dist/novidades"
        );
        assert!(
            out.contains(&format!(
                "<img src=\"{base}/a.jpg\" width=\"560\" alt=\"\"><br><a href=\"{video}\">Assistir ao vídeo</a>"
            )),
            "{out}"
        );
        assert!(
            out.contains(&format!(
                "Só imagem. <br><img src=\"{base}/b.png\" width=\"560\" alt=\"\">\n"
            )),
            "{out}"
        );
        assert_eq!(out.matches("Assistir ao vídeo").count(), 1, "{out}");
        // O texto da seção, que vai para as notas e para o Velopack (NV-B7), continua igual.
        let section = lamina_editor::updates::news::section_text(&notes, "1.0.0").unwrap();
        assert!(!section.contains("Assistir ao vídeo"));
        assert!(section.contains(&format!("video: {video} -->")));
    }

    #[test]
    fn vp_b7_midia_leve() {
        let dir = root().join(MEDIA_DIR);
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let size = entry.metadata().unwrap().len();
            assert!(size <= MAX_FILE, "{:?}: {size} bytes", entry.file_name());
        }
        let readme = fill(PUBLIC_README, "1.0.0");
        let total: u64 = relative_links(&readme)
            .iter()
            .filter_map(|l| std::fs::metadata(root().join(l)).ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum();
        assert!(total <= MAX_README, "{total} bytes");
    }
}
