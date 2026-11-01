//! Cenários da spec `capa` (CA) que rodam no editor: criar, editar, numeração, soltar e medir. Os do
//! arquivo e das contas ficam no `lamina-core`, e os da exportação, no `lamina-export`.

use lamina_core::{
    BlankPages, CoverMeasures, CoverOrigin, FrameContent, Mm, PageSide, PhotoId, Project, Spine,
};
use lamina_layout::BuildOptions;

use super::tl_common::{H, built, editor_at, spec, three_spreads};
use crate::building::build_unused;
use crate::cover_form::{CoverForm, CoverTab, GuideFile};
use crate::edits::spread_photos;
use crate::motion::{spread_badge, strip_rows};
use crate::photos::{Filter, photo_list};
use crate::{Command, Editor};

fn last_label(editor: &Editor) -> String {
    let list = editor.session().unwrap().history_list();
    list.steps.last().unwrap().label.clone()
}

fn album(editor: &Editor) -> &Project {
    editor.session().unwrap().album()
}

fn on_cover_with(project: Project, photos: &[PhotoId]) -> Editor {
    let mut editor = editor_at(project, 0);
    let measures = CoverMeasures::defaults(&album(&editor).spec);
    assert!(editor.create_cover(measures, None, photos));
    editor
}

fn cover_frames(editor: &Editor) -> usize {
    album(editor).cover.as_ref().unwrap().content.frames.len()
}

fn free_photos(project: &Project, n: usize) -> Vec<PhotoId> {
    project.photos.iter().map(|p| p.id).take(n).collect()
}

#[test]
fn ca_b2_caixa_ja_preenchida() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let editor = editor_at(project, 0);
    let form = editor.cover_form().unwrap();
    assert_eq!(form.tab, CoverTab::Album);
    assert_eq!(form.face_width, "30,3");
    assert_eq!(form.face_height, "30,6");
    assert_eq!(form.wrap, "1,5");
    assert_eq!(form.safe, "1");
    assert_eq!(
        form.summary().as_deref(),
        Some("Capa aberta: 62,1 × 30,6 cm, com lombada de 1,5 cm (15 folhas × 1,0 mm)")
    );
    let drawing = form.drawing().unwrap();
    assert!((drawing.aspect - 651.0 / 336.0).abs() < 1e-9);
    assert!(drawing.back.x < drawing.spine.x && drawing.spine.x < drawing.front.x);
}

#[test]
fn ca_b5_criar_soltando_fotos() {
    let project = three_spreads();
    let two = free_photos(&project, 2);
    let mut editor = on_cover_with(project.clone(), &two);
    assert_eq!(last_label(&editor), "Criar a capa");
    assert!(editor.on_cover());
    assert_eq!(cover_frames(&editor), 2);
    // O motor respeita a lombada: nenhum quadro entra nas duas linhas.
    let spine = album(&editor).cover_spine().unwrap().get();
    let cover = album(&editor).cover.as_ref().unwrap();
    let open = cover.measures.open_width(album(&editor).page_count()).get();
    for f in &cover.content.frames {
        let (a, b) = (f.rect.x() * open, (f.rect.x() + f.rect.width()) * open);
        let middle = open / 2.0;
        let crosses = a < middle + spine / 2.0 && b > middle - spine / 2.0;
        assert!(!crosses, "{a} a {b} cruza a lombada");
    }
    // As fotos contam como usadas: a capa é mais um uso delas.
    let items = photo_list(album(&editor), Filter::All);
    let first = items.iter().find(|p| p.id == two[0]).unwrap();
    assert_eq!(first.uses, 2);
    assert!(editor.run(Command::Undo));
    assert!(album(&editor).cover.is_none());
    assert!(!editor.on_cover());
}

#[test]
fn ca_b6_editar_na_capa() {
    let project = three_spreads();
    let one = free_photos(&project, 1);
    let mut editor = on_cover_with(project, &one);
    assert!(editor.new_text(None));
    assert_eq!(last_label(&editor), "Novo texto · capa");
    for command in [
        Command::Mirror,
        Command::MoveSpreadLeft,
        Command::DeleteSpread,
        Command::NewSpread,
        Command::BlankPage,
    ] {
        assert!(!editor.available(command), "{command:?}");
    }
    // Sair e desfazer volta para a capa.
    assert!(editor.run(Command::NextSpread));
    assert!(!editor.on_cover());
    assert!(editor.run(Command::Undo));
    assert!(
        editor.on_cover(),
        "desfazer uma edição da capa volta para a capa"
    );
    assert_eq!(cover_frames(&editor), 1);
    assert_eq!(album(&editor).spreads.len(), 3, "as lâminas não mudam");
}

