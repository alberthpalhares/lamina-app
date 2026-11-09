//! Cenários da spec `modelos` (MD) no editor: B1 a B7, B9 e B10. O B8 e o B12 estão em
//! `md_more.rs`; o B11 roda o binário `lamina`.

use lamina_core::{Border, FrameContent, Mm, Orientation, PageSide, Project, RelRect, Rgb};
use lamina_layout::{ModelSlot, SpreadModel};

use super::tl_common::{H, V, editor_at, spec};
use crate::snap::Grip;
use crate::{Command, Editor, ModelError, ModelsStore, Part, TextChange, option_title};

/// Um álbum de 30 × 30 com as lâminas `groups` (cada uma com as fotos dadas).
pub(super) fn album(groups: &[&[(u32, u32)]]) -> Project {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    // Fotos inteiras: um modelo com quadro saindo do corte por um fio é ajustado ao ler o arquivo.
    project.mounting.crop = lamina_core::CropLevel::Whole;
    let mut n = 0;
    for (i, g) in groups.iter().enumerate() {
        project.add_spread();
        let ids: Vec<_> = g
            .iter()
            .map(|(w, h)| {
                n += 1;
                project.add_photo(format!("f{n:02}.jpg"), *w, *h)
            })
            .collect();
        crate::edits::layout_with(&mut project, i, &ids).unwrap();
    }
    project
}

/// Editor com os modelos num arquivo temporário (o diretório fica vivo junto).
pub(super) fn with_models(project: Project, index: usize) -> (Editor, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = editor_at(project, index);
    editor.set_models(ModelsStore::load(&dir.path().join("modelos.json")));
    (editor, dir)
}

/// Lâmina 1 à mão (H, H, V), borda de 1 mm e o texto "Capítulo 1", salva como "Abertura".
pub(super) fn abertura(editor: &mut Editor) {
    editor.frame_grab(0, Grip::Move);
    editor.frame_drag(5.0, 5.0, false, 0.0);
    editor.frame_release();
    let border = Border {
        width: Mm(1.0),
        color: Rgb::WHITE,
    };
    editor
        .session_mut()
        .unwrap()
        .edit("borda", |p: &mut Project| {
            p.spreads[0]
                .frames
                .iter_mut()
                .for_each(|f| f.border = border);
            Ok::<(), ()>(())
        })
        .unwrap();
    editor.new_text(Some(PageSide::Left));
    editor.change_text(TextChange::Text("Capítulo 1".into()));
    editor.save_model("Abertura").unwrap();
}

pub(super) const HHV: &[(u32, u32)] = &[H, H, V];

fn titles(editor: &Editor) -> Vec<String> {
    editor
        .panel(Part::Spread)
        .options
        .iter()
        .map(|c| option_title(&c.family))
        .collect()
}

fn position(editor: &Editor, title: &str) -> Option<usize> {
    titles(editor).iter().position(|t| t == title)
}

#[test]
fn md_b1_salvar_a_lamina_como_modelo() {
    let (mut editor, dir) = with_models(album(&[HHV, HHV]), 0);
    assert!(editor.can_save_model());
    abertura(&mut editor);
    let model = editor.models()[0].clone();
    assert_eq!(model.name, "Abertura");
    let orientations: Vec<_> = model.slots.iter().map(|s| s.orientation).collect();
    let (h, v) = (Some(Orientation::Landscape), Some(Orientation::Portrait));
    assert_eq!(orientations, [h, h, v]);
    assert!(model.slots.iter().all(|s| s.border.width == Mm(1.0)));
    assert_eq!(model.texts.len(), 1);
    assert_eq!(model.texts[0].text.text, "Capítulo 1");
    assert!(model.left && model.right);
    let project = editor.session().unwrap().project();
    let rects: Vec<RelRect> = project.spreads[0]
        .frames
        .iter()
        .filter(|f| !matches!(f.content, FrameContent::Text(_)))
        .map(|f| f.rect)
        .collect();
    assert_eq!(
        model.slots.iter().map(|s| s.rect).collect::<Vec<_>>(),
        rects
    );
    let again = ModelsStore::load(&dir.path().join("modelos.json"));
    assert_eq!(again.all(), [model]);
    assert_eq!(editor.save_model("  "), Err(ModelError::EmptyName));
}

