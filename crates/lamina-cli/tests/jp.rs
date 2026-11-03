//! JP-B4 da spec `specs/janela-principal.md`: o "Novo álbum" da janela e o `lamina build` montam
//! o mesmo álbum (mesmo arquivo de projeto, byte a byte).

// Arquivo só de testes (`tests/`): as funções auxiliares também podem usar `unwrap` (AGENTS.md,
// regra 3, vale fora de testes).
#![allow(clippy::unwrap_used)]

mod common;

use common::{Base, text};
use lamina_editor::{Cache, NewAlbum, Session, SpecFields, create_album, suggestion_text};
use lamina_layout::Reuse;

#[test]
fn jp_b4_novo_album_e_o_mesmo_do_lamina_build() {
    let base = Base::with_photos(12, (300, 200));
    let out = base.build("pelo-build", &[]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains(&suggestion_text(3)));

    let fields = SpecFields {
        width_cm: 5.0,
        height_cm: 5.0,
        dpi: 72,
        ..SpecFields::default()
    };
    let new = NewAlbum {
        folder: base.path("fotos"),
        name: None,
        spec: fields.to_spec().unwrap(),
        spreads: None,
        reuse: Reuse::Medium,
        preferences: lamina_layout::Preferences::default(),
        learned: lamina_layout::Learned::default(),
        models: Vec::new(),
        limits: lamina_core::PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
        style: Default::default(),
        files: Vec::new(),
        build: true,
    };
    let (built, _) = create_album(&new, &Cache::new(base.path("cache")), &|_| {}).unwrap();
    assert_eq!(built.project.spreads.len(), 3, "a sugestão");
    let mut session = Session::new_unsaved(built.project);
    assert!(
        session.is_dirty(),
        "álbum novo nasce com alterações não salvas"
    );
    std::fs::create_dir_all(base.path("pela-janela")).unwrap();
    session
        .save_as(&base.path("pela-janela/fotos.lamina"))
        .unwrap();

    let by_build = std::fs::read(base.path("pelo-build/fotos.lamina")).unwrap();
    let by_window = std::fs::read(base.path("pela-janela/fotos.lamina")).unwrap();
    assert!(
        by_build == by_window,
        "os dois arquivos .lamina deveriam ser iguais"
    );
}
