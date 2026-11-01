//! Cenários da spec `pagina-em-branco` (PB-B3 a PB-B10, B12 a B14) que rodam no editor; PB-B1 e B2
//! moram no `lamina-core`, e o PB-B11, no `lamina-export`.

use lamina_core::{AlbumEdge, BlankPages, CaptureTime, FrameContent, PageSide, PhotoId, Project};
use lamina_layout::BuildOptions;
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use super::tl_common::{H, built, editor_at, single_start, spec, three_spreads};
use crate::blank::set_blank;
use crate::building::build_unused;
use crate::editor::ContextEntry;
use crate::edits::{layout_with, spread_photos};
use crate::motion::{spread_badge, strip_rows};
use crate::view::{Area, spread_view};
use crate::{Command, DropPlace, Editor, Part};

fn last_label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps.last().unwrap().label.clone()
}

fn project(editor: &Editor) -> &Project {
    editor.session().unwrap().project()
}

/// Nenhum quadro de foto da lâmina passa da dobra para a esquerda.
fn photos_only_on_right(p: &Project, index: usize) -> bool {
    p.spreads[index]
        .frames
        .iter()
        .filter(|f| matches!(f.content, FrameContent::Photo { .. }))
        .all(|f| f.rect.x() >= 0.5 - 1e-6)
}

fn photo_count(p: &Project, index: usize) -> usize {
    spread_photos(p, index).len()
}

/// Muitas fotos horizontais: uma lâmina leva o que precisar.
fn photos(n: usize) -> Vec<(u32, u32)> {
    vec![H; n]
}

#[test]
fn pb_b3_comecar_com_uma_folha_em_branco() {
    let start = built(single_start(spec(30.0, 30.0)), &photos(24), 6);
    let on_first = spread_photos(&start, 0);
    let mut editor = editor_at(start.clone(), 0);
    assert!(editor.run(Command::NewBlankSpreadBefore));
    assert_eq!(last_label(&editor), "Nova lâmina em branco · lâmina 1");
    let p = project(&editor);
    assert_eq!(p.spreads.len(), 7);
    assert_eq!(p.spec.edges().page_count(7), 13);
    assert_eq!(
        p.spreads[0].blank,
        BlankPages {
            left: false,
            right: true
        }
    );
    assert!(p.spreads[0].frames.is_empty());
    let mut moved = spread_photos(p, 1);
    let mut expected = on_first;
    moved.sort();
    expected.sort();
    assert_eq!(moved, expected, "a antiga lâmina 1 leva as mesmas fotos");
    assert!(
        p.spreads[1].frames.iter().any(|f| f.rect.x() < 0.5),
        "agora ela tem as duas páginas"
    );
    assert!(editor.run(Command::Undo));
    assert_eq!(project(&editor), &start);
}

#[test]
fn pb_b4_marcar_uma_pagina_com_fotos() {
    let mut start = three_spreads();
    let all: Vec<PhotoId> = start.photos.iter().map(|p| p.id).take(5).collect();
    layout_with(&mut start, 2, &all).unwrap();
    assert!(
        !photos_only_on_right(&start, 2),
        "precisa ter foto na esquerda"
    );
    let mut editor = editor_at(start, 2);
    editor.context_menu_on_page(PageSide::Left);
    assert!(editor.run(Command::BlankPage));
    assert_eq!(last_label(&editor), "Página em branco · lâmina 3");
    let p = project(&editor);
    assert!(p.spreads[2].blank.left);
    assert!(photos_only_on_right(p, 2));
    assert_eq!(photo_count(p, 2), 5, "nenhuma voltou ao painel");
    assert_eq!(editor.take_notice(), None);
    assert!(editor.run(Command::BlankPage), "o mesmo item desmarca");
    assert_eq!(last_label(&editor), "Desmarcar página em branco · lâmina 3");
    let p = project(&editor);
    assert!(p.spreads[2].blank.is_none());
    assert_eq!(photo_count(p, 2), 5, "desmarcar não rearranja");
    assert!(photos_only_on_right(p, 2));
}

