//! A capa no núcleo (spec CA): arquivo, medidas, projeção e reajuste (CA-B1, B2, B3, B13, B17, B18,
//! B21, B24 e B27). O resto dos cenários roda no editor e na exportação.

use serde_json::Value;

use crate::{
    AlbumSpec, CoreError, Cover, CoverGuide, CoverMeasures, CoverOrigin, Dpi, FrameContent, Mm,
    Project, RelRect, Spine,
};

fn album(spreads: usize) -> Project {
    let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    for _ in 0..spreads {
        project.add_spread();
    }
    project
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn with_cover(spreads: usize) -> Project {
    let mut project = album(spreads);
    let measures = CoverMeasures::defaults(&project.spec);
    project.create_cover(measures).unwrap();
    project
}

#[test]
fn ca_b1_formato_8_regrava_sem_capa_e_com_capa() {
    let without = album(2);
    let mut v7: Value = serde_json::from_str(&without.to_json().unwrap()).unwrap();
    v7["schema_version"] = serde_json::json!(7);
    let opened = Project::from_json(&v7.to_string()).unwrap();
    let saved = opened.to_json().unwrap();
    assert!(saved.contains("\"schema_version\": 8"));
    assert!(!saved.contains("\"capa\""), "{saved}");

    let with = with_cover(2);
    let saved = with.to_json().unwrap();
    assert!(saved.contains("\"capa\"") && saved.contains("\"medidas\""));
    let reopened = Project::from_json(&saved).unwrap();
    assert_eq!(reopened, with);
    assert_eq!(reopened.to_json().unwrap(), saved);
}

#[test]
fn ca_b2_medidas_ja_preenchidas() {
    let project = album(15);
    let m = CoverMeasures::defaults(&project.spec);
    assert!(close(m.face_width.get(), 303.0));
    assert!(close(m.face_height.get(), 306.0));
    assert!(close(m.spine_width(project.page_count()).get(), 15.0));
    assert!(close(m.wrap.get(), 15.0));
    assert!(close(m.safe.get(), 10.0));
    assert_eq!(m.origin, CoverOrigin::Album);
}

#[test]
fn ca_b3_a_especificacao_da_capa() {
    let project = album(15);
    let m = CoverMeasures::defaults(&project.spec);
    let spec = m.spec(&project.spec, project.page_count()).unwrap();
    let trim = spec.trim_rect();
    assert!(close(trim.width.get(), 621.0));
    assert!(close(trim.height.get(), 306.0));
    let canvas = spec.canvas_rect();
    assert!(close(canvas.width.get(), 651.0));
    assert!(close(canvas.height.get(), 336.0));
    // As linhas da lombada, a partir da borda esquerda do corte.
    let half = 15.0 / 2.0;
    let middle = trim.width.get() / 2.0;
    assert!(close(middle - half, 303.0));
    assert!(close(middle + half, 318.0));
    // A zona segura não entra na lombada.
    let left = spec.safe_rect(crate::PageSide::Left);
    let left_edge = left.x.get() + left.width.get() - trim.x.get();
    assert!(left_edge <= 303.0 + 1e-6, "{left_edge}");
    let right = spec.safe_rect(crate::PageSide::Right);
    assert!(right.x.get() - trim.x.get() >= 318.0 - 1e-6);
}

#[test]
fn a_capa_fica_fora_das_laminas_e_da_contagem() {
    let project = with_cover(10);
    assert_eq!(project.spreads.len(), 10);
    assert_eq!(project.page_count(), 20);
    let ids: Vec<_> = project.spreads.iter().map(|s| s.id).collect();
    let cover = project.cover.as_ref().unwrap();
    assert!(!ids.contains(&cover.content.id));
}

#[test]
fn ca_b13_arquivo_com_medida_invalida_nao_abre() {
    let mut project = with_cover(2);
    project.cover.as_mut().unwrap().measures.spine = Spine::Fixed {
        mm: Mm(-10.0),
        pages: 4,
    };
    let json = project.to_json().unwrap();
    assert!(matches!(
        Project::from_json(&json),
        Err(CoreError::InvalidCover {
            field: "lombada",
            ..
        })
    ));
}

#[test]
fn ca_b13_limites_das_medidas() {
    let project = album(2);
    let good = CoverMeasures::defaults(&project.spec);
    let cases = [
        (
            CoverMeasures {
                face_width: Mm(30.0),
                ..good
            },
            "face",
        ),
        (
            CoverMeasures {
                spine: Spine::Fixed {
                    mm: Mm(-10.0),
                    pages: 4,
                },
                ..good
            },
            "lombada",
        ),
        (
            CoverMeasures {
                spine: Spine::Fixed {
                    mm: Mm(120.0),
                    pages: 4,
                },
                ..good
            },
            "lombada",
        ),
        (
            CoverMeasures {
                safe: Mm(200.0),
                ..good
            },
            "zona_segura",
        ),
    ];
    for (measures, expected) in cases {
        assert!(
            matches!(measures.validate(), Err(CoreError::InvalidCover { field, .. }) if field == expected),
            "{expected}"
        );
    }
    good.validate().unwrap();
}

#[test]
fn ca_b17_medida_total_sem_a_dobra() {
    let m = CoverMeasures::from_total(
        (Mm(621.0), Mm(306.0)),
        (Mm(15.0), Mm(15.0), Mm(10.0)),
        false,
        30,
    )
    .unwrap();
    assert!(close(m.face_width.get(), 303.0));
    assert!(close(m.face_height.get(), 306.0));
    assert_eq!(
        m.spine,
        Spine::Fixed {
            mm: Mm(15.0),
            pages: 30
        }
    );
    assert!(matches!(
        m.origin,
        CoverOrigin::Total {
            includes_wrap: false,
            ..
        }
    ));
}

#[test]
fn ca_b18_medida_total_com_a_dobra() {
    let m = CoverMeasures::from_total(
        (Mm(651.0), Mm(336.0)),
        (Mm(15.0), Mm(15.0), Mm(10.0)),
        true,
        30,
    )
    .unwrap();
    assert!(close(m.face_width.get(), 303.0));
    assert!(close(m.face_height.get(), 306.0));
    let project = album(15);
    let canvas = m.spec(&project.spec, 30).unwrap().canvas_rect();
    assert!(close(canvas.width.get(), 651.0));
    assert!(close(canvas.height.get(), 336.0));
}

#[test]
fn total_que_nao_comporta_a_lombada_e_recusado() {
    assert!(matches!(
        CoverMeasures::from_total(
            (Mm(20.0), Mm(300.0)),
            (Mm(15.0), Mm(15.0), Mm(10.0)),
            true,
            30
        ),
        Err(CoreError::InvalidCover { field: "total", .. })
    ));
}

#[test]
fn ca_b21_o_caminho_do_guia_acompanha_o_album() {
    let mut project = with_cover(1);
    let root = std::env::temp_dir().join("lamina-ca-b21");
    let guide = root.join("a").join("guia.png");
    project.cover.as_mut().unwrap().guide = Some(CoverGuide {
        path: guide,
        includes_wrap: true,
        visible: true,
    });
    project.relativize(&root.join("a"));
    let stored = &project.cover.as_ref().unwrap().guide.as_ref().unwrap().path;
    assert_eq!(stored, std::path::Path::new("guia.png"));
    project.resolve_paths(&root.join("b"));
    let resolved = &project.cover.as_ref().unwrap().guide.as_ref().unwrap().path;
    assert_eq!(resolved, &root.join("b").join("guia.png"));
}

/// Um quadro vazio de `x` a `x + w` mm, na largura aberta `open`.
fn frame(project: &mut Project, x: f64, w: f64, open: f64) {
    let rect = RelRect::new(x / open, 0.25, w / open, 0.5).unwrap();
    let cover = project.cover.as_mut().unwrap();
    let id = crate::FrameId(90_000 + cover.content.frames.len() as u64);
    cover.content.frames.push(crate::Frame {
        id,
        rect,
        content: FrameContent::Empty,
        border: crate::Border::default(),
        radius: Mm(0.0),
        opacity: crate::Opacity::FULL,
    });
}

fn edges_mm(cover: &Cover, index: usize, open: f64) -> (f64, f64) {
    let r = cover.content.frames[index].rect;
    (r.x() * open, (r.x() + r.width()) * open)
}

#[test]
fn ca_b24_reajuste_dos_tres_tipos_de_quadro() {
    let mut project = with_cover(15);
    // Capa de 621 mm: verso até 303, lombada de 303 a 318, frente depois.
    frame(&mut project, 400.0, 100.0, 621.0); // na frente
    frame(&mut project, 305.0, 11.0, 621.0); // dentro da lombada
    frame(&mut project, 50.0, 500.0, 621.0); // cruza as duas linhas
    let before = project.cover.as_ref().unwrap().measures;
    let spine = project.cover_spine().unwrap();
    project.add_spread();
    project.refit_cover(&before, spine);
    let cover = project.cover.as_ref().unwrap();
    let open = 2.0 * 303.0 + 16.0;

    // A frente desloca 1 mm e guarda o tamanho e a distância da borda de fora.
    let (a, b) = edges_mm(cover, 0, open);
    assert!(close(b - a, 100.0));
    assert!(close(open - b, 121.0));
    // A lombada continua com o quadro centrado.
    let (a, b) = edges_mm(cover, 1, open);
    assert!(close(b - a, 11.0));
    assert!(close((a + b) / 2.0, open / 2.0));
    // A foto que cruza estica: a borda do verso fica, e a da frente anda 1 mm.
    let (a, b) = edges_mm(cover, 2, open);
    assert!(close(a, 50.0));
    assert!(close(b, 551.0));
}

#[test]
fn ca_b27_a_origem_vai_e_volta_do_arquivo() {
    let mut project = with_cover(15);
    let m = CoverMeasures::from_total(
        (Mm(651.0), Mm(336.0)),
        (Mm(15.0), Mm(15.0), Mm(10.0)),
        true,
        30,
    )
    .unwrap();
    project.cover.as_mut().unwrap().measures = m;
    let reopened = Project::from_json(&project.to_json().unwrap()).unwrap();
    assert_eq!(reopened.cover.as_ref().unwrap().measures.origin, m.origin);
}

#[test]
fn a_projecao_edita_e_volta_ao_projeto() {
    let mut project = with_cover(3);
    let mut canvas = project.cover_canvas().unwrap().unwrap();
    assert_eq!(canvas.spreads.len(), 1);
    assert!(canvas.cover.is_none());
    let id = canvas.spreads[0].id;
    let rect = RelRect::new(0.1, 0.1, 0.2, 0.2).unwrap();
    canvas.add_frame(id, rect, FrameContent::Empty).unwrap();
    let next = canvas.add_spread();
    canvas.spreads.pop();
    project.absorb_cover(canvas);
    assert_eq!(project.spreads.len(), 3, "as lâminas não mudam");
    assert_eq!(project.cover.as_ref().unwrap().content.frames.len(), 1);
    // Os ids continuam de onde a projeção parou.
    assert!(project.add_spread().get() > next.get());
}

#[test]
fn absorver_uma_lamina_que_nao_e_a_capa_nao_troca_a_capa() {
    let mut project = with_cover(1);
    let before = project.cover.clone();
    let mut canvas = project.cover_canvas().unwrap().unwrap();
    canvas.add_spread();
    project.absorb_cover(canvas);
    assert_eq!(project.cover, before);
}

#[test]
fn foto_que_preenche_a_face_continua_no_corte_quando_a_face_muda() {
    let mut project = with_cover(15);
    // Frente de 318 a 621 mm, de cima a baixo.
    let cover = project.cover.as_mut().unwrap();
    let face = 303.0 / 621.0;
    cover.content.frames.push(crate::Frame {
        id: crate::FrameId(90_000),
        rect: RelRect::new(318.0 / 621.0, 0.0, face, 1.0).unwrap(),
        content: FrameContent::Empty,
        border: crate::Border::default(),
        radius: Mm(0.0),
        opacity: crate::Opacity::FULL,
    });
    let before = cover.measures;
    let spine = project.cover_spine().unwrap();
    let cover = project.cover.as_mut().unwrap();
    cover.measures.face_width = Mm(310.0);
    cover.measures.face_height = Mm(320.0);
    project.refit_cover(&before, spine);
    let rect = project.cover.as_ref().unwrap().content.frames[0].rect;
    assert!((rect.x() + rect.width() - 1.0).abs() < 1e-9, "{rect:?}");
    assert!(
        rect.y().abs() < 1e-9 && (rect.height() - 1.0).abs() < 1e-9,
        "{rect:?}"
    );
    let open = 2.0 * 310.0 + 15.0;
    assert!((rect.x() * open - 325.0).abs() < 1e-6, "{rect:?}");
}
