//! Cenários da spec `capa` (CA) com a capa aberta: o que olha o álbum inteiro não pode ler a
//! projeção, que só tem a capa (correção C4).

use lamina_core::{CoverMeasures, PhotoId, Project};
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use super::tl_common::{built, editor_at, spec, three_spreads};
use crate::{DropPlace, Editor, Session};

fn on_cover(project: Project) -> Editor {
    let mut editor = editor_at(project, 0);
    let measures = CoverMeasures::defaults(&editor.session().unwrap().album().spec);
    assert!(editor.create_cover(measures, None, &[]));
    assert!(editor.on_cover());
    editor
}

fn album(editor: &Editor) -> &Project {
    editor.session().unwrap().album()
}

fn first_photos(project: &Project, spread: usize) -> Vec<PhotoId> {
    crate::edits::spread_photos(project, spread)
}

#[test]
fn ca_b29_recentes_e_miniatura_com_a_capa_aberta() {
    let dir = tempfile::tempdir().unwrap();
    let project = built(spec(30.0, 30.0), &[super::tl_common::H; 48], 12);
    let file = Session::new_unsaved(project)
        .save_as(&dir.path().join("casamento.lamina"))
        .unwrap();
    let base = Base::with_photos(1, (600, 400));
    let mut editor = Editor::new();
    let store = crate::recents::RecentsStore::load(&base.path("config/recentes.json"));
    editor.set_recents(store);
    editor.open(&file).unwrap();
    assert!(editor.go_to_spread(4));
    let measures = CoverMeasures::defaults(&album(&editor).spec);
    assert!(editor.create_cover(measures, None, &[]));
    assert!(editor.on_cover());
    editor.save_to(None).unwrap();
    let saved = &editor.recents().list()[0];
    assert_eq!(saved.spreads, 12, "as lâminas do álbum, não as da projeção");
    assert_eq!(saved.current, 4, "a lâmina em que parou");
    let (shown, _) = editor.thumbnail_job().unwrap();
    assert_eq!(shown.spreads.len(), 12, "a miniatura é a do álbum");
    assert!(shown.cover.is_some());
    assert!(editor.close_album());
    assert_eq!(editor.recents().list()[0].current, 4);
}

#[test]
fn ca_b30_mexer_nas_laminas_pela_faixa_com_a_capa_aberta() {
    let project = three_spreads();
    let first = project.spreads[0].id;
    let mut editor = on_cover(project);
    assert!(
        editor.move_spread_to_gap(0, 3),
        "arrastar para o fim da faixa"
    );
    assert!(!editor.on_cover());
    assert_eq!(album(&editor).spreads[2].id, first);

    let project = three_spreads();
    let photos = first_photos(&project, 0);
    let mut editor = on_cover(project);
    let at = editor.album_end();
    assert_eq!(at, 3, "a lâmina nova entra no fim do álbum");
    assert!(editor.drop_photos(DropPlace::NewSpread(at), &photos[..1]));
    assert_eq!(album(&editor).spreads.len(), 4);
    assert_eq!(first_photos(album(&editor), 3), photos[..1]);

    let mut editor = on_cover(three_spreads());
    let before = first_photos(album(&editor), 0).len();
    assert!(editor.drag_cut(0, -1), "o corte é entre lâminas do álbum");
    assert!(!editor.on_cover());
    assert_eq!(first_photos(album(&editor), 0).len(), before - 1);
}

#[test]
fn ca_b31_a_sequencia_mostra_o_album_com_a_capa_aberta() {
    let project = three_spreads();
    let rows = crate::motion::timeline_rows(&project).len();
    let editor = on_cover(project);
    let (shown, listed) = editor.sequence_rows().unwrap();
    assert_eq!(shown.spreads.len(), 3);
    assert_eq!(listed.len(), rows);
}

#[test]
fn ca_b32_soltar_fora_dos_quadros_nao_usa_o_motor() {
    let project = three_spreads();
    let photos = first_photos(&project, 0);
    let mut editor = on_cover(project);
    let steps = editor.session().unwrap().history_list().steps.len();
    assert!(!editor.drop_photos(DropPlace::Spread(0), &photos[..1]));
    let cover = album(&editor).cover.as_ref().unwrap();
    assert!(
        cover.content.frames.is_empty(),
        "quem decide é a área solta"
    );
    assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
    assert!(editor.drop_on_cover(0.9, &photos[..1]));
    let cover = album(&editor).cover.as_ref().unwrap();
    assert_eq!(cover.content.frames.len(), 1);
}

#[test]
fn ca_b33_a_ia_ve_os_avisos_da_capa_e_das_laminas() {
    let base = Base::with_photos(8, (600, 400));
    let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], true);
    lab.ok(
        "novo_album",
        json!({
            "pasta_fotos": base.path("fotos"),
            "salvar_em": base.dir.path().join("ca.lamina"),
            "montar": true,
            "laminas": 2,
        }),
    );
    lab.ok("criar_capa", json!({}));
    lab.ok(
        "colocar_foto",
        json!({ "capa": true, "quadro": 1, "foto": 3 }),
    );
    let reply = lab.ok("pre_impressao", json!({}));
    let warnings = reply.data["avisos"].as_array().unwrap();
    let says = |start: &str| {
        warnings
            .iter()
            .any(|w| w["aviso"].as_str().unwrap().starts_with(start))
    };
    assert!(says("Capa:"), "{warnings:?}");
    assert!(says("Lâmina 1:"), "{warnings:?}");
    let on_cover = warnings
        .iter()
        .find(|w| w["aviso"].as_str().unwrap().starts_with("Capa:"))
        .unwrap();
    assert_eq!(on_cover["capa"], true, "{on_cover}");
    assert!(on_cover.get("lamina").is_none(), "{on_cover}");
}