#[test]
fn pb_b5_sem_pagina_que_sobre() {
    // Na ponta com página simples, as 4 fotos não têm para onde ir.
    let start = built(single_start(spec(30.0, 30.0)), &photos(8), 2);
    let on_first = photo_count(&start, 0);
    assert!(on_first > 0);
    let mut editor = editor_at(start, 0);
    editor.context_menu_on_page(PageSide::Right);
    assert!(editor.run(Command::BlankPage));
    assert!(project(&editor).spreads[0].frames.is_empty());
    let notice = editor.take_notice().unwrap();
    let expected = if on_first == 1 {
        "1 foto voltou ao painel".to_owned()
    } else {
        format!("{on_first} fotos voltaram ao painel")
    };
    assert_eq!(notice, expected);
    assert!(editor.run(Command::Undo));
    assert_eq!(photo_count(project(&editor), 0), on_first);

    // Na lâmina de 20, ficam 12 e voltam as 8 últimas pela captura.
    let mut full = Project::new("teste", spec(30.0, 30.0));
    for i in 0..20u32 {
        let id = full.add_photo(format!("f{i:02}.jpg"), 3000, 2000);
        // A captura vai ao contrário dos identificadores, para distinguir as duas ordens.
        let minute = 59 - i;
        let taken = CaptureTime::from_exif(&format!("2026:09:12 10:{minute:02}:00"));
        full.photos
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap()
            .taken_at = taken;
    }
    full.add_spread();
    let all: Vec<PhotoId> = full.photos.iter().map(|p| p.id).collect();
    layout_with(&mut full, 0, &all).unwrap();
    let returned = set_blank(&mut full, 0, PageSide::Left, true).unwrap();
    assert_eq!(returned, 8);
    let kept = spread_photos(&full, 0);
    assert_eq!(kept.len(), 12);
    assert!(photos_only_on_right(&full, 0));
    // As 12 primeiras pela captura são as de identificador mais alto.
    assert!(kept.iter().all(|id| all[8..].contains(id)), "{kept:?}");
}

#[test]
fn pb_b6_o_motor_pula_a_pagina_em_branco() {
    let mut start = built(spec(30.0, 30.0), &photos(16), 3);
    set_blank(&mut start, 1, PageSide::Left, true).unwrap();
    let chosen = spread_photos(&start, 2)[..4].to_vec();
    let mut editor = editor_at(start, 1);
    editor.set_selection(chosen);
    assert!(editor.run(Command::BuildFromSelection));
    assert!(photos_only_on_right(project(&editor), 1));
    assert!(editor.run(Command::NextLayout));
    assert!(photos_only_on_right(project(&editor), 1));
    let panel = editor.panel(Part::Spread);
    assert!(!panel.options.is_empty());
    for option in &panel.options {
        assert!(option.frames.iter().all(|r| r.x() >= 0.5 - 1e-6));
    }
    assert!(project(&editor).spreads[1].blank.left);
}

#[test]
fn pb_b7_soltar_foto() {
    let mut start = built(spec(30.0, 30.0), &photos(30), 6);
    set_blank(&mut start, 1, PageSide::Left, true).unwrap();
    set_blank(&mut start, 4, PageSide::Left, true).unwrap();
    set_blank(&mut start, 4, PageSide::Right, true).unwrap();
    let outside = spread_photos(&start, 5)[0];
    let mut editor = editor_at(start.clone(), 1);
    assert!(editor.drop_photos(DropPlace::Spread(1), &[outside]));
    let p = project(&editor);
    assert!(spread_photos(p, 1).contains(&outside));
    assert!(photos_only_on_right(p, 1));

    let mut editor = editor_at(start.clone(), 4);
    assert!(!editor.drop_photos(DropPlace::Spread(4), &[outside]));
    assert_eq!(
        editor.take_notice().as_deref(),
        Some("Esta lâmina está em branco. Desmarque uma página para pôr fotos.")
    );
    assert_eq!(project(&editor), &start);

    // A partir de outra lâmina (as vizinhas do rolo e a faixa), a mesma recusa.
    let mut editor = editor_at(start.clone(), 0);
    assert!(!editor.drop_photos(DropPlace::Spread(4), &[outside]));
    assert!(editor.take_notice().unwrap().contains("está em branco"));
    assert_eq!(project(&editor), &start);
}

