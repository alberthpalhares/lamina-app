//! Cenários da spec `album-em-branco` (BR): o álbum começa sem lâminas e "Montar o álbum…" monta
//! as fotos não usadas. Os gestos de uma lâmina, a IA e a velocidade estão em `br_mais.rs` e
//! `br_rapido.rs`.

use lamina_core::{
    AlbumEdge, AlbumLook, Border, FrameContent, Mm, PageLimits, PhotoId, Project, Rgb,
};
use lamina_layout::{BuildOptions, Reuse, auto_build};

use super::common::Base;
use super::tl_common::{H, built, editor_at, spec};
use crate::building::{ALL_USED, NO_PHOTOS};
use crate::new_project::{AlbumStyle, apply_look, put_style};
use crate::{Command, Editor, Filter, NewAlbum, Session, create_album, import_project, photo_list};

/// Fundo cinza, espaço de 4 mm, borda de 0,5 mm branca e cantos de 2 mm.
pub fn style() -> AlbumStyle {
    AlbumStyle {
        look: AlbumLook {
            background: Rgb {
                r: 128,
                g: 128,
                b: 128,
            },
            border: Border {
                width: Mm(0.5),
                color: Rgb::WHITE,
            },
            radius: Mm(2.0),
        },
        gap: Mm(4.0),
        ..AlbumStyle::default()
    }
}

/// O pedido do Novo álbum com as fotos da base, a aparência de [`style`] e a caixa "Montar as
/// lâminas agora" desmarcada.
pub fn blank_request(base: &Base) -> NewAlbum {
    NewAlbum {
        build: false,
        style: style(),
        ..base.new_album(None)
    }
}

/// O álbum em branco da base (BR-B1).
pub fn blank(base: &Base) -> Project {
    create_album(&blank_request(base), &base.cache(), &|_| {})
        .unwrap()
        .0
        .project
}

/// As fotos de cada lâmina, em ordem.
pub fn photos_by_spread(project: &Project) -> Vec<Vec<PhotoId>> {
    (0..project.spreads.len())
        .map(|i| crate::edits::spread_photos(project, i))
        .collect()
}

fn last_label(editor: &Editor) -> String {
    let history = editor.session().unwrap().history_list();
    history.steps.last().unwrap().label.clone()
}

#[test]
fn br_b1_criar_com_fotos_sem_montar() {
    let base = Base::with_photos(40, (600, 400));
    let project = blank(&base);
    assert_eq!((project.photos.len(), project.spreads.len()), (40, 0));
    assert_eq!(project.look, style().look);
    assert_eq!(project.gap, Mm(4.0));
    let mut editor = Editor::new();
    let file = editor
        .adopt_saved(project, &base.path("albuns/branco.lamina"))
        .unwrap();
    assert!(file.is_file());
    let session = editor.session().unwrap();
    assert_eq!(photo_list(session.project(), Filter::Unused).len(), 40);
    assert!(session.history_list().steps.is_empty());
}

#[test]
fn br_b2_com_a_caixa_marcada_como_antes() {
    let base = Base::with_photos(40, (600, 400));
    let new = NewAlbum {
        build: true,
        spreads: Some(10),
        ..blank_request(&base)
    };
    let (now, _) = create_album(&new, &base.cache(), &|_| {}).unwrap();
    // O caminho de antes da BR: importar, aparência, montagem automática e a aparência nas lâminas.
    let mut imported = import_project(
        &new.folder,
        new.name.as_deref(),
        new.spec.clone(),
        &base.cache(),
        true,
        &|_| {},
    )
    .unwrap();
    put_style(&mut imported.project, &new.style);
    let options = BuildOptions {
        reuse: new.reuse,
        ..BuildOptions::default()
    };
    let mut before = auto_build(&imported.project, 10, &options).unwrap();
    apply_look(&mut before.project);
    let a = Session::new_unsaved(now.project)
        .save_as(&base.path("agora.lamina"))
        .unwrap();
    let b = Session::new_unsaved(before.project)
        .save_as(&base.path("antes.lamina"))
        .unwrap();
    assert!(std::fs::read(a).unwrap() == std::fs::read(b).unwrap());
}

