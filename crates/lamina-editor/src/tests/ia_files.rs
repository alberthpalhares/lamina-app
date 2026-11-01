//! Cenários da spec IA com arquivos: pastas permitidas, salvar, abrir, novo álbum, exportar e
//! determinismo (IA-B11 a IA-B16).

use lamina_layout::Reuse;
use serde_json::json;

use super::common::Base;
use super::ia_common::{Lab, album_project, read, unused};
use crate::agent::ToolError;
use crate::{
    Editor, ExportOptions, NewAlbum, Session, builtin_presets, create_album, export_project,
};

#[test]
fn ia_b11_pastas_permitidas() {
    let base = Base::with_photos(30, (600, 400));
    let file = base.save(album_project(&base), "A/album.lamina");
    std::fs::create_dir_all(base.path("C")).unwrap();
    let mut lab = Lab::new(&base, vec![base.path("A")], true);
    lab.ok("abrir_album", json!({ "caminho": file }));
    let outside = base.path("C").join("x.lamina");
    let err = lab
        .call("abrir_album", json!({ "caminho": outside }))
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAllowed(_)));
    assert!(err.to_string().starts_with("A pasta "), "{err}");
    assert!(err.to_string().ends_with(
        "não está liberada para a IA. Libere-a nas configurações do Lamina no Claude (ou com \
         --permitir)."
    ));
    let sneaky = base.path("A").join("..").join("C").join("x.lamina");
    let err = lab
        .call("abrir_album", json!({ "caminho": sneaky }))
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAllowed(_)));
    let spread = lab.ok("ver_lamina", json!({ "lamina": 1 }));
    assert_eq!(
        spread.images.len(),
        1,
        "as fotos de fora da pasta A entram na prévia"
    );
}

#[test]
fn ia_b12_salvar_sem_perder_nada() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, file) = Lab::opened(&base, true);
    let free = unused(lab.project())[0].get();
    lab.ok(
        "colocar_foto",
        json!({ "lamina": 1, "quadro": 1, "foto": free }),
    );
    let other = b"{\"outro\": \"programa\"}".to_vec();
    std::fs::write(&file, &other).unwrap();
    let err = lab.call("salvar_album", json!({})).unwrap_err();
    assert!(matches!(err, ToolError::ChangedOnDisk(_)));
    assert!(err.to_string().ends_with(
        "mudou no disco depois de aberto (talvez no Lamina). Salve com outro nome para não \
         perder nada."
    ));
    let copy = base.path("copia.lamina");
    lab.ok("salvar_album", json!({ "caminho": copy }));
    assert_eq!(read(&file), other);
    let saved = Session::open(&copy).unwrap();
    assert_eq!(saved.project(), lab.project());
}

#[test]
fn ia_b13_abrir_outro_com_alteracoes_nao_salvas() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let free = unused(lab.project())[0].get();
    lab.ok(
        "colocar_foto",
        json!({ "lamina": 1, "quadro": 1, "foto": free }),
    );
    let other = base.save(base.project(4), "outro.lamina");
    let err = lab
        .call("abrir_album", json!({ "caminho": other }))
        .unwrap_err();
    assert!(matches!(err, ToolError::Unsaved(_)));
    assert_eq!(
        err.to_string(),
        "O álbum casamento tem alterações não salvas. Salve com salvar_album ou repita com \
         descartar: true."
    );
    lab.ok(
        "abrir_album",
        json!({ "caminho": other, "descartar": true }),
    );
    assert_eq!(lab.project().spreads.len(), 4);
}

#[test]
fn ia_b14_novo_album_igual_ao_da_janela() {
    let base = Base::with_photos(30, (600, 400));
    let roots = vec![base.path("fotos"), base.path("ia")];
    let mut lab = Lab::new(&base, roots, true);
    let target = base.path("ia").join("album.lamina");
    let args = json!({
        "pasta_fotos": base.path("fotos"),
        "salvar_em": target,
        "preset": "30x30cm",
        "laminas": 8,
        "montar": true,
    });
    lab.ok("novo_album", args);

    let preset = builtin_presets()
        .into_iter()
        .find(|p| p.name == "30x30cm")
        .unwrap();
    let window = Editor::new();
    let folder = base.path("fotos");
    let new = NewAlbum {
        // O nome do arquivo pedido, não o da pasta das fotos (IA-B26).
        name: Some("album".into()),
        folder,
        spec: preset.fields.to_spec().unwrap(),
        spreads: Some(8),
        reuse: Reuse::Medium,
        preferences: window.preferences().clone(),
        learned: lamina_layout::Learned::default(),
        models: window.models().to_vec(),
        limits: preset.limits,
        export: preset.export,
        style: Default::default(),
        files: Vec::new(),
        build: true,
    };
    let (built, _) = create_album(&new, &base.cache(), &|_| {}).unwrap();
    std::fs::create_dir_all(base.path("janela")).unwrap();
    let by_window = Session::new_unsaved(built.project)
        .save_as(&base.path("janela").join("album.lamina"))
        .unwrap();
    assert!(read(&target) == read(&by_window), "os dois .lamina diferem");
    let err = lab.call(
        "novo_album",
        json!({ "pasta_fotos": base.path("fotos"), "salvar_em": target }),
    );
    assert!(matches!(err, Err(ToolError::FileExists(_))));
}