#[test]
fn pb_b8_montar_o_album_mantem_a_lamina_em_branco() {
    let mut album = Project::new("teste", single_start(spec(30.0, 30.0)));
    for i in 0..12 {
        album.add_photo(format!("f{i:02}.jpg"), 3000, 2000);
    }
    let mut editor = editor_at_blank(album);
    assert!(editor.run(Command::NewBlankSpread));
    assert!(editor.run(Command::NewSpread));
    let p = project(&editor);
    assert_eq!(p.spreads.len(), 2);
    let blank_id = p.spreads[0].id;
    let (out, _) = build_unused(p, 3, &BuildOptions::default()).unwrap();
    assert_eq!(out.spreads.len(), 4, "a lâmina vazia saiu");
    assert_eq!(out.spreads[0].id, blank_id);
    assert!(out.spreads[0].blank.right);
    assert!(out.spreads[0].frames.is_empty());
    assert!(
        out.spreads[1].frames.iter().any(|f| f.rect.x() < 0.5),
        "a 1ª lâmina nova é miolo"
    );
}

fn editor_at_blank(project: Project) -> Editor {
    let mut editor = Editor::new();
    editor.adopt(project);
    editor
}

#[test]
fn pb_b9_espelhar_e_mover() {
    let mut start = built(single_start(spec(30.0, 30.0)), &photos(20), 5);
    set_blank(&mut start, 2, PageSide::Left, true).unwrap();
    let mut editor = editor_at(start, 2);
    assert!(editor.run(Command::Mirror));
    let spread = &project(&editor).spreads[2];
    assert_eq!(
        spread.blank,
        BlankPages {
            left: false,
            right: true
        }
    );
    assert!(
        spread
            .frames
            .iter()
            .all(|f| f.rect.x() + f.rect.width() <= 0.5 + 1e-6)
    );

    // Mover para a ponta uma lâmina com a esquerda em branco: a esquerda não existe ali.
    let mut other = built(single_start(spec(30.0, 30.0)), &photos(20), 5);
    set_blank(&mut other, 3, PageSide::Left, true).unwrap();
    let photos_of_it = spread_photos(&other, 3);
    let id = other.spreads[3].id;
    let mut editor = editor_at(other, 3);
    assert!(editor.move_current_to(0));
    let p = project(&editor);
    assert_eq!(p.spreads[0].id, id);
    assert!(p.spreads[0].blank.is_none(), "a marca saiu");
    assert_eq!(spread_photos(p, 0).len(), photos_of_it.len());
    assert!(photos_only_on_right(p, 0));
}

#[test]
fn pb_b10_texto_numa_pagina_em_branco() {
    let mut start = three_spreads();
    set_blank(&mut start, 0, PageSide::Right, true).unwrap();
    let mut editor = editor_at(start, 0);
    assert!(editor.new_text(Some(PageSide::Right)));
    let p = project(&editor);
    let text = p.spreads[0].frames.last().unwrap();
    assert!(matches!(text.content, FrameContent::Text(_)));
    assert!(text.rect.x() >= 0.5, "o texto entra na página em branco");
    let before = p.clone();

    // Um quadro de foto da direita da lâmina 2, colado na mesma posição da lâmina 1.
    assert!(editor.session_mut().unwrap().go_to(1));
    let on_right = project(&editor).spreads[1]
        .frames
        .iter()
        .position(|f| f.rect.x() >= 0.5)
        .unwrap();
    editor.context_menu(Some(on_right));
    assert!(editor.run(Command::CopyFrames));
    assert!(editor.session_mut().unwrap().go_to(0));
    assert!(!editor.run(Command::PasteFrames));
    assert!(editor.take_notice().unwrap().contains("está em branco"));
    assert_eq!(project(&editor), &before);
}

#[test]
fn pb_b12_tela_faixa_e_rolo() {
    let mut start = built(spec(30.0, 30.0), &photos(32), 8);
    set_blank(&mut start, 3, PageSide::Left, true).unwrap();
    let area = Area {
        width: 1000.0,
        height: 600.0,
        margin: 0.0,
    };
    let view = spread_view(&start, 3, area, &[]).unwrap();
    assert_eq!(view.blank.len(), 1, "a esquerda está em branco");
    assert!(view.inactive.is_empty(), "em branco não é inativa");
    assert!(view.blank[0].x < view.spine_x);
    assert!(spread_view(&start, 2, area, &[]).unwrap().blank.is_empty());

    let editor = editor_at(start.clone(), 3);
    let rows = strip_rows(editor.session().unwrap());
    let row = &rows[3].1;
    assert!(
        row.active.left && row.active.right,
        "as duas continuam impressas"
    );
    assert!(row.spread.blank.left);

    assert_eq!(
        spread_badge(start.spec.edges(), 3, 8),
        "Lâmina 4 · páginas 7 e 8"
    );
}