#[test]
fn md_b2_o_modelo_aparece_no_painel_e_e_aplicado() {
    let (mut editor, _dir) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    editor.session_mut().unwrap().go_to(1);
    let i = position(&editor, "Modelo · Abertura").expect("na lista Na ordem");
    let model = editor.models()[0].clone();
    assert!(editor.apply_option(Part::Spread, i));
    let s = editor.session().unwrap();
    let project = s.project();
    for f in &project.spreads[1].frames {
        let FrameContent::Photo { photo, .. } = &f.content else {
            continue;
        };
        let o = Orientation::from_aspect_ratio(project.photo(*photo).unwrap().aspect_ratio());
        let slot = model
            .slots
            .iter()
            .find(|sl| sl.rect == f.rect)
            .expect("quadro do modelo");
        assert_eq!(slot.orientation, Some(o));
    }
    assert_eq!(s.undo_label(), Some("Trocar modelo"));
    assert_eq!(project.spreads[1].model.as_deref(), Some("modelo:Abertura"));
}

#[test]
fn md_b3_casa_so_noutra_ordem() {
    let (mut editor, _dir) = with_models(album(&[HHV, &[V, H, H]]), 0);
    abertura(&mut editor);
    editor.session_mut().unwrap().go_to(1);
    assert_eq!(position(&editor, "Modelo · Abertura"), None, "não na ordem");
    assert!(editor.run(Command::ToggleReordered));
    let i = position(&editor, "Modelo · Abertura").expect("nas reordenadas");
    assert!(editor.apply_option(Part::Spread, i));
    let model = editor.models()[0].clone();
    let project = editor.session().unwrap().project();
    let vertical = project.spreads[1].frames.iter().find(|f| match &f.content {
        FrameContent::Photo { photo, .. } => project.photo(*photo).unwrap().aspect_ratio() < 1.0,
        _ => false,
    });
    let v_slot = model
        .slots
        .iter()
        .find(|s| s.orientation == Some(Orientation::Portrait));
    assert_eq!(vertical.map(|f| f.rect), v_slot.map(|s| s.rect));
}

#[test]
fn md_b4_nao_casa() {
    let (mut editor, _dir) = with_models(album(&[HHV, &[H, H, H, H], &[H, H, H]]), 0);
    abertura(&mut editor);
    for index in [1, 2] {
        editor.session_mut().unwrap().go_to(index);
        // O modelo do fotógrafo não aparece; os de fábrica da mistura podem (spec RM, beta 9.1).
        assert!(!titles(&editor).iter().any(|t| t.starts_with("Modelo ·")));
        editor.run(Command::ToggleReordered);
        assert!(!titles(&editor).iter().any(|t| t.starts_with("Modelo ·")));
        editor.run(Command::ToggleReordered);
    }
}

/// Modelo simétrico: duas horizontais espelhadas e uma vertical no meio, sobre a dobra.
pub(super) fn symmetric() -> SpreadModel {
    let slot = |x: f64, y: f64, w: f64, h: f64, o| ModelSlot {
        rect: RelRect::new(x, y, w, h).unwrap(),
        orientation: Some(o),
        border: Border::default(),
        radius: Mm(0.0),
    };
    SpreadModel {
        name: "Simétrico".into(),
        slots: vec![
            slot(0.05, 0.3, 0.25, 0.3, Orientation::Landscape),
            slot(0.7, 0.3, 0.25, 0.3, Orientation::Landscape),
            slot(0.4, 0.2, 0.2, 0.6, Orientation::Portrait),
        ],
        texts: Vec::new(),
        left: true,
        right: true,
        factory: false,
    }
}

#[test]
fn md_b5_espelhado() {
    let (mut editor, dir) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    let mut store = ModelsStore::load(&dir.path().join("modelos.json"));
    store.save(symmetric()).unwrap();
    editor.set_models(store);
    editor.session_mut().unwrap().go_to(1);
    let t = titles(&editor);
    assert!(t.contains(&"Modelo · Abertura".to_owned()));
    assert!(t.contains(&"Modelo · Abertura · espelhado".to_owned()));
    assert_eq!(
        t.iter()
            .filter(|x| x.starts_with("Modelo · Simétrico"))
            .count(),
        1
    );
}

