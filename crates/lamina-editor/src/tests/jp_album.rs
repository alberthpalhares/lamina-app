//! Novo álbum, navegador de fotos e avisos (JP-B5, B8, B10).

use std::sync::Mutex;

use lamina_core::{FrameContent, PhotoId};
use lamina_export::WarningKind;

use super::common::Base;
use crate::{
    Editor, EditorError, Filter, Session, Step, check_spreads, count_photos, create_album,
    photo_info, photo_list, suggestion_for, suggestion_text,
};

#[test]
fn jp_b5_andamento_do_trabalho_longo() {
    let base = Base::with_photos(30, (300, 200));
    let steps = Mutex::new(Vec::new());
    let (built, failures) = create_album(&base.new_album(None), &base.cache(), &|s| {
        steps.lock().unwrap().push(s);
    })
    .unwrap();
    assert!(failures.is_empty());
    let steps = steps.into_inner().unwrap();
    let counts: Vec<usize> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Importing { done, total: 30 } => Some(*done),
            _ => None,
        })
        .collect();
    assert!(counts.windows(2).all(|w| w[0] < w[1]), "{counts:?}");
    assert_eq!(counts.last(), Some(&30));
    assert_eq!(steps.last(), Some(&Step::Building));
    assert_eq!(built.project.spreads.len(), 8, "sugestão: 30 ÷ 4");
}

#[test]
fn sugestao_e_erros_da_pasta() {
    assert_eq!(
        suggestion_text(3),
        "Sugestão: 3 lâminas (cerca de 4 fotos por lâmina)."
    );
    assert_eq!(
        suggestion_text(1),
        "Sugestão: 1 lâmina (cerca de 4 fotos por lâmina)."
    );
    let base = Base::with_photos(0, (10, 10));
    let empty = create_album(&base.new_album(None), &base.cache(), &|_| {});
    assert!(matches!(empty, Err(EditorError::NoPhotos(_))));
    let mut gone = base.new_album(None);
    gone.folder = base.path("nao-existe");
    let err = create_album(&gone, &base.cache(), &|_| {}).unwrap_err();
    assert!(matches!(err, EditorError::MissingFolder(_)));
    assert!(err.to_string().starts_with("A pasta "), "{err}");
}

#[test]
fn novo_album_confere_antes_de_montar() {
    let base = Base::with_photos(12, (60, 40));
    std::fs::write(base.path("fotos/notas.txt"), "x").unwrap();
    assert_eq!(count_photos(&base.path("fotos")).unwrap(), 12);
    assert!(matches!(
        count_photos(&base.path("nao-existe")),
        Err(EditorError::MissingFolder(_))
    ));
    let spec = base.new_album(None).spec;
    assert_eq!(suggestion_for(12, &spec), 3);
    assert_eq!(check_spreads(12, 3, &spec), Ok(()));
    let err = check_spreads(12, 20, &spec).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Há 12 fotos para 20 lâminas: use no máximo 12 lâminas."
    );
}

/// Quadros com a foto `id`.
fn frames_with(project: &lamina_core::Project, id: PhotoId) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for (s, spread) in project.spreads.iter().enumerate() {
        for (f, frame) in spread.frames.iter().enumerate() {
            if matches!(frame.content, FrameContent::Photo { photo, .. } if photo == id) {
                found.push((s, f));
            }
        }
    }
    found
}

#[test]
fn jp_b8_navegador_de_fotos() {
    let base = Base::with_photos(12, (600, 400));
    let mut project = base.project(3);
    let all = photo_list(&project, Filter::All);
    assert!(
        all.iter().all(|p| p.uses == 1),
        "a montagem automática usa cada foto uma vez"
    );
    // A primeira foto sai do quadro dela, que passa a mostrar a última (usada duas vezes).
    let (gone, twice) = (all[0].id, all[11].id);
    let (s, f) = frames_with(&project, gone)[0];
    let FrameContent::Photo { crop, .. } = project.spreads[s].frames[f].content.clone() else {
        panic!("quadro com foto");
    };
    project.spreads[s].frames[f].content = FrameContent::Photo { photo: twice, crop };

    let names = |filter| -> Vec<String> {
        photo_list(&project, filter)
            .into_iter()
            .map(|p| p.name)
            .collect()
    };
    let expected: Vec<String> = (0..12).map(|i| format!("f{i:02}.jpg")).collect();
    assert_eq!(
        names(Filter::All),
        expected,
        "ordem de captura (sem data: nome)"
    );
    assert_eq!(names(Filter::Used), expected[1..]);
    assert_eq!(names(Filter::Unused), ["f00.jpg"]);
    let mut uses: Vec<usize> = photo_list(&project, Filter::All)
        .iter()
        .map(|p| p.uses)
        .collect();
    uses.sort_unstable();
    uses.dedup();
    assert_eq!(uses, [0, 1, 2]);
    assert_eq!(photo_info(&project, twice).unwrap().uses, 2);

    let first = frames_with(&project, twice)
        .iter()
        .map(|(s, _)| *s)
        .min()
        .unwrap();
    let mut editor = Editor::new();
    editor.adopt(project);
    assert!(
        editor
            .session()
            .unwrap()
            .counter()
            .ends_with("11 de 12 fotos usadas")
    );
    assert!(editor.go_to_photo(twice));
    assert_eq!(editor.session().unwrap().current(), first);
    assert!(!editor.go_to_photo(gone), "foto não usada não tem lâmina");
}

#[test]
fn jp_b10_aviso_de_pre_impressao_na_faixa() {
    let base = Base::with_photos(12, (600, 400));
    let mut project = base.project(3);
    // Com os modelos de fábrica (spec RM, beta 9.1), a montagem pode já ter avisos de ampliação
    // (fotos pequenas em desenhos que preenchem mais); contamos os de antes para comparar.
    let clean = Session::new_unsaved(project.clone());
    let before: usize = (0..3).map(|i| clean.warnings_for(i).len()).sum();
    // Uma foto da lâmina 2 com 1/20 dos pixels: ampliada muito acima de 120%.
    let FrameContent::Photo { photo, .. } = project.spreads[1].frames[0].content else {
        panic!("quadro com foto");
    };
    let small = project.photos.iter_mut().find(|p| p.id == photo).unwrap();
    small.width_px /= 20;
    small.height_px /= 20;
    let file = base.save(project, "album/casamento.lamina");
    let session = Session::open(&file).unwrap();
    let warnings = session.warnings_for(1);
    let now = (0..3).map(|i| session.warnings_for(i).len()).sum::<usize>();
    assert!(
        now > before,
        "a foto ampliada demais aumenta os avisos ({before} → {now})"
    );
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w.kind, WarningKind::Enlarged { .. })),
        "{warnings:?}"
    );
}