#[test]
fn ia_b15_exportar_sem_substituir() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let out = base.path("saida");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("lamina-01.jpg"), b"antigo").unwrap();
    let err = lab
        .call("exportar", json!({ "pasta": out, "formato": "jpeg" }))
        .unwrap_err();
    assert!(matches!(err, ToolError::ExportExists { count: 1, .. }));
    assert_eq!(
        err.to_string(),
        format!(
            "Já existem 1 arquivos em {} (por exemplo, lamina-01.jpg). Use substituir: true ou \
             escolha outra pasta.",
            out.display()
        )
    );
    assert_eq!(read(&out.join("lamina-01.jpg")), b"antigo");
    lab.ok(
        "exportar",
        json!({ "pasta": out, "formato": "jpeg", "substituir": true }),
    );
    let window = base.path("janela");
    export_project(
        lab.project(),
        &window,
        &ExportOptions::default(),
        &|_, _| {},
    )
    .unwrap();
    for n in 1..=6 {
        let name = format!("lamina-0{n}.jpg");
        assert!(
            read(&out.join(&name)) == read(&window.join(&name)),
            "{name} difere"
        );
    }
}

/// Substituir não apaga as sobras de uma exportação anterior: a IA recusa (spec IA, regra 6).
#[test]
fn ia_b15_substituir_com_sobras_recusa_sem_apagar() {
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, true);
    let out = base.path("saida");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("lamina-09.jpg"), b"antigo").unwrap();
    let args = json!({ "pasta": out, "substituir": true });
    let err = lab.call("exportar", args).unwrap_err();
    assert!(
        matches!(err, ToolError::Leftovers { count: 1, .. }),
        "{err}"
    );
    assert!(err.to_string().contains("A IA não apaga arquivos"));
    assert_eq!(read(&out.join("lamina-09.jpg")), b"antigo");
}

/// A primeira linha em que dois `.lamina` diferem, para a mensagem do teste.
fn first_difference(a: &[u8], b: &[u8]) -> String {
    let (a, b) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    let mut lines = a.lines().zip(b.lines()).enumerate();
    match lines.find(|(_, (x, y))| x != y) {
        Some((n, (x, y))) => format!("linha {}: {x:?} × {y:?}", n + 1),
        None => format!("tamanhos {} × {}", a.len(), b.len()),
    }
}

/// Regra 12 no álbum de teste: montado duas vezes (importar e Auto Build), dá o mesmo arquivo.
#[test]
fn album_de_teste_montado_duas_vezes_e_o_mesmo() {
    let base = Base::with_photos(30, (600, 400));
    let first = read(&base.save(album_project(&base), "um/album.lamina"));
    let second = read(&base.save(album_project(&base), "dois/album.lamina"));
    assert!(first == second, "{}", first_difference(&first, &second));
}

#[test]
fn ia_b16_mesma_sequencia_mesmo_arquivo() {
    let base = Base::with_photos(30, (600, 400));
    let first = base.save(album_project(&base), "um/album.lamina");
    let second = base.path("dois/album.lamina");
    std::fs::create_dir_all(base.path("dois")).unwrap();
    std::fs::copy(&first, &second).unwrap();
    let mut saved = Vec::new();
    for file in [&first, &second] {
        let mut lab = Lab::new(&base, vec![base.dir.path().to_path_buf()], true);
        lab.ok("abrir_album", json!({ "caminho": file }));
        let free = unused(lab.project())[0].get();
        let calls = [
            ("sugerir_layouts", json!({ "lamina": 3 })),
            ("aplicar_layout", json!({ "lamina": 3, "opcao": 2 })),
            (
                "colocar_foto",
                json!({ "lamina": 2, "quadro": 1, "foto": free }),
            ),
            (
                "trocar_fotos",
                json!({ "lamina": 2, "quadro_a": 1, "quadro_b": 2 }),
            ),
            ("tirar_foto", json!({ "lamina": 2, "quadro": 3 })),
            ("nova_lamina", json!({})),
            ("mover_lamina", json!({ "lamina": 7, "para": 2 })),
            ("espelhar", json!({ "lamina": 4 })),
            ("desfazer", json!({})),
            ("refazer", json!({})),
        ];
        for (name, args) in calls {
            lab.ok(name, args);
        }
        lab.ok("salvar_album", json!({}));
        saved.push(read(file));
    }
    assert!(
        saved[0] == saved[1],
        "os dois .lamina diferem: {}",
        first_difference(&saved[0], &saved[1])
    );
}
