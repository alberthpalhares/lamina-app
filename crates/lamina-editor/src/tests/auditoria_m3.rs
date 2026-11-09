//! Achados da auditoria do Marco 3 (`docs/auditorias/2026-10-18-marco-3.md`) que viraram teste no
//! editor: M2, M3, M4 (presets), M6, M7, D4 e D5.

use lamina_core::{Border, Mm, Orientation, PageSide, Project, RelRect, Rgb, TextBox};
use lamina_layout::{ModelSlot, ModelText, SpreadModel};

use super::tl_common::{editor_at, three_spreads};
use crate::frame_form::{page_at, parse_frame};
use crate::hand::frame_mm;
use crate::new_project::Unit;
use crate::presets::{Preset, PresetsStore};
use crate::snap::Grip;
use crate::{Command, ModelsStore, Part, Session, TextChange, option_title};

#[test]
fn trocar_de_lamina_no_meio_do_gesto_descarta_o_gesto() {
    let mut editor = editor_at(three_spreads(), 0);
    let before = editor.session().unwrap().project().clone();
    assert!(editor.frame_grab(0, Grip::Move));
    editor.frame_drag(30.0, 0.0, false, 0.0);
    assert!(editor.run(Command::NextSpread));
    assert!(!editor.frame_release(), "o gesto não grava em outra lâmina");
    assert_eq!(editor.session().unwrap().project(), &before);
    // Ir à lâmina pela faixa e pelo histórico também descarta.
    editor.frame_grab(0, Grip::Move);
    editor.frame_drag(30.0, 0.0, false, 0.0);
    assert!(editor.go_to_spread(2));
    assert!(!editor.frame_release());
    assert_eq!(editor.session().unwrap().project(), &before);
}

#[test]
fn abrir_outro_projeto_descarta_o_gesto_de_quadro() {
    let mut editor = editor_at(three_spreads(), 0);
    editor.frame_grab(0, Grip::Move);
    editor.frame_drag(30.0, 0.0, false, 0.0);
    let other = three_spreads();
    editor.adopt(other.clone());
    assert!(!editor.frame_release());
    assert_eq!(editor.session().unwrap().project(), &other);
}

#[test]
fn um_comando_encerra_a_emenda_do_texto() {
    let mut editor = editor_at(three_spreads(), 0);
    editor.new_text(None);
    let frame = editor.selected_frame().unwrap();
    editor.change_text(TextChange::Text("Ana".into()));
    editor.change_text(TextChange::Text("Ana &".into()));
    assert!(editor.run(Command::NextSpread));
    assert!(editor.run(Command::PreviousSpread));
    editor.select_frame(Some(frame));
    editor.change_text(TextChange::Text("Ana & Pedro".into()));
    let s = editor.session().unwrap();
    let labels: Vec<String> = s
        .history_list()
        .steps
        .iter()
        .map(|h| h.label.clone())
        .collect();
    let texts = labels.iter().filter(|l| l.starts_with("Texto")).count();
    assert_eq!(texts, 2, "{labels:?}");
}

#[test]
fn aba_quadro_mostra_e_le_na_unidade_da_tela() {
    // A unidade de sempre é cm (spec NP, regra 3): 12 cm de largura aparecem como "12".
    let mut editor = editor_at(three_spreads(), 0);
    editor.select_frame(Some(0));
    let form = editor.frame_form().unwrap();
    assert_eq!(form.unit, "cm");
    let s = editor.session().unwrap();
    let r = frame_mm(s.project(), 0, 0).unwrap();
    assert_eq!(form.width, Unit::Cm.show(r.width.get()));
    let project = s.project().clone();
    let rect = parse_frame(&project, ["1", "2", "3,5", "4"], Unit::Cm).unwrap();
    let trim = project.spec.trim_rect();
    assert!((rect.x.get() - trim.x.get() - 10.0).abs() < 1e-9);
    assert!((rect.width.get() - 35.0).abs() < 1e-9);
    let inch = parse_frame(&project, ["0", "0", "1", "1"], Unit::Inch).unwrap();
    assert!((inch.width.get() - 25.4).abs() < 1e-9);
}

#[test]
fn aba_quadro_digitada_prende_ao_corte_e_marca_a_mao() {
    let mut editor = editor_at(three_spreads(), 0);
    editor.select_frame(Some(0));
    let form = editor.frame_form().unwrap();
    assert!(form.text.is_none());
    let project = editor.session().unwrap().project().clone();
    let rect = parse_frame(&project, ["-10", "5", "2", "2,5"], Unit::Mm).unwrap();
    assert!(editor.set_frame_mm(rect));
    let s = editor.session().unwrap();
    assert_eq!(s.undo_label(), Some("Redimensionar quadro"));
    let r = frame_mm(s.project(), 0, 0).unwrap();
    let trim = s.project().spec.trim_rect();
    assert!((r.x.get() - trim.x.get()).abs() < 1e-9, "preso ao corte");
    assert!((r.width.get() - 5.0).abs() < 1e-9 && (r.height.get() - 5.0).abs() < 1e-9);
    assert!(s.project().spreads[0].manual);
    assert_eq!(
        parse_frame(&project, ["doze", "1", "1", "1"], Unit::Mm),
        Err(crate::frame_form::BAD_NUMBER)
    );
}

