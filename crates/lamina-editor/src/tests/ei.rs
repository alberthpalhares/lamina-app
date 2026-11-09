//! Cenários da spec `exportar-pela-janela` (EI) que rodam sem janela: exportar pela janela
//! (B1 a B3) e reencontrar fotos (B5 a B8). B4 e B9 moram no `lamina-export`; B10 no
//! `lamina-core`; B11 nos testes do binário `lamina`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_export::{ExportOptions, WarningKind, export, open_project};
use lamina_media::original_source;

use super::common::{Base, file_name};
use crate::{
    Editor, export_progress, export_project, export_summary, not_found_message, relinked_message,
    search_folder, search_near,
};

fn opened(file: &Path) -> Editor {
    let mut editor = Editor::new();
    editor.open(file).unwrap();
    editor
}

fn sorted_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    files
}

#[test]
fn ei_b1_exportar_pela_janela_e_igual_ao_lamina_exportar() {
    let base = Base::with_photos(12, (600, 400));
    let file = base.save(base.project(3), "album.lamina");
    let editor = opened(&file);
    let (janela, linha) = (base.path("janela"), base.path("linha"));
    let options = ExportOptions::default();

    let report = export_project(
        editor.session().unwrap().project(),
        &janela,
        &options,
        &|_, _| {},
    )
    .unwrap();
    export(
        &open_project(&file).unwrap(),
        &linha,
        &original_source(),
        &options,
        &|_, _| {},
    )
    .unwrap();

    let (a, b) = (sorted_files(&janela), sorted_files(&linha));
    assert_eq!(a.len(), 3);
    assert_eq!(
        a.iter().map(|p| file_name(p)).collect::<Vec<_>>(),
        b.iter().map(|p| file_name(p)).collect::<Vec<_>>()
    );
    for (x, y) in a.iter().zip(&b) {
        assert!(
            std::fs::read(x).unwrap() == std::fs::read(y).unwrap(),
            "{x:?} difere"
        );
    }
    let summary = export_summary(&report, &janela);
    assert!(
        summary.starts_with(&format!("3 arquivos em {} (", janela.display())),
        "{summary}"
    );
}

/// Três lâminas: a foto do primeiro quadro da lâmina 2 fica ampliada (medidas pequenas no
/// projeto) e o primeiro quadro da lâmina 3 fica vazio.
fn with_two_warnings(base: &Base) -> (Project, PhotoId) {
    let mut project = base.project(3);
    let FrameContent::Photo { photo, .. } = project.spreads[1].frames[0].content else {
        panic!("o primeiro quadro da lâmina 2 devia ter foto");
    };
    let small = project.photos.iter_mut().find(|p| p.id == photo).unwrap();
    small.width_px /= 20;
    small.height_px /= 20;
    project.spreads[2].frames[0].content = FrameContent::Empty;
    (project, photo)
}

#[test]
fn ei_b2_pre_impressao_com_ir_para_a_lamina_e_ignorar() {
    let base = Base::with_photos(12, (600, 400));
    let (project, _) = with_two_warnings(&base);
    let mut editor = Editor::new();
    editor.adopt(project);
    let folder = base.path("saida");
    let session = editor.session().unwrap();
    let mut check = session
        .check_export(&folder, &ExportOptions::default())
        .unwrap();

    // Com os modelos de fábrica (spec RM, beta 9.1), a montagem pode trazer avisos NearCut
    // próprios; os construídos aqui são a foto ampliada na lâmina 3 e o quadro vazio na 4.
    let kinds: Vec<(usize, usize, bool)> = check
        .warnings
        .iter()
        .map(|w| {
            let enlarged = matches!(w.kind, WarningKind::Enlarged { .. });
            (w.spread, w.frame, enlarged)
        })
        .collect();
    assert!(
        kinds.contains(&(2, 0, true)) && kinds.contains(&(3, 0, false)),
        "{:?}",
        check.warnings
    );
    assert!(check.existing.is_empty());

    let before = check.warnings.len();
    assert!(check.ignore(1), "ignorar o quadro vazio");
    assert_eq!(check.warnings.len(), before - 1);
    assert!(!check.ignore(5));

    // "Ir para a lâmina" leva ao aviso da foto ampliada (lâmina 3, quadro 1).
    let warning = check
        .warnings
        .iter()
        .find(|w| matches!(w.kind, WarningKind::Enlarged { .. }))
        .expect("o aviso da foto ampliada")
        .clone();
    assert!(editor.go_to_warning(&warning));
    // `Warning::spread` é 1-based: ir para a lâmina dele deixa a atual em spread − 1.
    assert_eq!(editor.session().unwrap().current(), warning.spread - 1);
    assert_eq!(editor.selected_frame(), Some(warning.frame));
    assert!(!folder.exists(), "nada foi exportado");
}