#[test]
fn br_b3_montar_o_album_em_branco() {
    let base = Base::with_photos(40, (600, 400));
    let new = NewAlbum {
        build: true,
        spreads: Some(10),
        ..blank_request(&base)
    };
    let (montado, _) = create_album(&new, &base.cache(), &|_| {}).unwrap();
    let mut editor = Editor::new();
    editor.adopt(blank(&base));
    let shown: Vec<PhotoId> = editor
        .session()
        .unwrap()
        .project()
        .photos
        .iter()
        .map(|p| p.id)
        .collect();
    editor.set_selection(shown[..3].to_vec());
    let plan = editor.album_build().unwrap();
    assert_eq!(plan.suggestion, 10);
    assert_eq!(plan.what(), "As 40 fotos do álbum viram lâminas.");
    editor.build_album(None, Reuse::Medium).unwrap();
    let session = editor.session().unwrap();
    assert_eq!(session.project().spreads, montado.project.spreads);
    assert_eq!(
        last_label(&editor),
        "Montar 10 lâminas com 40 fotos · álbum"
    );
    assert_eq!(session.current(), 0);
    assert!(editor.selection().is_empty());
}

/// 36 fotos sem arquivo; as de números 5 a 20 em 4 lâminas feitas à mão.
fn four_by_hand() -> (Editor, Vec<PhotoId>) {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    let ids: Vec<PhotoId> = (0..36)
        .map(|i| project.add_photo(format!("f{i:02}.jpg"), H.0, H.1))
        .collect();
    let mut editor = Editor::new();
    editor.adopt(project);
    for k in 0..4 {
        let four = &ids[4 + 4 * k..8 + 4 * k];
        assert!(editor.drop_photos(crate::DropPlace::NewSpread(k), four));
    }
    (editor, ids)
}

#[test]
fn br_b4_com_laminas_so_as_nao_usadas_no_fim() {
    let (mut editor, ids) = four_by_hand();
    let before = editor.session().unwrap().project().spreads.clone();
    let plan = editor.album_build().unwrap();
    assert_eq!((plan.suggestion, plan.kept), (5, 4));
    assert_eq!(
        plan.what(),
        "20 fotos ainda não usadas viram lâminas novas, depois da lâmina 4."
    );
    editor.build_album(None, Reuse::Medium).unwrap();
    let session = editor.session().unwrap();
    let project = session.project();
    assert_eq!(project.spreads[..4], before[..]);
    let new: Vec<PhotoId> = photos_by_spread(project)[4..].concat();
    let unused: Vec<PhotoId> = ids[..4].iter().chain(&ids[20..]).copied().collect();
    assert_eq!(new, unused, "as 20, na ordem de captura, uma vez cada");
    assert_eq!(project.spreads.len(), 9);
    project.validate().unwrap();
    let mut seen = std::collections::HashSet::new();
    let frames = project.spreads.iter().flat_map(|s| &s.frames);
    for id in project
        .spreads
        .iter()
        .map(|s| s.id.get())
        .chain(frames.map(|f| f.id.get()))
    {
        assert!(seen.insert(id), "id {id} repetido");
    }
    assert_eq!(session.current(), 4);
}

/// Um álbum que termina com página simples: 3 lâminas (a 3 só com a página esquerda) e 10 fotos
/// não usadas.
fn single_end() -> Editor {
    let s = spec(30.0, 30.0).with_edges(AlbumEdge::FullSpread, AlbumEdge::SinglePage);
    let mut project = built(s, &[H; 12], 3);
    for i in 0..10 {
        project.add_photo(format!("g{i:02}.jpg"), H.0, H.1);
    }
    editor_at(project, 0)
}

/// Lados da lâmina `index` que têm quadro: (esquerda, direita).
fn sides(project: &Project, index: usize) -> (bool, bool) {
    let frames = &project.spreads[index].frames;
    let left = frames.iter().any(|f| f.rect.x() < 0.5 - 1e-6);
    let right = frames
        .iter()
        .any(|f| f.rect.x() + f.rect.width() > 0.5 + 1e-6);
    (left, right)
}