#[test]
fn ca_b8_fora_da_numeracao() {
    let project = built(spec(30.0, 30.0), &[H; 40], 10);
    let mut editor = on_cover_with(project, &[]);
    let session = editor.session().unwrap();
    assert!(
        session.counter().starts_with("10 lâminas (20 páginas)"),
        "{}",
        session.counter()
    );
    assert_eq!(strip_rows(session).len(), 10);
    assert_eq!(
        spread_badge(album(&editor).spec.edges(), 0, 10),
        "Lâmina 1 · páginas 1 e 2"
    );
    // → na capa vai para a lâmina 1; ← na lâmina 1 volta; Home vai para a lâmina 1.
    assert!(editor.run(Command::NextSpread));
    assert!(!editor.on_cover());
    assert_eq!(editor.session().unwrap().current(), 0);
    assert!(editor.run(Command::PreviousSpread));
    assert!(editor.on_cover());
    assert!(!editor.available(Command::PreviousSpread));
    assert!(editor.run(Command::FirstSpread));
    assert!(!editor.on_cover());
    assert_eq!(editor.session().unwrap().current(), 0);
    assert!(editor.run(Command::LastSpread));
    assert_eq!(editor.session().unwrap().current(), 9);
}

#[test]
fn ca_b12_montar_o_album_nao_toca_na_capa() {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    for i in 0..12 {
        project.add_photo(format!("f{i:02}.jpg"), 3000, 2000);
    }
    let measures = CoverMeasures::defaults(&project.spec);
    project.create_cover(measures).unwrap();
    let before = project.cover.clone();
    let (out, _) = build_unused(&project, 3, &BuildOptions::default()).unwrap();
    assert_eq!(out.spreads.len(), 3);
    assert_eq!(out.cover, before);
}

#[test]
fn ca_b13_mensagens_da_caixa() {
    let project = three_spreads();
    let editor = editor_at(project, 0);
    let good = editor.cover_form().unwrap();
    let with = |change: &dyn Fn(&mut CoverForm)| {
        let mut form = good.clone();
        change(&mut form);
        form.measures().err()
    };
    assert_eq!(
        with(&|f| f.face_width = "3".to_owned()).as_deref(),
        Some("Confira as medidas: a face da capa vai de 5 a 100 cm.")
    );
    let fixed = |text: &'static str| {
        with(&move |f: &mut CoverForm| {
            f.spine_auto = false;
            text.clone_into(&mut f.spine);
        })
    };
    assert_eq!(
        fixed("-1").as_deref(),
        Some("Confira as medidas: a lombada vai de 0 a 10 cm.")
    );
    assert_eq!(
        fixed("12").as_deref(),
        Some("Confira as medidas: a lombada vai de 0 a 10 cm.")
    );
    assert_eq!(
        with(&|f| f.safe = "20".to_owned()).as_deref(),
        Some("Confira as medidas: a zona segura precisa ser menor que metade da face.")
    );
}

#[test]
fn ca_b17_e_b18_aba_da_medida_total() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    form.tab = CoverTab::Total;
    "62,1".clone_into(&mut form.total_width);
    "30,6".clone_into(&mut form.total_height);
    "1,5".clone_into(&mut form.spine);
    let m = form.measures().unwrap();
    assert!((m.face_width.get() - 303.0).abs() < 1e-6);
    assert_eq!(
        m.spine,
        Spine::Fixed {
            mm: Mm(15.0),
            pages: 30
        }
    );
    form.includes_wrap = true;
    "65,1".clone_into(&mut form.total_width);
    "33,6".clone_into(&mut form.total_height);
    let m = form.measures().unwrap();
    assert!((m.face_width.get() - 303.0).abs() < 1e-6);
    assert!((m.face_height.get() - 306.0).abs() < 1e-6);
}