#[test]
fn ei_b2_arquivos_que_ja_existem_aparecem_para_a_pergunta() {
    let base = Base::with_photos(12, (600, 400));
    let mut editor = Editor::new();
    editor.adopt(base.project(3));
    let folder = base.path("saida");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("lamina-02.jpg"), b"antigo").unwrap();
    let check = editor
        .session()
        .unwrap()
        .check_export(&folder, &ExportOptions::default())
        .unwrap();
    assert_eq!(check.existing, ["lamina-02.jpg"]);
}

#[test]
fn ei_b3_andamento_cresce_ate_o_total() {
    let base = Base::with_photos(12, (300, 200));
    let project = base.project(6);
    let seen = RefCell::new(Vec::new());
    let progress = |done: usize, total: usize| seen.borrow_mut().push((done, total));
    export_project(
        &project,
        &base.path("saida"),
        &ExportOptions::default(),
        &progress,
    )
    .unwrap();
    let seen = seen.into_inner();
    let expected: Vec<(usize, usize)> = (1..=6).map(|n| (n, 6)).collect();
    assert_eq!(seen, expected);
    assert_eq!(export_progress(6, 6), "Exportando 6 de 6…");
}

/// O id da foto de nome `name`.
fn id_of(project: &Project, name: &str) -> PhotoId {
    project
        .photos
        .iter()
        .find(|p| file_name(&p.path) == name)
        .unwrap()
        .id
}

fn photo_path(editor: &Editor, id: PhotoId) -> PathBuf {
    editor
        .session()
        .unwrap()
        .project()
        .photo(id)
        .unwrap()
        .path
        .clone()
}

#[test]
fn ei_b5_reencontrar_pelo_nome_na_pasta_do_projeto() {
    let base = Base::with_photos(12, (300, 200));
    let file = base.save(base.project(3), "album.lamina");
    std::fs::create_dir_all(base.path("fotos/extras")).unwrap();
    std::fs::create_dir_all(base.path("fotos/copias")).unwrap();
    std::fs::rename(
        base.path("fotos/f03.jpg"),
        base.path("fotos/extras/f03.jpg"),
    )
    .unwrap();
    // Mesmo nome, outro conteúdo: não serve.
    std::fs::copy(
        base.path("fotos/f04.jpg"),
        base.path("fotos/copias/f03.jpg"),
    )
    .unwrap();

    let mut editor = opened(&file);
    let session = editor.session_mut().unwrap();
    let id = id_of(session.project(), "f03.jpg");
    assert_eq!(session.missing(), [id]);
    let found = search_near(
        session.project(),
        session.missing(),
        session.file(),
        &base.cache(),
    );
    assert_eq!(session.relink(&found), 1);
    assert!(session.missing().is_empty());
    assert!(session.is_dirty(), "reencontrar é uma alteração não salva");
    assert_eq!(relinked_message(1, 0).unwrap(), "1 foto reencontrada.");
    assert_eq!(photo_path(&editor, id), base.path("fotos/extras/f03.jpg"));
}

#[test]
fn ei_b5_desfazer_volta_a_foto_para_as_ausentes() {
    let base = Base::with_photos(12, (300, 200));
    let file = base.save(base.project(3), "album.lamina");
    std::fs::create_dir_all(base.path("fotos/extras")).unwrap();
    std::fs::rename(
        base.path("fotos/f03.jpg"),
        base.path("fotos/extras/f03.jpg"),
    )
    .unwrap();
    let mut editor = opened(&file);
    let session = editor.session_mut().unwrap();
    let found = search_near(
        session.project(),
        session.missing(),
        session.file(),
        &base.cache(),
    );
    session.relink(&found);
    assert_eq!(session.undo().unwrap(), "Reencontrar fotos · 1 foto");
    assert_eq!(session.missing().len(), 1);
    assert_eq!(
        relinked_message(3, 2).unwrap(),
        "3 fotos reencontradas; 2 ainda faltam."
    );
}

