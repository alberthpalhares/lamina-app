//! A aba Detalhes, o painel contextual (spec AF, seção 4 e regras 10 e 12): sem seleção, o
//! álbum; com uma foto, a foto e a câmera (lida do arquivo na hora, sem guardar); com um texto,
//! os campos do texto; com vários quadros, a seleção do modo organizar. A aba só se abre sozinha
//! ao entrar no recorte, no modo organizar ou em "Ajustar a foto…".

use lamina_core::{ColorSpace, FrameContent, Orientation, Project};
use lamina_media::camera_details;

use super::Editor;

/// O que a aba Detalhes mostra agora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Details {
    /// O álbum: as linhas (título, valor).
    Album(Vec<(String, String)>),
    /// Uma foto: as linhas dela (a seção Foto vem antes, na janela).
    Photo(Vec<(String, String)>),
    /// Um texto: os campos do texto e do quadro (aba Quadro da TX).
    Text,
    /// Vários quadros (modo organizar).
    Frames(usize),
}

fn row(title: &str, value: impl Into<String>) -> (String, String) {
    (title.to_owned(), value.into())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// As linhas do álbum (AF-B20).
pub fn album_rows(project: &Project, unit: &str) -> Vec<(String, String)> {
    let spreads = project.spreads.len();
    let pages = project.spec.edges().page_count(spreads);
    let mut rows = vec![row(
        "Lâminas",
        format!(
            "{} · {}",
            plural(spreads, "lâmina", "lâminas"),
            plural(pages, "página", "páginas")
        ),
    )];
    let limits = &project.page_limits;
    if limits.min.is_some() || limits.max.is_some() {
        let range = match (limits.min, limits.max) {
            (Some(a), Some(b)) => format!("de {a} a {b} páginas"),
            (Some(a), None) => format!("pelo menos {a} páginas"),
            (None, Some(b)) => format!("até {b} páginas"),
            (None, None) => String::new(),
        };
        let pages32 = u32::try_from(pages).unwrap_or(u32::MAX);
        let outside =
            limits.min.is_some_and(|a| pages32 < a) || limits.max.is_some_and(|b| pages32 > b);
        let warn = if outside {
            " · fora da faixa da encadernadora"
        } else {
            ""
        };
        rows.push(row("Faixa de páginas", format!("{range}{warn}")));
    }
    let count = |o: Orientation| {
        project
            .photos
            .iter()
            .filter(|p| p.orientation() == o)
            .count()
    };
    let used = project
        .photos
        .iter()
        .filter(|p| {
            project
                .all_content()
                .flat_map(|s| &s.frames)
                .any(|f| matches!(&f.content, FrameContent::Photo { photo, .. } if *photo == p.id))
        })
        .count();
    let adjusted = project
        .photos
        .iter()
        .filter(|p| !p.adjust.is_neutral())
        .count();
    rows.push(row(
        "Fotos",
        format!(
            "{} · {} horizontais, {} verticais, {} quadradas · {} usadas",
            plural(project.photos.len(), "foto", "fotos"),
            count(Orientation::Landscape),
            count(Orientation::Portrait),
            count(Orientation::Square),
            used
        ),
    ));
    rows.push(row(
        "Ajustes",
        plural(adjusted, "foto com ajuste", "fotos com ajuste"),
    ));
    let space = match project.export.space {
        ColorSpace::Srgb => "sRGB",
        ColorSpace::AdobeRgb => "Adobe RGB",
    };
    rows.push(row("Perfil de saída", space));
    rows.push(row("Unidade", unit));
    rows
}

/// As linhas do álbum nas seções do painel (spec DJ, regra 15): Álbum e Impressão, nessa ordem; a
/// seção Lâmina vem da janela (a lâmina atual e os avisos dela).
#[allow(clippy::type_complexity)]
pub fn album_sections(
    rows: Vec<(String, String)>,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    rows.into_iter()
        .partition(|(title, _)| title != "Perfil de saída")
}

impl Editor {
    /// O que a aba Detalhes mostra para a seleção de agora.
    pub fn details(&self) -> Details {
        let frames = self.selected_frames();
        let Some(s) = self.session.as_ref() else {
            return Details::Album(Vec::new());
        };
        let project = s.project();
        if frames.len() > 1 {
            return Details::Frames(frames.len());
        }
        let content = frames
            .first()
            .and_then(|f| project.spreads.get(s.current())?.frames.get(*f))
            .map(|f| &f.content);
        match content {
            Some(FrameContent::Text(_)) => Details::Text,
            Some(FrameContent::Photo { photo, .. }) => {
                Details::Photo(self.photo_rows(project, *photo, frames[0]))
            }
            _ => Details::Album(album_rows(project, self.strips.strips.unit.label())),
        }
    }

    /// As linhas de uma foto: arquivo, pasta, dimensões, DPI efetivo e a câmera (AF-B21).
    fn photo_rows(
        &self,
        project: &Project,
        id: lamina_core::PhotoId,
        frame: usize,
    ) -> Vec<(String, String)> {
        let Ok(photo) = project.photo(id) else {
            return Vec::new();
        };
        let path = match self.session.as_ref().and_then(|s| s.file()?.parent()) {
            Some(dir) => dir.join(&photo.path),
            None => photo.path.clone(),
        };
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let folder = path.parent().map(|p| p.display().to_string());
        let mut rows = vec![row("Arquivo", name.unwrap_or_default())];
        if let Some(folder) = folder.filter(|f| !f.is_empty()) {
            rows.push(row("Pasta", folder));
        }
        rows.push(row(
            "Dimensões",
            format!("{} × {} px", photo.width_px, photo.height_px),
        ));
        let index = self.session.as_ref().map_or(0, |s| s.current());
        if let Some(dpi) = project
            .spreads
            .get(index)
            .and_then(|s| s.frames.get(frame))
            .and_then(|f| project.effective_dpi(f).ok().flatten())
        {
            rows.push(row("DPI efetivo", format!("{dpi:.0}")));
        }
        let camera = self.camera_of(&path);
        let optional = [
            ("Perfil de cor", camera.color),
            ("Câmera", photo.camera.clone()),
            ("Data", photo.taken_at.map(|t| t.to_string())),
            ("Velocidade", camera.shutter),
            ("Abertura", camera.aperture),
            ("ISO", camera.iso),
            ("Distância focal", camera.focal),
        ];
        rows.extend(optional.into_iter().filter_map(|(t, v)| Some(row(t, v?))));
        rows
    }

    /// As informações da câmera do arquivo, lidas uma vez por foto selecionada.
    fn camera_of(&self, path: &std::path::Path) -> lamina_media::CameraDetails {
        let mut cache = self.camera.borrow_mut();
        if let Some((p, d)) = cache.as_ref()
            && p == path
        {
            return d.clone();
        }
        let d = camera_details(path);
        *cache = Some((path.to_path_buf(), d.clone()));
        d
    }

    /// A janela abre a aba Detalhes? Só depois do recorte, do modo organizar ou de "Ajustar a
    /// foto…" (regra 10); clicar numa foto não troca a aba. Ler limpa o pedido.
    pub fn take_details_request(&mut self) -> bool {
        std::mem::take(&mut self.wants_details)
    }
}
