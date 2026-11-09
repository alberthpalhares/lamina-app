//! A ordenação das fotos do painel (spec OF, beta 9.1): o critério escolhido (memória) e a ordem
//! personalizada (gravada no projeto). A montagem automática e a linha de comando continuam na
//! ordem de captura (regra 12 do determinismo).

use lamina_core::{Photo, PhotoId};

/// O critério de ordenação do painel de fotos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    /// Data de captura (padrão, como hoje); sem data, no fim.
    #[default]
    Capture,
    /// Nome do arquivo, alfabética sem diferenciar maiúsculas.
    Name,
    /// A ordem personalizada que o fotógrafo montou arrastando (`Project::photo_order`).
    Custom,
}

impl SortBy {
    /// O nome no seletor do painel.
    pub fn label(self) -> &'static str {
        match self {
            SortBy::Capture => "Data de captura",
            SortBy::Name => "Nome do arquivo",
            SortBy::Custom => "Personalizada",
        }
    }

    /// As opções do seletor, na ordem da tela.
    pub const ALL: [SortBy; 3] = [SortBy::Capture, SortBy::Name, SortBy::Custom];
}

/// As fotos na ordem do critério. `custom` é a ordem personalizada do projeto (vazia se não há).
/// Fotos sem o dado do critério vão para o fim, pela ordem de captura (regra 7).
pub fn sorted<'a>(
    photos: impl Iterator<Item = &'a Photo> + Clone,
    sort: SortBy,
    custom: &[PhotoId],
) -> Vec<&'a Photo> {
    let mut list: Vec<&Photo> = photos.collect();
    match sort {
        SortBy::Capture => {
            list.sort_by_key(|p| (p.taken_at.is_none(), p.taken_at, p.path.clone()));
        }
        SortBy::Name => {
            list.sort_by_key(|p| {
                let name = p
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_lowercase();
                (name, p.path.clone())
            });
        }
        SortBy::Custom => {
            // A posição de cada foto na ordem personalizada; as que não estão nela vão para o
            // fim, pela ordem de captura.
            let pos = |id: PhotoId| custom.iter().position(|c| *c == id);
            list.sort_by_key(|p| {
                (
                    pos(p.id).is_none(),
                    pos(p.id).unwrap_or(usize::MAX),
                    p.taken_at,
                )
            });
        }
    }
    list
}

/// Move a foto `id` para a posição `to` na ordem personalizada. Se ainda não há ordem
/// personalizada, ela nasce da ordem de captura. Devolve a nova ordem.
pub fn move_to(order: &[PhotoId], capture: &[PhotoId], id: PhotoId, to: usize) -> Vec<PhotoId> {
    let mut out: Vec<PhotoId> = if order.is_empty() {
        capture.to_vec()
    } else {
        order.to_vec()
    };
    let Some(from) = out.iter().position(|p| *p == id) else {
        return out;
    };
    out.remove(from);
    let to = to.min(out.len());
    out.insert(to, id);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lamina_core::{AlbumSpec, Dpi, Mm, Project};

    fn project_with(names: &[&str]) -> Project {
        let spec =
            AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        for (i, n) in names.iter().enumerate() {
            project.add_photo(
                format!("{n}.jpg"),
                100 + u32::try_from(i).unwrap_or(u32::MAX),
                100,
            );
        }
        project
    }

    #[test]
    fn of_nome_alfabetica() {
        let project = project_with(&["b", "a", "c"]);
        let ids: Vec<PhotoId> = sorted(project.photos.iter(), SortBy::Name, &[])
            .into_iter()
            .map(|p| p.id)
            .collect();
        let names: Vec<&str> = ids
            .iter()
            .map(|id| {
                project
                    .photo(*id)
                    .unwrap()
                    .path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
            })
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn of_personalizada_move() {
        let project = project_with(&["a", "b", "c", "d"]);
        let capture: Vec<PhotoId> = project.photos.iter().map(|p| p.id).collect();
        // Mover a 4ª para a 1ª posição.
        let order = move_to(&[], &capture, capture[3], 0);
        assert_eq!(order[0], capture[3]);
        // Mover de volta.
        let order = move_to(&order, &capture, capture[3], 3);
        assert_eq!(order, capture);
    }

    #[test]
    fn of_b6_montagem_pela_janela_segue_o_criterio() {
        // Com o critério "Nome do arquivo", a montagem pela janela distribui as fotos em ordem
        // alfabética, então o conjunto de fotos da primeira lâmina difere da linha de comando
        // (que distribui na ordem de captura, regra 12).
        use lamina_layout::{BuildOptions, auto_build};
        let spec =
            AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        // 4 fotos com orientações diferentes e datas de captura invertidas em relação ao nome:
        // a captura (por data) é [d, c, b, a] e a alfabética é [a, b, c, d]. A distribuição por
        // orientação muda o conjunto de fotos da primeira lâmina.
        let mut add = |name: &str, w: u32, h: u32, secs: i64| {
            let id = project.add_photo(format!("{name}.jpg"), w, h);
            project
                .photos
                .iter_mut()
                .find(|p| p.id == id)
                .unwrap()
                .taken_at = Some(lamina_core::CaptureTime::from_seconds(secs));
        };
        add("d", 100, 60, 1000);
        add("c", 60, 100, 2000);
        add("b", 100, 60, 3000);
        add("a", 60, 100, 4000);
        let by_name: Vec<PhotoId> = sorted(project.photos.iter(), SortBy::Name, &[])
            .into_iter()
            .map(|p| p.id)
            .collect();
        let window = auto_build(
            &project,
            2,
            &BuildOptions {
                photo_order: by_name,
                ..BuildOptions::default()
            },
        )
        .unwrap();
        let cli = auto_build(&project, 2, &BuildOptions::default()).unwrap();
        // O conjunto de fotos da primeira lâmina.
        let set_of = |p: &Project| {
            let mut ids: Vec<PhotoId> = p.spreads[0]
                .frames
                .iter()
                .filter_map(|f| match &f.content {
                    lamina_core::FrameContent::Photo { photo, .. } => Some(*photo),
                    _ => None,
                })
                .collect();
            ids.sort();
            ids
        };
        assert_ne!(
            set_of(&window.project),
            set_of(&cli.project),
            "a janela (alfabética) distribui diferente da linha de comando (captura)"
        );
        // A linha de comando é determinística: duas montagens iguais.
        let cli2 = auto_build(&project, 2, &BuildOptions::default()).unwrap();
        assert_eq!(
            set_of(&cli.project),
            set_of(&cli2.project),
            "a linha de comando não muda"
        );
    }

    #[test]
    fn of_personalizada_respeita_a_ordem() {
        let project = project_with(&["a", "b", "c"]);
        let capture: Vec<PhotoId> = project.photos.iter().map(|p| p.id).collect();
        let custom = vec![capture[2], capture[0], capture[1]];
        let got: Vec<PhotoId> = sorted(project.photos.iter(), SortBy::Custom, &custom)
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(got, custom);
    }
}