#[test]
fn ca_b19_arquivo_guia_com_dpi() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    form.tab = CoverTab::Guide;
    form.set_guide(GuideFile {
        path: "guia.tif".into(),
        size_px: (7689, 3969),
        dpi: Some(300.0),
    });
    assert_eq!(form.total_width, "65,1");
    assert_eq!(form.total_height, "33,6");
    assert!(!form.needs_guide_width());
    "1,5".clone_into(&mut form.spine);
    form.includes_wrap = true;
    let m = form.measures().unwrap();
    assert!((m.face_width.get() - 303.0).abs() < 0.1);
    assert_eq!(m.origin, CoverOrigin::Guide);
    assert!(form.guide_to_save().is_some());
}

#[test]
fn ca_b20_arquivo_guia_sem_dpi() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    form.tab = CoverTab::Guide;
    form.set_guide(GuideFile {
        path: "guia.png".into(),
        size_px: (6510, 3360),
        dpi: None,
    });
    assert!(form.needs_guide_width());
    form.set_guide_width("65,1");
    assert_eq!(form.total_width, "65,1");
    assert_eq!(form.total_height, "33,6");
}

#[test]
fn ca_b22_guia_de_outra_proporcao_avisa_na_caixa() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    form.tab = CoverTab::Guide;
    "1,5".clone_into(&mut form.spine);
    form.includes_wrap = true;
    form.set_guide(GuideFile {
        path: "guia.tif".into(),
        size_px: (7689, 3969),
        dpi: Some(300.0),
    });
    assert!(form.warnings().is_empty(), "{:?}", form.warnings());
    // A mesma imagem, 2% mais larga que a capa digitada.
    form.set_guide(GuideFile {
        path: "guia.tif".into(),
        size_px: (7843, 3969),
        dpi: None,
    });
    assert_eq!(form.warnings().len(), 1);
}

#[test]
fn ca_b23_lombada_fixa_so_avisa_quando_as_paginas_mudam() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let mut editor = editor_at(project, 0);
    let mut measures = CoverMeasures::defaults(&album(&editor).spec);
    measures.spine = Spine::Fixed {
        mm: Mm(12.0),
        pages: 30,
    };
    assert!(editor.create_cover(measures, None, &[]));
    let texts = |e: &Editor| {
        lamina_export::cover_preflight(album(e))
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
    assert!(texts(&editor).is_empty(), "{:?}", texts(&editor));
    assert!(editor.run(Command::NextSpread));
    assert!(editor.run(Command::NewSpread));
    assert_eq!(
        texts(&editor),
        ["Capa: o álbum tinha 30 páginas quando a lombada foi fixada; agora tem 32."]
    );
}

#[test]
fn ca_b24_reajuste_entra_na_mesma_edicao() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let photo = project.photos[0].id;
    let mut editor = on_cover_with(project, &[]);
    assert!(editor.drop_on_cover(0.9, &[photo]));
    let before = album(&editor).clone();
    assert!(editor.run(Command::NextSpread));
    assert!(editor.run(Command::NewSpread));
    assert!(
        last_label(&editor).ends_with("lombada 1,5 → 1,6 cm"),
        "{}",
        last_label(&editor)
    );
    let now = album(&editor).cover.as_ref().unwrap();
    assert_ne!(
        now.content.frames[0].rect,
        before.cover.as_ref().unwrap().content.frames[0].rect
    );
    assert!(editor.run(Command::Undo));
    assert_eq!(
        album(&editor),
        &before,
        "um Ctrl+Z volta tudo, com as frações exatas"
    );
}

#[test]
fn ca_b25_capa_menor_que_a_pagina() {
    let project = three_spreads();
    let mut editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    "29".clone_into(&mut form.face_width);
    "30".clone_into(&mut form.face_height);
    let expected = "Capa: a face (29,0 cm) é menor que a página (30,0 cm).";
    assert_eq!(form.warnings(), [expected]);
    assert!(editor.create_cover(form.measures().unwrap(), None, &[]));
    let warnings = lamina_export::cover_preflight(album(&editor));
    assert_eq!(warnings[0].to_string(), expected);
}