#[test]
fn md_b6_favoritar_rejeitar_e_renomear() {
    let (mut editor, _dir) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    editor.session_mut().unwrap().go_to(1);
    let i = position(&editor, "Modelo · Abertura").unwrap();
    assert!(editor.mark_option(i, true));
    let e = position(&editor, "Modelo · Abertura · espelhado").unwrap();
    assert!(editor.mark_option(e, false));
    assert!(editor.rename_model("Abertura", "Início").unwrap());
    assert_eq!(
        position(&editor, "Modelo · Início"),
        Some(0),
        "favorito no topo"
    );
    assert_eq!(
        position(&editor, "Modelo · Início · espelhado"),
        None,
        "rejeitado fora"
    );
    assert!(editor.remove_model("Início").unwrap());
    let prefs = editor.preferences();
    assert!(
        !prefs
            .favorites
            .iter()
            .chain(&prefs.rejected)
            .any(|k| k.starts_with("modelo:"))
    );
}

#[test]
fn md_b7_textos_do_modelo() {
    let (mut editor, _dir) = with_models(album(&[HHV, HHV, HHV]), 0);
    abertura(&mut editor);
    let texts = |editor: &Editor, index: usize| -> Vec<String> {
        editor.session().unwrap().project().spreads[index]
            .frames
            .iter()
            .filter_map(|f| match &f.content {
                FrameContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect()
    };
    editor.session_mut().unwrap().go_to(1);
    let i = position(&editor, "Modelo · Abertura").unwrap();
    editor.apply_option(Part::Spread, i);
    assert_eq!(texts(&editor, 1), ["Capítulo 1"]);
    editor.session_mut().unwrap().go_to(2);
    editor.new_text(None);
    editor.change_text(TextChange::Text("Festa".into()));
    editor.select_frame(None);
    let i = position(&editor, "Modelo · Abertura").unwrap();
    editor.apply_option(Part::Spread, i);
    assert_eq!(texts(&editor, 2), ["Festa"]);
}

#[test]
fn md_b9_espaco_e_especificacao_mantem_o_modelo() {
    let (mut editor, _dir) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    editor.session_mut().unwrap().go_to(1);
    let i = position(&editor, "Modelo · Abertura").unwrap();
    editor.apply_option(Part::Spread, i);
    let mut project = editor.session().unwrap().project().clone();
    let rects: Vec<RelRect> = project.spreads[1].frames.iter().map(|f| f.rect).collect();
    crate::appearance::regap(&mut project, 1, Mm(9.0)).unwrap();
    crate::respec::respec(&mut project, spec(25.0, 25.0)).unwrap();
    let after: Vec<RelRect> = project.spreads[1].frames.iter().map(|f| f.rect).collect();
    assert_eq!(after, rects);
    assert_eq!(project.spreads[1].model.as_deref(), Some("modelo:Abertura"));
}

#[test]
fn md_b10_arquivo_de_modelos_ilegivel() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("modelos.json");
    std::fs::write(&file, "isto não é json").unwrap();
    let store = ModelsStore::load(&file);
    assert!(store.all().is_empty());
    let warning = store.warning.clone().unwrap();
    assert!(
        warning.starts_with("Não foi possível ler os modelos: "),
        "{warning}"
    );
    let mut editor = editor_at(album(&[HHV]), 0);
    editor.set_models(store);
    assert_eq!(editor.take_notice(), Some(warning));
    assert!(!editor.panel(Part::Spread).options.is_empty());
    assert!(file.exists(), "o arquivo não é apagado");
    // Apagar um modelo não muda o álbum que o usa.
    let (mut editor, _d) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    editor.session_mut().unwrap().go_to(1);
    let i = position(&editor, "Modelo · Abertura").unwrap();
    editor.apply_option(Part::Spread, i);
    let before = editor.session().unwrap().project().clone();
    editor.remove_model("Abertura").unwrap();
    assert_eq!(editor.session().unwrap().project(), &before);
}
