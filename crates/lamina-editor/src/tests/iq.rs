//! Cenários da spec `inicio-rapido` (IQ): o álbum vazio com o tamanho padrão, aberto sem arquivo,
//! e o que vem depois — as fotos (IM) e o tamanho novo com os quadros readequados (PR, ADR 0003).

use lamina_core::{DesignMode, Mm, PageLimits};
use lamina_layout::Reuse;
use serde_json::json;

use super::common::Base;
use super::ia_common::Lab;
use super::tl_common::spec;
use crate::agent::ToolError;
use crate::importing::import_photos;
use crate::new_project::AlbumStyle;
use crate::quick_start::{QUICK_START_NAME, quick_start_project};
use crate::{Command, Editor};

/// As fotos da pasta `fotos` da base importadas no álbum aberto.
fn import_folder(editor: &mut Editor, base: &Base) -> usize {
    let folder = base.path("fotos");
    let report = import_photos(std::slice::from_ref(&folder), &base.cache(), &|_, _| {}).unwrap();
    editor.add_photos(&report).unwrap().added
}

#[test]
fn iq_b1_o_album_nasce_vazio_e_com_o_tamanho_padrao() {
    let project = quick_start_project().unwrap();
    assert_eq!(project.name, QUICK_START_NAME);
    assert_eq!(project.spec.page_width(), Mm::from_cm(30.0));
    assert_eq!(project.spec.page_height(), Mm::from_cm(30.0));
    assert_eq!(project.spec.dpi().get(), 300);
    assert_eq!(project.spec.mode(), DesignMode::Spread);
    assert_eq!(project.spec.bleed().outer, Mm(3.0));
    assert_eq!(project.spec.safe_margins().outer, Mm(10.0));
    assert!(
        project.photos.is_empty() && project.spreads.is_empty(),
        "sem fotos e sem lâminas"
    );
    let style = AlbumStyle::default();
    assert_eq!(project.mounting, style.mounting);
    assert_eq!(project.look, style.look);
    assert_eq!(project.gap, style.gap);
}

#[test]
fn iq_b2_abre_sem_arquivo_e_em_branco() {
    let mut editor = Editor::new();
    editor.quick_start().unwrap();
    let session = editor.session().expect("sessão aberta");
    assert!(editor.is_blank(), "álbum em branco, sem lâminas");
    assert_eq!(session.file(), None, "ainda sem arquivo");
    assert!(
        editor.needs_confirmation(),
        "álbum novo sem arquivo: fechar pergunta"
    );
    assert!(editor.available(Command::SaveAs));
    // O álbum não entra nos recentes antes de salvar (TI, regra 1).
    assert!(editor.recents().list().is_empty());
}

#[test]
fn iq_b3_trocar_o_tamanho_depois_readequa_os_quadros() {
    let base = Base::with_photos(12, (600, 400));
    let mut editor = Editor::new();
    editor.quick_start().unwrap();
    assert_eq!(import_folder(&mut editor, &base), 12);
    editor.build_album(None, Reuse::Medium).unwrap();
    assert!(!editor.is_blank(), "as lâminas foram montadas");
    let photos = editor.session().unwrap().project().photos.len();

    editor
        .respec(spec(25.0, 20.0), PageLimits::default())
        .unwrap();
    let project = editor.session().unwrap().project();
    assert_eq!(project.spec, spec(25.0, 20.0));
    assert_eq!(project.photos.len(), photos, "nenhuma foto some");
    assert!(
        project.spreads.iter().any(|s| !s.frames.is_empty()),
        "as lâminas continuam com quadros"
    );
    // Os quadros se encaixam no tamanho novo pelas frações do corte (ADR 0003).
    for frame in project.spreads.iter().flat_map(|s| &s.frames) {
        let r = frame.rect;
        assert!(r.x() >= 0.0 && r.y() >= 0.0, "quadro {r:?} fora do corte");
        assert!(
            r.x() + r.width() <= 1.0 + 1e-9 && r.y() + r.height() <= 1.0 + 1e-9,
            "quadro {r:?} passa do corte"
        );
    }
}

#[test]
fn iq_b4_as_fotos_chegam_depois() {
    let base = Base::with_photos(3, (600, 400));
    let mut editor = Editor::new();
    editor.quick_start().unwrap();
    assert_eq!(import_folder(&mut editor, &base), 3);
    let project = editor.session().unwrap().project();
    assert_eq!(project.photos.len(), 3);
    let names: Vec<String> = project
        .photos
        .iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["f00.jpg", "f01.jpg", "f02.jpg"], "em ordem");
    assert!(project.spreads.is_empty(), "importar não monta lâminas");
    // Uma edição só, que Ctrl+Z desfaz (IM, regra 1).
    assert!(editor.run(Command::Undo));
    assert!(editor.session().unwrap().project().photos.is_empty());
}

#[test]
fn iq_b5_o_comando_chega_a_janela_e_a_mais_ninguem() {
    let mut editor = Editor::new();
    assert!(editor.available(Command::QuickStart), "sem álbum aberto");
    editor.quick_start().unwrap();
    assert!(editor.available(Command::QuickStart), "com o álbum vazio");
    assert_eq!(
        Command::from_name("arquivo.inicio-rapido"),
        Some(Command::QuickStart)
    );
    assert_eq!(Command::QuickStart.label(), "Início rápido");
    assert_eq!(
        Command::QuickStart.shortcut(),
        None,
        "Ctrl+N é do assistente"
    );

    // A IA não cria álbum vazio pelo comando: ele é só da janela (regra 6).
    let base = Base::with_photos(30, (600, 400));
    let (mut lab, _) = Lab::opened(&base, false);
    let err = lab
        .call(
            "executar_comando",
            json!({ "comando": "arquivo.inicio-rapido" }),
        )
        .unwrap_err();
    assert!(matches!(err, ToolError::WindowOnly(_)));
    assert_eq!(
        err.to_string(),
        "O comando arquivo.inicio-rapido só existe na janela do Lamina."
    );
}

#[test]
fn iq_b6_o_botao_comecar_e_o_menu_chamam_o_comando() {
    // O que só dá para conferir lendo as telas mora nos testes do `lamina-app`
    // (`crates/lamina-app/tests/iq.rs`); aqui, o contrato dos botões da tela inicial (DI).
    let first = crate::recents::welcome_buttons(false);
    assert_eq!(
        first.body,
        [Command::NewAlbum, Command::QuickStart, Command::OpenExample]
    );
    assert_eq!(first.bar, [Command::Open]);
    let returning = crate::recents::welcome_buttons(true);
    assert!(returning.body.is_empty());
    assert_eq!(
        returning.bar,
        [
            Command::OpenExample,
            Command::Open,
            Command::NewAlbum,
            Command::QuickStart
        ]
    );
}