#[test]
fn br_b5_pagina_simples_no_fim() {
    let mut editor = single_end();
    assert_eq!(sides(editor.session().unwrap().project(), 2), (true, false));
    editor.build_album(Some(2), Reuse::Medium).unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!(project.spreads.len(), 5);
    assert_eq!(
        sides(project, 3),
        (true, true),
        "a lâmina 4 usa as duas páginas"
    );
    assert_eq!(
        sides(project, 4),
        (true, false),
        "a lâmina 5 só usa a esquerda"
    );
    assert!(
        sides(project, 2).1,
        "a antiga última ganhou a página da direita"
    );
    let photos = photos_by_spread(project);
    assert_eq!(photos[2].len(), 4, "rearranjada com as mesmas fotos");
}

#[test]
fn br_b6_ctrl_z_desfaz_a_montagem_inteira() {
    let mut editor = single_end();
    let before = editor.session().unwrap().project().clone();
    editor.build_album(Some(2), Reuse::Medium).unwrap();
    let after = editor.session().unwrap().project().clone();
    assert!(editor.run(Command::Undo));
    assert_eq!(editor.session().unwrap().project(), &before);
    assert!(editor.run(Command::Redo));
    assert_eq!(editor.session().unwrap().project(), &after);
}

#[test]
fn br_b7_quando_montar_o_album_pode() {
    let full = editor_at(built(spec(30.0, 30.0), &[H; 12], 3), 0);
    assert!(!full.available(Command::BuildAlbum));
    assert_eq!(full.album_build(), Err(ALL_USED));
    let mut none = Editor::new();
    none.adopt(Project::new("vazio", spec(30.0, 30.0)));
    assert!(!none.available(Command::BuildAlbum));
    assert_eq!(none.album_build(), Err(NO_PHOTOS));
    assert!(!Editor::new().available(Command::BuildAlbum));
    let mut organizing = single_end();
    assert!(organizing.available(Command::BuildAlbum));
    assert!(organizing.run(Command::Organize));
    assert!(!organizing.available(Command::BuildAlbum));
    let mut busy = single_end();
    busy.set_busy(true);
    assert!(!busy.available(Command::BuildAlbum));
    // Pelo menu, no primeiro: a mensagem, e nada muda.
    let mut full = full;
    let before = full.session().unwrap().project().clone();
    assert!(!full.run(Command::BuildAlbum));
    assert_eq!(full.session().unwrap().project(), &before);
}

#[test]
fn br_b8_o_numero_de_laminas_e_o_resultado() {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    for i in 0..48 {
        project.add_photo(format!("f{i:02}.jpg"), H.0, H.1);
    }
    project.page_limits = PageLimits {
        min: Some(30),
        max: None,
    };
    let plan = crate::building::album_build(&project).unwrap();
    let message = |n| plan.check(&project, n).unwrap_err().to_string();
    assert_eq!(
        message(3),
        "São 48 fotos e cabem 24 em 3 lâminas: use pelo menos 6 lâminas."
    );
    assert_eq!(
        message(60),
        "Há 48 fotos para 60 lâminas: use no máximo 48 lâminas."
    );
    assert!(matches!(plan.check(&project, 12), Ok(())));
    let (line, warning) = plan.result(&project, 12);
    assert_eq!(line, "O álbum fica com 12 lâminas (24 páginas).");
    assert_eq!(
        warning.as_deref(),
        Some("O álbum tem 24 páginas; o mínimo é 30.")
    );
}

#[test]
fn br_aparencia_vem_do_album_na_lamina_sem_fotos() {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    put_style(&mut project, &style());
    let ids: Vec<PhotoId> = (0..3)
        .map(|i| project.add_photo(format!("f{i}.jpg"), H.0, H.1))
        .collect();
    project.add_spread();
    crate::edits::layout_with(&mut project, 0, &ids).unwrap();
    let frame = &project.spreads[0].frames[0];
    assert!(matches!(frame.content, FrameContent::Photo { .. }));
    assert_eq!((frame.border, frame.radius), (style().look.border, Mm(2.0)));
    assert_eq!(
        crate::edits::photo_style(&project, 0),
        (style().look.border, Mm(2.0))
    );
}