#[test]
fn ei_b6_reencontrar_pelo_catalogo() {
    let base = Base::with_photos(12, (300, 200));
    let file = base.save(base.project(3), "projeto/album.lamina");
    std::fs::create_dir_all(base.path("outra")).unwrap();
    std::fs::copy(base.path("fotos/f05.jpg"), base.path("outra/f05.jpg")).unwrap();
    let cache = base.cache();
    let catalog = cache.open_catalog().unwrap();
    lamina_media::import_folder(&base.path("outra"), &catalog, &cache.thumbs_dir()).unwrap();
    drop(catalog);
    std::fs::remove_file(base.path("fotos/f05.jpg")).unwrap();

    let mut editor = opened(&file);
    let session = editor.session_mut().unwrap();
    let id = id_of(session.project(), "f05.jpg");
    let found = search_near(session.project(), session.missing(), session.file(), &cache);
    assert_eq!(session.relink(&found), 1);
    assert_eq!(photo_path(&editor, id), base.path("outra/f05.jpg"));
}

#[test]
fn ei_b7_procurar_numa_pasta_escolhida() {
    let base = Base::with_photos(12, (300, 200));
    let file = base.save(base.project(3), "album.lamina");
    std::fs::create_dir_all(base.path("longe")).unwrap();
    std::fs::create_dir_all(base.path("vazia")).unwrap();
    std::fs::rename(base.path("fotos/f01.jpg"), base.path("longe/primeira.jpg")).unwrap();
    std::fs::rename(base.path("fotos/f02.jpg"), base.path("longe/segunda.jpg")).unwrap();
    std::fs::copy(base.path("fotos/f06.jpg"), base.path("vazia/outra.jpg")).unwrap();

    let mut editor = opened(&file);
    let session = editor.session_mut().unwrap();
    assert_eq!(session.missing().len(), 2);
    let (a, b) = (
        id_of(session.project(), "f01.jpg"),
        id_of(session.project(), "f02.jpg"),
    );
    let empty = base.path("vazia");
    let none = search_folder(session.project(), session.missing(), &empty, &|_, _| {});
    assert!(none.is_empty());
    assert_eq!(
        not_found_message(2, &empty),
        format!(
            "Nenhuma das 2 fotos ausentes foi encontrada em {}.",
            empty.display()
        )
    );
    let seen = RefCell::new(Vec::new());
    let found = search_folder(
        session.project(),
        session.missing(),
        &base.path("longe"),
        &|done, total| seen.borrow_mut().push((done, total)),
    );
    assert_eq!(seen.into_inner(), [(1, 2), (2, 2)]);
    assert_eq!(session.relink(&found), 2);
    assert!(session.missing().is_empty());
    assert_eq!(photo_path(&editor, a), base.path("longe/primeira.jpg"));
    assert_eq!(photo_path(&editor, b), base.path("longe/segunda.jpg"));
}

#[test]
fn ei_b8_medidas_relidas_ao_reencontrar() {
    let base = Base::with_photos(12, (60, 40));
    let mut project = base.project(3);
    let id = id_of(&project, "f00.jpg");
    let photo = project.photos.iter_mut().find(|p| p.id == id).unwrap();
    photo.width_px *= 20;
    photo.height_px *= 20;
    let file = base.save(project, "album.lamina");
    std::fs::create_dir_all(base.path("longe")).unwrap();
    std::fs::rename(base.path("fotos/f00.jpg"), base.path("longe/x.jpg")).unwrap();

    let mut editor = opened(&file);
    let session = editor.session_mut().unwrap();
    let enlarged = |s: &crate::Session| {
        (0..s.spread_count()).any(|i| {
            s.warnings_for(i).iter().any(
                |w| matches!(&w.kind, WarningKind::Enlarged { photo, .. } if photo == "f00.jpg" || photo == "x.jpg"),
            )
        })
    };
    assert!(!enlarged(session), "antes: medidas grandes, sem aviso");
    let found = search_folder(
        session.project(),
        session.missing(),
        &base.path("longe"),
        &|_, _| {},
    );
    assert_eq!(session.relink(&found), 1);
    let photo = session.project().photo(id).unwrap();
    assert_eq!((photo.width_px, photo.height_px), (60, 40));
    assert!(
        enlarged(session),
        "depois: a pré-impressão avisa a ampliação"
    );
}