#[test]
fn ca_b26_soltar_na_capa_aberta() {
    let project = three_spreads();
    let ids = free_photos(&project, 3);
    let mut editor = on_cover_with(project, &[]);
    // A capa tem dobra de 15 mm: a frente fica à direita do meio, e o verso, à esquerda.
    assert!(editor.drop_on_cover(0.9, &[ids[0]]));
    assert!(editor.drop_on_cover(0.1, &[ids[1]]));
    let frames = &album(&editor).cover.as_ref().unwrap().content.frames;
    assert_eq!(frames.len(), 2);
    let rect = |photo: PhotoId| {
        frames
            .iter()
            .find(|f| matches!(f.content, FrameContent::Photo { photo: p, .. } if p == photo))
            .unwrap()
            .rect
    };
    let front = rect(ids[0]);
    assert!(front.x() > 0.5 && (front.x() + front.width() - 1.0).abs() < 1e-9);
    assert!((front.height() - 1.0).abs() < 1e-9);
    let back = rect(ids[1]);
    assert!(back.x().abs() < 1e-9 && back.x() + back.width() < 0.5);
    // Na lombada, cobre a capa inteira.
    assert!(editor.drop_on_cover(0.5, &[ids[2]]));
    let frames = &album(&editor).cover.as_ref().unwrap().content.frames;
    assert_eq!(frames.len(), 1);
    assert!((frames[0].rect.width() - 1.0).abs() < 1e-9);
}

#[test]
fn ca_b27_a_caixa_reabre_na_aba_de_origem() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let mut editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    form.tab = CoverTab::Total;
    "65,1".clone_into(&mut form.total_width);
    "33,6".clone_into(&mut form.total_height);
    "1,5".clone_into(&mut form.spine);
    form.includes_wrap = true;
    assert!(editor.create_cover(form.measures().unwrap(), None, &[]));
    let again = editor.cover_form().unwrap();
    assert_eq!(again.tab, CoverTab::Total);
    assert_eq!(again.total_width, "65,1");
    assert_eq!(again.total_height, "33,6");
    assert!(again.includes_wrap);
}

#[test]
fn ca_b28_uma_foto_no_espaco_da_capa() {
    let project = three_spreads();
    let one = free_photos(&project, 1);
    let editor = on_cover_with(project, &one);
    assert_eq!(last_label(&editor), "Criar a capa");
    let frames = &album(&editor).cover.as_ref().unwrap().content.frames;
    assert_eq!(frames.len(), 1);
    assert!((frames[0].rect.width() - 1.0).abs() < 1e-9);
    assert!((frames[0].rect.height() - 1.0).abs() < 1e-9);
}

#[test]
fn apagar_a_capa_e_desfazer() {
    let project = three_spreads();
    let one = free_photos(&project, 1);
    let mut editor = on_cover_with(project, &one);
    assert!(editor.run(Command::DeleteCover));
    assert_eq!(last_label(&editor), "Apagar a capa");
    assert!(album(&editor).cover.is_none());
    assert!(!editor.on_cover());
    assert!(!editor.available(Command::GoToCover));
    assert!(editor.run(Command::Undo));
    assert!(album(&editor).cover.is_some());
    assert!(editor.on_cover());
}

#[test]
fn tirar_foto_do_album_tira_da_capa_e_o_arquivo_continua_valido() {
    let project = three_spreads();
    let one = free_photos(&project, 1);
    let mut editor = on_cover_with(project, &one);
    assert!(editor.remove_from_album(&one));
    assert!(!editor.on_cover(), "só vale no álbum: a capa fecha");
    let p = album(&editor);
    assert!(p.cover.as_ref().unwrap().content.frames.is_empty());
    assert!(spread_photos(p, 0).iter().all(|id| *id != one[0]));
    p.validate().unwrap();
    let json = p.to_json().unwrap();
    Project::from_json(&json).unwrap();
}

#[test]
fn pagina_em_branco_nao_conta_para_a_capa_de_outro_jeito() {
    // A lombada calculada conta a página em branco, como a impressão (spec PB, regra 2).
    let mut project = built(spec(30.0, 30.0), &[H; 8], 2);
    project.spreads[1].blank = BlankPages {
        left: true,
        right: false,
    };
    let measures = CoverMeasures::defaults(&project.spec);
    assert!((measures.spine_width(project.page_count()).get() - 2.0).abs() < 1e-9);
    let _ = PageSide::Left;
}

