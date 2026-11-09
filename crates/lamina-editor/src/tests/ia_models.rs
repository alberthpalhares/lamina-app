//! Cenários da spec IA do IA-M2 que mexem no álbum inteiro ou nos arquivos do fotógrafo: IA-B22
//! (modelo e especificação) e IA-B23 (reencontrar fotos), com as recusas.

use serde_json::json;

use super::common::Base;
use super::ia_common::{Lab, album_project};
use super::md::{HHV, abertura, album, with_models};
use crate::agent::ToolError;
use crate::{Editor, ModelError, Part, Session, builtin_presets, option_title, search_folder};

/// Um laboratório da IA sobre o editor da janela (com os modelos do fotógrafo).
fn lab_on(base: &Base, editor: Editor) -> Lab {
    let mut lab = Lab::new(base, vec![base.dir.path().to_path_buf()], true);
    lab.editor = editor;
    lab
}

#[test]
fn ia_b22_modelo_e_especificacao() {
    let base = Base::with_photos(0, (1, 1));
    let (mut editor, _dir) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut editor);
    let (mut window, _dir2) = with_models(album(&[HHV, HHV]), 0);
    abertura(&mut window);
    let mut lab = lab_on(&base, editor);
    lab.ok("usar_modelo", json!({ "lamina": 2, "nome": "abertura" }));

    window.session_mut().unwrap().go_to(1);
    let titles: Vec<String> = window
        .panel(Part::Spread)
        .options
        .iter()
        .map(|c| option_title(&c.family))
        .collect();
    let i = titles
        .iter()
        .position(|t| t == "Modelo · Abertura")
        .unwrap();
    assert!(window.apply_option(Part::Spread, i));
    assert_eq!(
        lab.project().spreads[1],
        window.session().unwrap().project().spreads[1]
    );

    lab.ok("trocar_especificacao", json!({ "preset": "25x25cm" }));
    let preset = builtin_presets()
        .into_iter()
        .find(|p| p.name == "25x25cm")
        .unwrap();
    window
        .respec(preset.fields.to_spec().unwrap(), preset.limits)
        .unwrap();
    assert_eq!(lab.project(), window.session().unwrap().project());
    let labels = lab.labels();
    assert_eq!(
        labels[labels.len() - 2..],
        [
            "IA: Trocar modelo · lâmina 2",
            "IA: Trocar a especificação · todas as lâminas"
        ]
    );
    let before_respec = {
        let mut w = window;
        w.session_mut().unwrap().undo();
        w.session().unwrap().project().clone()
    };
    lab.ok("desfazer", json!({}));
    assert_eq!(lab.project(), &before_respec, "a troca é uma edição só");
}

#[test]
fn modelos_e_especificacao_recusam() {
    let base = Base::with_photos(0, (1, 1));
    let (editor, _dir) = with_models(album(&[HHV, &[super::tl_common::H; 4]]), 0);
    let mut lab = lab_on(&base, editor);
    let err = lab
        .call("usar_modelo", json!({ "lamina": 1, "nome": "Abertura" }))
        .unwrap_err();
    assert!(matches!(err, ToolError::NoModels));
    lab.ok("salvar_modelo", json!({ "lamina": 1, "nome": "Abertura" }));
    let err = lab
        .call("salvar_modelo", json!({ "lamina": 2, "nome": "Abertura" }))
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Já existe o modelo Abertura. Use substituir: true ou outro nome."
    );
    let err = lab
        .call("salvar_modelo", json!({ "lamina": 2, "nome": "  " }))
        .unwrap_err();
    assert!(matches!(err, ToolError::Model(ModelError::EmptyName)));
    let err = lab
        .call("usar_modelo", json!({ "lamina": 2, "nome": "Abertura" }))
        .unwrap_err();
    assert!(matches!(err, ToolError::ModelDoesNotFit { spread: 2, .. }));
    let err = lab
        .call("usar_modelo", json!({ "lamina": 2, "nome": "Capa" }))
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Não existe o modelo Capa. Os modelos são: Abertura."
    );
    let before = lab.project().clone();
    let refusals = [
        json!({}),
        json!({ "preset": "Gigante" }),
        json!({ "paginas_min": 40, "paginas_max": 20 }),
    ];
    let errors: Vec<ToolError> = refusals
        .into_iter()
        .map(|args| lab.call("trocar_especificacao", args).unwrap_err())
        .collect();
    assert!(matches!(errors[0], ToolError::NothingAsked(_)));
    assert!(matches!(&errors[1], ToolError::NoPreset { name, .. } if name == "Gigante"));
    assert_eq!(
        errors[2].to_string(),
        "O mínimo de páginas passa do máximo."
    );
    assert_eq!(lab.project(), &before);
    assert!(lab.labels().is_empty());
}

