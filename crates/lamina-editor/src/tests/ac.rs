//! Cenários das specs `aprovacao-cliente` (AC) e `aprovacao-cliente-html` (AH) que moram no editor:
//! os dados do fotógrafo, a pasta por álbum e a conferência antes de gerar. O formulário e os textos da caixa estão em `ac_form.rs`.

use std::path::Path;

use lamina_core::{AlbumSpec, Dpi, FrameContent, Mm, Project, RelRect, Rgb, TextBox};
use lamina_export::{ProjectFileError, WarningKind};

use crate::session::Session;
use crate::{APPROVAL_FILE, ApprovalForm, ApprovalStore, Cache, EditorError, generate_approval};

pub(super) fn album(name: &str) -> Project {
    let spec =
        AlbumSpec::new(Mm::from_cm(20.0), Mm::from_cm(20.0), Dpi::new(100).unwrap()).unwrap();
    let mut project = Project::new(name, spec);
    project.add_spread();
    project
}

fn cache(dir: &Path) -> Cache {
    Cache::new(dir.join("cache"))
}

#[test]
fn ac_b30_dados_do_fotografo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(APPROVAL_FILE);
    let project = album("Primeiro");
    let mut store = ApprovalStore::load(&file);
    let mut form = ApprovalForm::new(&store.data, &project);
    form.watermark_on = true;
    form.watermark_text = "Estúdio Exemplo".to_owned();
    form.html = false;
    let out = dir.path().join("saida");
    let report = generate_approval(
        &project,
        &out,
        &form.options(false),
        &cache(dir.path()),
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(report.files.len(), 1);
    store.data = form.data();
    store.save().unwrap();

    let other = album("Segundo");
    let reopened = ApprovalStore::load(&file);
    assert_eq!(reopened.warning, None);
    let form = ApprovalForm::new(&reopened.data, &other);
    assert!(form.watermark_on);
    assert_eq!(form.watermark_text, "Estúdio Exemplo");
    assert_eq!(form.client_name, "Segundo", "o nome é do álbum");
    assert!(!other.to_json().unwrap().contains("Estúdio Exemplo"));
    assert!(!project.to_json().unwrap().contains("Estúdio Exemplo"));
}

#[test]
fn ac_b31_arquivo_de_dados_ilegivel() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(APPROVAL_FILE);
    for broken in [
        "{ estragado",
        r#"{"versao": 99, "marca_dagua_ligada": true, "marca_dagua_texto": "x"}"#,
    ] {
        std::fs::write(&file, broken).unwrap();
        let mut store = ApprovalStore::load(&file);
        assert_eq!(
            store.warning.as_deref(),
            Some(
                "Não foi possível ler os dados guardados da aprovação do cliente. Os campos começam vazios."
            )
        );
        assert!(!store.data.watermark_on);
        assert_eq!(store.data.watermark_text, "");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            broken,
            "ler não estraga o arquivo"
        );
        store.data.watermark_on = true;
        store.data.watermark_text = "Novo".to_owned();
        store.save().unwrap();
        let again = ApprovalStore::load(&file);
        assert_eq!(again.warning, None, "gerar grava um arquivo válido");
        assert_eq!(again.data.watermark_text, "Novo");
    }
}

#[test]
fn ac_b32_pasta_por_album() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let first = Session::new_unsaved(album("Primeiro"))
        .save_as(&a.join("Primeiro.lamina"))
        .unwrap();
    let second = Session::new_unsaved(album("Segundo"))
        .save_as(&b.join("Segundo.lamina"))
        .unwrap();
    let mut session = Session::open(&first).unwrap();
    assert_eq!(session.approval_folder(), a.join("aprovacao-cliente"));
    let chosen = dir.path().join("escolhida");
    session.set_approval_folder(&chosen);
    assert_eq!(session.approval_folder(), chosen);
    let session = Session::open(&second).unwrap();
    assert_eq!(session.approval_folder(), b.join("aprovacao-cliente"));
}

/// A pasta da impressão e a da aprovação não se misturam.
#[test]
fn ac_b32_a_pasta_nao_e_a_da_impressao() {
    let mut session = Session::new_unsaved(album("Festa"));
    session.set_export_folder(Path::new("impressao"));
    assert!(session.approval_folder().ends_with("aprovacao-cliente"));
}

#[test]
fn ah_b16_dados_do_fotografo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(APPROVAL_FILE);
    let project = album("Festa");
    let mut store = ApprovalStore::load(&file);
    let mut form = ApprovalForm::new(&store.data, &project);
    form.whatsapp = "(11) 99999-9999".to_owned();
    form.email = "estudio@exemplo.com".to_owned();
    store.data = form.data();
    store.save().unwrap();
    let again = ApprovalForm::new(&ApprovalStore::load(&file).data, &project);
    assert_eq!(again.whatsapp, "(11) 99999-9999", "como foi digitado");
    assert_eq!(again.email, "estudio@exemplo.com");

    let old = r#"{"versao": 1, "marca_dagua_ligada": false, "marca_dagua_texto": ""}"#;
    std::fs::write(&file, old).unwrap();
    let store = ApprovalStore::load(&file);
    assert_eq!(store.warning, None);
    assert_eq!(store.data.whatsapp, "");
    assert_eq!(store.data.email, "");
}