#[test]
fn pagina_sob_o_mouse() {
    let project = three_spreads();
    assert_eq!(page_at(&project, 0.1), Some(PageSide::Left));
    assert_eq!(page_at(&project, 0.9), Some(PageSide::Right));
    assert_eq!(page_at(&project, 1.5), None);
    assert_eq!(page_at(&project, -1.0), None);
}

#[test]
fn modelo_com_quadro_fora_do_corte_e_preso_ao_aplicar() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("modelos.json");
    // Arquivo editado à mão: x 0,9 com largura 0,5 passa do corte.
    let json = r#"{"versao":1,"modelos":[{"name":"Torto","left":true,"right":true,"slots":[
        {"rect":{"x":0.05,"y":0.1,"width":0.3,"height":0.4},"orientation":"landscape"},
        {"rect":{"x":0.4,"y":0.1,"width":0.3,"height":0.4},"orientation":"landscape"},
        {"rect":{"x":0.9,"y":0.5,"width":0.5,"height":0.4},"orientation":"landscape"},
        {"rect":{"x":0.5,"y":0.6,"width":0.2,"height":0.3},"orientation":"portrait"}]}]}"#;
    std::fs::write(&file, json).unwrap();
    let store = ModelsStore::load(&file);
    assert_eq!(store.warning, None);
    let slot = &store.all()[0].slots[2];
    assert!(slot.rect.x() + slot.rect.width() <= 1.0 + 1e-12);
    let mut project = Project::new("teste", super::tl_common::spec(30.0, 30.0));
    project.add_spread();
    let sizes = [(3000, 2000), (3000, 2000), (3000, 2000), (2000, 3000)];
    let ids: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(n, (w, h))| project.add_photo(format!("f{n}.jpg"), *w, *h))
        .collect();
    crate::edits::layout_with(&mut project, 0, &ids).unwrap();
    let mut editor = editor_at(project, 0);
    editor.set_models(store);
    let find = |editor: &crate::Editor| {
        editor
            .panel(Part::Spread)
            .options
            .iter()
            .position(|c| option_title(&c.family) == "Modelo · Torto")
    };
    // Na ordem ou, se as fotos vieram noutra ordem, nas reordenadas.
    let i = find(&editor).or_else(|| {
        editor.run(Command::ToggleReordered);
        find(&editor)
    });
    let i = i.unwrap();
    assert!(editor.apply_option(Part::Spread, i));
    let saved = dir.path().join("album.lamina");
    editor.session_mut().unwrap().save_as(&saved).unwrap();
    let again = Session::open(&saved).unwrap();
    assert!(
        again.project().spreads[0]
            .frames
            .iter()
            .all(|f| f.rect.x() + f.rect.width() <= 1.0 + 1e-9)
    );
}

#[test]
fn quadro_invalido_nao_entra_no_projeto() {
    let mut project = Project::new("teste", super::tl_common::spec(30.0, 30.0));
    let id = project.add_spread();
    let bad: RelRect =
        serde_json::from_str(r#"{"x":0.9,"y":0.0,"width":0.5,"height":0.5}"#).unwrap();
    let content = lamina_core::FrameContent::Empty;
    assert!(matches!(
        project.add_frame(id, bad, content),
        Err(lamina_core::CoreError::InvalidRelRect(_))
    ));
}

fn slot(x: f64, o: Orientation) -> ModelSlot {
    ModelSlot {
        rect: RelRect::new(x, 0.3, 0.2, 0.4).unwrap(),
        orientation: Some(o),
        border: Border::default(),
        radius: Mm(0.0),
    }
}

#[test]
fn modelo_simetrico_nas_fotos_mas_com_titulo_ganha_espelho() {
    let title = ModelText {
        rect: RelRect::new(0.05, 0.05, 0.3, 0.1).unwrap(),
        text: TextBox {
            text: "Capítulo".into(),
            ..TextBox::new(Rgb::BLACK)
        },
    };
    let model = SpreadModel {
        name: "Título".into(),
        slots: vec![
            slot(0.1, Orientation::Landscape),
            slot(0.7, Orientation::Landscape),
        ],
        texts: vec![title],
        left: true,
        right: true,
        factory: false,
    };
    assert!(!model.is_symmetric(), "o título só está à esquerda");
    let swapped = SpreadModel {
        slots: vec![
            slot(0.1, Orientation::Landscape),
            slot(0.7, Orientation::Portrait),
        ],
        texts: Vec::new(),
        ..model.clone()
    };
    assert!(!swapped.is_symmetric(), "orientações trocadas no espelho");
    let plain = SpreadModel {
        texts: Vec::new(),
        ..model
    };
    assert!(plain.is_symmetric());
}

#[test]
fn renomear_preset_mantem_a_posicao() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = PresetsStore::load(&dir.path().join("presets.json"));
    for name in ["A", "B", "C"] {
        store
            .save(Preset {
                name: name.into(),
                ..crate::presets::builtin_presets()[0].clone()
            })
            .unwrap();
    }
    assert!(store.rename("A", "Z").unwrap());
    let names: Vec<&str> = store.user().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Z", "B", "C"]);
}