#[test]
fn comandos_de_dialogo_apontam_as_ferramentas_novas() {
    let base = Base::with_photos(0, (1, 1));
    let (editor, _dir) = with_models(album(&[HHV]), 0);
    let mut lab = lab_on(&base, editor);
    let pairs = [
        ("arquivo.procurar-fotos", "reencontrar_fotos"),
        ("arquivo.especificacao", "trocar_especificacao"),
        ("lamina.salvar-modelo", "salvar_modelo"),
    ];
    for (command, tool) in pairs {
        let err = lab
            .call("executar_comando", json!({ "comando": command }))
            .unwrap_err();
        assert!(
            matches!(err, ToolError::DialogCommand { tool: t, .. } if t == tool),
            "{command}: {err}"
        );
    }
}

#[test]
fn ia_b23_reencontrar_fotos() {
    let base = Base::with_photos(30, (600, 400));
    let file = base.save(album_project(&base), "album.lamina");
    let moved = base.path("movidas");
    std::fs::create_dir_all(&moved).unwrap();
    let photos = Session::open(&file).unwrap().project().photos.clone();
    for (i, photo) in photos.iter().take(3).enumerate() {
        std::fs::rename(&photo.path, moved.join(format!("outro-nome-{i}.jpg"))).unwrap();
    }
    let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], true);
    lab.ok("abrir_album", json!({ "caminho": file }));
    assert_eq!(lab.editor.session().unwrap().missing().len(), 3);
    let reply = lab.ok("reencontrar_fotos", json!({ "pasta": moved }));
    assert_eq!(reply.text, "3 fotos reencontradas.");
    assert!(lab.editor.session().unwrap().missing().is_empty());
    assert_eq!(lab.labels(), ["IA: Reencontrar fotos · 3 fotos"]);
    // Arquivo → Reencontrar fotos… na mesma pasta.
    let mut window = Session::open(&file).unwrap();
    let missing = window.missing().to_vec();
    let found = search_folder(window.project(), &missing, &moved, &|_, _| {});
    assert_eq!(window.relink(&found), 3);
    assert_eq!(lab.project().photos, window.project().photos);

    let err = lab
        .call("reencontrar_fotos", json!({ "pasta": moved }))
        .unwrap_err();
    assert!(matches!(err, ToolError::NothingMissing));
    let outside = tempfile::tempdir().unwrap();
    let err = lab
        .call("reencontrar_fotos", json!({ "pasta": outside.path() }))
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAllowed(_)));
}

#[test]
fn reencontrar_numa_pasta_sem_as_fotos_recusa() {
    let base = Base::with_photos(30, (600, 400));
    let file = base.save(album_project(&base), "album.lamina");
    let photo = Session::open(&file).unwrap().project().photos[0]
        .path
        .clone();
    std::fs::remove_file(&photo).unwrap();
    std::fs::create_dir_all(base.path("vazia")).unwrap();
    let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], true);
    lab.ok("abrir_album", json!({ "caminho": file }));
    let err = lab
        .call("reencontrar_fotos", json!({ "pasta": base.path("vazia") }))
        .unwrap_err();
    assert!(
        err.to_string()
            .starts_with("A foto ausente não foi encontrada em ")
    );
    assert!(lab.labels().is_empty());
}