#[test]
fn ca_b14_ia() {
    use serde_json::json;

    use super::common::Base;
    use super::ia_common::Lab;

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
    let message = |r: Result<crate::agent::ToolReply, crate::agent::ToolError>| {
        r.err().map(|e| e.to_string())
    };
    let steps = lab.labels().len();
    let place = json!({ "capa": true, "quadro": 1, "foto": 3 });
    assert_eq!(
        message(lab.call("colocar_foto", place.clone())).as_deref(),
        Some("O álbum ainda não tem capa. Use criar_capa.")
    );
    assert_eq!(lab.labels().len(), steps, "a recusa não vira passo");

    lab.ok("criar_capa", json!({}));
    assert!(lab.project_album().cover.is_some());
    lab.ok("colocar_foto", place);
    let seen = lab.ok("ver_lamina", json!({ "capa": true }));
    assert!(!seen.images.is_empty(), "devolve a imagem da capa");
    assert_eq!(
        message(lab.call(
            "colocar_foto",
            json!({ "capa": true, "lamina": 2, "quadro": 1, "foto": 3 })
        ))
        .as_deref(),
        Some("Use capa ou lamina, não os dois.")
    );
    let labels = lab.labels();
    assert_eq!(labels[steps], "IA: Criar a capa");
    assert_eq!(labels[steps + 1], "IA: Colocar foto · capa");

    let summary = lab.ok("resumo_do_album", json!({}));
    let capa = &summary.data["capa"];
    assert_eq!(capa["fotos"], json!([3]), "{capa}");
    assert!(capa["face_cm"].is_array());
}

#[test]
fn ca_b16_lombada_e_vista_da_capa_rapido() {
    use std::time::{Duration, Instant};

    use lamina_core::testing::{TRIES, budget};

    use crate::view::{Area, spread_view};

    let project = built(spec(30.0, 30.0), &[H; 160], 40);
    let photos = free_photos(&project, 6);
    let area = Area {
        width: 1600.0,
        height: 900.0,
        margin: 0.0,
    };
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        let mut editor = on_cover_with(project.clone(), &photos);
        assert!(editor.run(Command::NextSpread));
        assert!(editor.run(Command::NewSpread), "a lombada recalcula");
        assert!(editor.go_to_cover());
        let started = Instant::now();
        let s = editor.session().unwrap();
        assert!(spread_view(s.project(), 0, area, &[]).is_some());
        best = best.min(started.elapsed());
    }
    assert!(best <= budget(16), "{best:?}");
}

#[test]
fn ca_b4_a_lombada_acompanha_o_album() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let photo = project.photos[0].id;
    let mut editor = on_cover_with(project, &[]);
    assert!(editor.drop_on_cover(0.9, &[photo]));
    let measures = album(&editor).cover.as_ref().unwrap().measures;
    let right_gap = |e: &Editor| {
        let cover = album(e).cover.as_ref().unwrap();
        let open = cover.measures.open_width(album(e).page_count()).get();
        let rect = cover.content.frames[0].rect;
        (rect.width() * open, open - (rect.x() + rect.width()) * open)
    };
    let before = right_gap(&editor);
    assert!(editor.run(Command::NextSpread));
    for _ in 0..5 {
        assert!(editor.run(Command::NewSpread));
    }
    assert!((album(&editor).cover_spine().unwrap().get() - 20.0).abs() < 1e-6);
    let now = right_gap(&editor);
    assert!((now.0 - before.0).abs() < 1e-6, "mesmo tamanho");
    assert!(
        (now.1 - before.1).abs() < 1e-6,
        "mesma distância da borda direita"
    );

    // Com a medida fixa, a lombada para, e a pré-impressão avisa quando o álbum muda.
    let mut fixed = measures;
    fixed.spine = Spine::Fixed {
        mm: Mm(15.0),
        pages: album(&editor).page_count(),
    };
    assert!(editor.set_cover_measures(fixed, None));
    for _ in 0..5 {
        assert!(editor.run(Command::NewSpread));
    }
    assert!((album(&editor).cover_spine().unwrap().get() - 15.0).abs() < 1e-6);
    let warnings = lamina_export::cover_preflight(album(&editor));
    assert_eq!(
        warnings[0].to_string(),
        "Capa: o álbum tinha 40 páginas quando a lombada foi fixada; agora tem 50."
    );
}

#[test]
fn soltar_fotos_em_lamina_nova_com_a_capa_aberta_nao_troca_a_capa() {
    use crate::DropPlace;

    let project = three_spreads();
    let ids = free_photos(&project, 3);
    let mut editor = on_cover_with(project, &ids[..1]);
    let before = album(&editor).cover.clone();
    assert!(editor.drop_photos(DropPlace::NewSpread(1), &ids[1..]));
    assert!(!editor.on_cover(), "a lâmina nova é no álbum");
    assert_eq!(album(&editor).cover, before, "a capa fica como estava");
    assert_eq!(album(&editor).spreads.len(), 4);
    assert_eq!(spread_photos(album(&editor), 1).len(), 2);
}