fn blank_item(menu: &[ContextEntry]) -> (bool, bool) {
    menu.iter()
        .find_map(|e| match e {
            ContextEntry::Item {
                command: Command::BlankPage,
                enabled,
                checked,
                ..
            } => Some((*enabled, *checked)),
            _ => None,
        })
        .expect("o item está no menu")
}

#[test]
fn pb_b13_menu_de_contexto() {
    let start = built(single_start(spec(30.0, 30.0)), &photos(16), 4);
    assert_eq!(start.spec.edges().start, AlbumEdge::SinglePage);
    let mut editor = editor_at(start, 0);
    // A esquerda da ponta não existe: o item fica desativado.
    let menu = editor.context_menu_on_page(PageSide::Left);
    assert_eq!(blank_item(&menu), (false, false));
    let menu = editor.context_menu_on_page(PageSide::Right);
    assert_eq!(blank_item(&menu), (true, false));

    assert!(editor.session_mut().unwrap().go_to(1));
    let menu = editor.context_menu_on_page(PageSide::Left);
    assert_eq!(blank_item(&menu), (true, false));
    assert!(editor.run(Command::BlankPage));
    assert!(project(&editor).spreads[1].blank.left);
    assert!(
        !project(&editor).spreads[1].blank.right,
        "vale só a clicada"
    );
    let menu = editor.context_menu_on_page(PageSide::Left);
    assert_eq!(blank_item(&menu), (true, true), "aparece marcado");
    let menu = editor.context_menu_on_page(PageSide::Right);
    assert_eq!(blank_item(&menu), (true, false));
}

#[test]
fn pb_b14_ia() {
    let base = Base::with_photos(24, (600, 400));
    let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], false);
    lab.ok(
        "novo_album",
        json!({
            "pasta_fotos": base.path("fotos"),
            "salvar_em": base.dir.path().join("pb.lamina"),
            "montar": true,
            "laminas": 6,
        }),
    );
    let edited = {
        let session = lab.editor.session_mut().unwrap();
        session.edit("Ponta", |p| {
            p.spec = p
                .spec
                .clone()
                .with_edges(AlbumEdge::SinglePage, AlbumEdge::FullSpread);
            Ok::<(), crate::EditorError>(())
        })
    };
    edited.unwrap();
    let steps = lab.labels().len();

    lab.ok(
        "pagina_em_branco",
        json!({ "lamina": 2, "lado": "esquerda", "em_branco": true }),
    );
    lab.ok("nova_lamina", json!({ "posicao": 1, "em_branco": true }));
    let refused = lab.call(
        "pagina_em_branco",
        json!({ "lamina": 1, "lado": "esquerda", "em_branco": true }),
    );
    assert_eq!(
        refused.err().map(|e| e.to_string()).as_deref(),
        Some("A lâmina 1 começa só com a página direita.")
    );
    let labels = lab.labels();
    assert_eq!(
        labels.len(),
        steps + 2,
        "a recusa não vira passo: {labels:?}"
    );
    assert_eq!(labels[steps], "IA: Página em branco · lâmina 2");
    assert_eq!(labels[steps + 1], "IA: Nova lâmina em branco · lâmina 1");

    let summary = lab.ok("resumo_do_album", json!({}));
    assert_eq!(
        summary.data["paginas_em_branco"],
        json!([
            { "lamina": 1, "lados": ["direita"] },
            { "lamina": 3, "lados": ["esquerda"] },
        ])
    );
}

#[test]
fn trocar_as_pontas_limpa_a_marca_da_pagina_que_sumiu() {
    let mut start = built(spec(30.0, 30.0), &photos(12), 3);
    set_blank(&mut start, 0, PageSide::Left, true).unwrap();
    let new = single_start(spec(30.0, 30.0));
    crate::respec::respec(&mut start, new).unwrap();
    assert!(start.spreads[0].blank.is_none());
    start.validate().unwrap();
}