#[test]
fn ac_b21_avisos_e_arquivos_antes_de_gerar() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("saida");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("web-lamina-01.jpg"), b"x").unwrap();
    let mut project = album("Festa");
    let spread = project.spreads[0].id;
    let rect = RelRect::new(0.1, 0.1, 0.2, 0.2).unwrap();
    project
        .add_frame(spread, rect, FrameContent::Empty)
        .unwrap();
    let session = Session::new_unsaved(project);
    let form = ApprovalForm::new(&ApprovalStore::default().data, session.album());
    let check = session
        .check_approval(&folder, &form.options(false))
        .unwrap();
    assert_eq!(check.existing, ["web-lamina-01.jpg"]);
    assert_eq!(check.warnings.len(), 1);
    assert!(matches!(check.warnings[0].kind, WarningKind::EmptyFrame));
    assert_eq!(
        check.cover.as_deref(),
        Some(
            "O álbum ainda não tem capa. No HTML, o cliente vê uma capa lisa escrita \"Capa ainda não criada\"."
        )
    );
}

#[test]
fn com_capa_ou_sem_html_nao_ha_aviso_de_capa() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = album("Festa");
    let session = Session::new_unsaved(project.clone());
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, session.album());
    form.html = false;
    let check = session
        .check_approval(dir.path(), &form.options(false))
        .unwrap();
    assert_eq!(check.cover, None, "só o JPG");
    let measures = lamina_core::CoverMeasures::defaults(&project.spec);
    project.create_cover(measures).unwrap();
    let session = Session::new_unsaved(project);
    form.html = true;
    let check = session
        .check_approval(dir.path(), &form.options(false))
        .unwrap();
    assert_eq!(check.cover, None, "com capa");
}

#[test]
fn ac_b22_foto_ausente_para_tudo_antes_de_gravar() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = album("Festa");
    let photo = project.add_photo(dir.path().join("sumiu.jpg"), 600, 400);
    let spread = project.spreads[0].id;
    let rect = RelRect::new(0.1, 0.1, 0.5, 0.5).unwrap();
    let content = FrameContent::Photo {
        photo,
        crop: lamina_core::Crop::default(),
    };
    project.add_frame(spread, rect, content).unwrap();
    let out = dir.path().join("saida");
    let form = ApprovalForm::new(&ApprovalStore::default().data, &project);
    let result = generate_approval(
        &project,
        &out,
        &form.options(false),
        &cache(dir.path()),
        &|_, _| {},
    );
    let Err(EditorError::File(ProjectFileError::MissingPhoto { name, path })) = result else {
        panic!("esperava a foto ausente: {result:?}");
    };
    assert_eq!(name, "sumiu.jpg");
    assert!(path.ends_with("sumiu.jpg"), "{path}");
    assert!(!out.exists(), "nada é gravado");
    let session = Session::new_unsaved(project);
    assert!(matches!(
        session.check_approval(&out, &form.options(false)),
        Err(EditorError::File(ProjectFileError::MissingPhoto { .. }))
    ));
}

/// A geração usa a cópia de tela do cache: um álbum grande não abre os originais.
#[test]
fn a_geracao_usa_a_copia_de_tela_do_cache() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("foto.jpg");
    let photo = image::RgbImage::from_pixel(3000, 2000, image::Rgb([200, 0, 0]));
    image::DynamicImage::ImageRgb8(photo).save(&source).unwrap();
    let cache = cache(dir.path());
    let mut project = album("Festa");
    let id = project.add_photo(&source, 3000, 2000);
    project.photos[0].fingerprint = Some(lamina_media::fingerprint::fingerprint(&source).unwrap());
    cache.ensure_images(&project.photos[0]).unwrap();
    // A cópia de tela ganha outra cor: o verde só aparece se ela desenhar a lâmina.
    let screen = cache.screen(&project.photos[0]).unwrap();
    let green = image::RgbImage::from_pixel(1600, 1067, image::Rgb([0, 200, 0]));
    image::DynamicImage::ImageRgb8(green).save(&screen).unwrap();
    let spread = project.spreads[0].id;
    let rect = RelRect::new(0.0, 0.0, 1.0, 1.0).unwrap();
    let content = FrameContent::Photo {
        photo: id,
        crop: lamina_core::Crop::default(),
    };
    project.add_frame(spread, rect, content).unwrap();
    let mut form = ApprovalForm::new(&ApprovalStore::default().data, &project);
    form.html = false;
    form.number = false;
    let out = dir.path().join("saida");
    let report =
        generate_approval(&project, &out, &form.options(false), &cache, &|_, _| {}).unwrap();
    let image = image::open(&report.files[0]).unwrap().to_rgb8();
    let middle = image.get_pixel(1024, 512);
    assert!(middle[1] > 150 && middle[0] < 60, "{middle:?}");
}

/// O nome em texto também é só do álbum: um `TextBox` qualquer não entra nos dados guardados.
#[test]
fn o_arquivo_guarda_so_os_dados_do_fotografo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(APPROVAL_FILE);
    let mut project = album("Festa Secreta");
    let spread = project.spreads[0].id;
    let text = FrameContent::Text(TextBox {
        text: "Texto do álbum".to_owned(),
        ..TextBox::new(Rgb::BLACK)
    });
    let rect = RelRect::new(0.1, 0.1, 0.5, 0.5).unwrap();
    project.add_frame(spread, rect, text).unwrap();
    let mut store = ApprovalStore::load(&file);
    store.data = ApprovalForm::new(&store.data, &project).data();
    store.save().unwrap();
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(!saved.contains("Festa Secreta"), "{saved}");
    assert!(saved.contains("\"versao\": 1"), "{saved}");
    assert!(saved.contains("marca_dagua_ligada"), "{saved}");
}