#[test]
fn clicar_numa_lamina_da_faixa_sai_da_capa() {
    let mut editor = on_cover_with(three_spreads(), &[]);
    assert!(editor.go_to_spread(2));
    assert!(!editor.on_cover());
    assert_eq!(editor.session().unwrap().current(), 2);
}

#[test]
fn aviso_da_capa_sobrevive_a_edicao_de_lamina() {
    let project = three_spreads();
    let mut editor = editor_at(project, 0);
    let mut measures = CoverMeasures::defaults(&album(&editor).spec);
    measures.face_width = Mm(290.0);
    assert!(editor.create_cover(measures, None, &[]));
    assert!(editor.session().unwrap().cover_warned());
    assert!(editor.run(Command::NextSpread));
    // Uma edição só de lâmina: os avisos da capa não entram na conta das lâminas.
    assert!(editor.new_text(None));
    assert!(editor.session().unwrap().cover_warned());
}

#[test]
fn ca_b12_as_fotos_da_capa_nao_entram_em_laminas_novas() {
    let mut project = Project::new("teste", spec(30.0, 30.0));
    for i in 0..12 {
        project.add_photo(format!("f{i:02}.jpg"), 3000, 2000);
    }
    let measures = CoverMeasures::defaults(&project.spec);
    let mut editor = Editor::new();
    editor.adopt(project);
    let two = free_photos(album(&editor), 2);
    assert!(editor.create_cover(measures, None, &two));
    let (out, _) = build_unused(album(&editor), 3, &BuildOptions::default()).unwrap();
    assert_eq!(out.cover, album(&editor).cover);
    for index in 0..out.spreads.len() {
        let used = spread_photos(&out, index);
        assert!(!used.contains(&two[0]) && !used.contains(&two[1]));
    }
}

#[test]
fn reconfirmar_a_caixa_nao_apaga_o_aviso_da_lombada() {
    let project = built(spec(30.0, 30.0), &[H; 15], 15);
    let mut editor = editor_at(project, 0);
    let mut form = editor.cover_form().unwrap();
    form.tab = CoverTab::Total;
    "62,1".clone_into(&mut form.total_width);
    "30,6".clone_into(&mut form.total_height);
    "1,5".clone_into(&mut form.spine);
    assert!(editor.create_cover(form.measures().unwrap(), None, &[]));
    assert!(editor.run(Command::NextSpread));
    assert!(editor.run(Command::NewSpread));
    let again = editor.cover_form().unwrap();
    assert!(editor.set_cover_measures(again.measures().unwrap(), None));
    let warnings = lamina_export::cover_preflight(album(&editor));
    assert!(
        warnings[0].to_string().contains("tinha 30 páginas"),
        "{warnings:?}"
    );
}

#[test]
fn varias_fotos_numa_face_nao_mexem_na_outra() {
    let project = three_spreads();
    let ids = free_photos(&project, 3);
    let mut editor = on_cover_with(project, &[]);
    assert!(editor.drop_on_cover(0.1, &ids[..1]));
    let back = album(&editor).cover.as_ref().unwrap().content.frames[0].rect;
    assert!(editor.drop_on_cover(0.9, &ids[1..]));
    let frames = &album(&editor).cover.as_ref().unwrap().content.frames;
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].rect, back);
    assert!(frames[1].rect.x() > 0.5 && frames[2].rect.x() > 0.5);
}

#[test]
fn esconder_o_guia_nao_entra_no_historico() {
    let project = three_spreads();
    let mut editor = editor_at(project, 0);
    let measures = CoverMeasures::defaults(&album(&editor).spec);
    let guide = lamina_core::CoverGuide {
        path: "guia.png".into(),
        includes_wrap: true,
        visible: true,
    };
    assert!(editor.create_cover(measures, Some(guide), &[]));
    assert!(editor.cover_overlay().unwrap().guide.is_some());
    let steps = editor.session().unwrap().history_list().steps.len();
    assert!(editor.run(Command::ToggleCoverGuide));
    assert!(editor.cover_overlay().unwrap().guide.is_none());
    assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
}
