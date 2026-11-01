//! Cenários da spec `zoom-e-tamanho-real` (ZR) que rodam sem janela: as dicas da paleta, o zoom
//! em passos de 1,5, o ponto embaixo do mouse, o 1:1 calibrado e estimado, a calibração por
//! monitor, Ajustar, Voltar e o que nunca vai para o arquivo.

use lamina_core::Mm;

use super::tl_common::{H, V, built, editor_at, spec, three_spreads};
use crate::calibration::{ESTIMATE_TIP, Monitor};
use crate::editor::{PaletteTool, Zoom, zoom_about};
use crate::group::mm_per_px;
use crate::keys::{Key, Shortcut};
use crate::palette_tips::{PALETTE_BUTTONS, PaletteButton, TIP_DELAY_MS};
use crate::{Area, Command, Editor, FrameKind, Install, StripsStore, spread_view};

/// Um álbum de 30 × 30 cm com 3 mm de sangria (a lâmina tem 606 mm), no modo organizar, com
/// Ajustar de 1.000 px.
fn organizing() -> Editor {
    let spec = spec(30.0, 30.0).with_bleed(Mm(3.0)).unwrap();
    let mut editor = editor_at(built(spec, &[H, H, V, H, H, H, V, H], 2), 0);
    assert!(editor.run(Command::Organize));
    editor.set_fit_width(1000.0);
    editor
}

/// Os rótulos do menu de contexto, com os dos submenus.
fn menu_labels(menu: &[crate::editor::ContextEntry]) -> Vec<String> {
    use crate::editor::ContextEntry as E;
    menu.iter()
        .flat_map(|e| match e {
            E::Item { label, .. } => vec![label.clone()],
            E::Sub { items, .. } => menu_labels(items),
            _ => Vec::new(),
        })
        .collect()
}

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn zr_b1_dicas_da_paleta() {
    let mut editor = organizing();
    let tip = |e: &Editor, b: PaletteButton| e.palette_tip(b);
    let tools = PaletteTool::ALL.map(|t| tip(&editor, PaletteButton::Tool(t)));
    assert_eq!(
        tools,
        [
            "Selecionar e mover quadros",
            "Quadro de foto: arraste para desenhar",
            "Texto: arraste para desenhar",
            "Mão: arraste para mover a vista (ou segure Espaço)",
            "Lupa (Z): clique aproxima, Alt+clique afasta",
        ]
    );
    let run = |c| tip(&editor, PaletteButton::Run(c));
    assert_eq!(run(Command::ViewZoomOut), "Afastar (−)");
    assert_eq!(run(Command::ViewZoomIn), "Aproximar (+)");
    assert_eq!(run(Command::ActualSize), "Tamanho real (1)");
    assert_eq!(run(Command::ViewFit), "Ajustar a lâmina à área (0)");
    // As teclas vêm do registro.
    assert_eq!(
        Command::Loupe.shortcut(),
        Some(Shortcut::key(Key::Char('z')))
    );
    assert_eq!(
        Command::ActualSize.shortcut(),
        Some(Shortcut::key(Key::Char('1')))
    );
    // Distribuir com dois quadros selecionados não vale.
    editor.organize_click(0, false, false);
    editor.organize_click(1, true, false);
    assert!(
        tip(&editor, PaletteButton::Run(Command::DistributeAcross)).ends_with(" (indisponível)")
    );
    assert_eq!(TIP_DELAY_MS, 500);
    assert_eq!(editor.palette_tips().len(), PALETTE_BUTTONS.len());
}

#[test]
fn zr_b2_quadro_vazio_no_modo() {
    let mut editor = organizing();
    assert!(editor.run(Command::NewPhotoFrame), "um quadro vazio");
    let empty = editor.selected_frame().unwrap();
    for _ in 0..3 {
        editor.zoom_step(true, None, true);
    }
    assert!(close(editor.view_zoom(), 3.375, 1e-9));
    let project = editor.session().unwrap().project().clone();
    let area = Area {
        width: 1000.0 * editor.view_zoom(),
        height: 700.0 * editor.view_zoom(),
        margin: 0.0,
    };
    let view = spread_view(&project, 0, area, &[]).unwrap();
    assert!(
        matches!(view.frames[empty].kind, FrameKind::Empty),
        "o X em qualquer zoom"
    );
    editor.organize_click(empty, false, false);
    let menu = editor.context_menu(Some(empty));
    assert!(menu_labels(&menu).iter().any(|l| l == "Apagar quadro"));
    assert!(editor.run(Command::ClearFrame));
    let s = editor.session().unwrap();
    assert_eq!(
        s.project().spreads[0].frames.len(),
        project.spreads[0].frames.len() - 1
    );
    assert!(
        s.history_list()
            .steps
            .last()
            .unwrap()
            .label
            .starts_with("Apagar quadro")
    );
}

#[test]
fn zr_b3_1_para_1_calibrado_e_estimado() {
    let mut editor = organizing();
    editor.set_monitor(Monitor {
        key: "A 2560×1440".to_owned(),
        scale: 1.25,
    });
    assert!(editor.run(Command::ActualSize));
    assert!(
        close(editor.view_zoom(), 2.290, 0.0005),
        "{}",
        editor.view_zoom()
    );
    assert_eq!(editor.zoom_label(), "≈ 1:1");
    let tips = editor.palette_tips();
    let real = PALETTE_BUTTONS
        .iter()
        .position(|b| *b == PaletteButton::Run(Command::ActualSize))
        .unwrap();
    assert!(tips[real].contains(ESTIMATE_TIP));
    assert!(editor.calibrate(340.0));
    assert!(
        close(editor.view_zoom(), 2.407, 0.0005),
        "{}",
        editor.view_zoom()
    );
    assert_eq!(editor.zoom_label(), "1:1");
    let project = editor.session().unwrap().project().clone();
    let drawn = 1000.0 * editor.view_zoom();
    assert!(close(drawn, 2407.0, 0.5));
    let mm = 100.0 * mm_per_px(&project, drawn).unwrap();
    assert!(
        close(mm, 25.2, 0.05),
        "um quadro arrastado 100 px anda {mm} mm"
    );
}

#[test]
fn zr_b4_calibracao_por_monitor_com_limites() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(
        &file,
        r#"{"versao": 2, "tamanho_real": [
            {"monitor": "C 800×600", "px_por_mm": 0.5},
            {"monitor": "D 800×600", "px_por_mm": 25}
        ]}"#,
    )
    .unwrap();
    let mut editor = organizing();
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    let a = Monitor {
        key: "A 2560×1440".to_owned(),
        scale: 1.25,
    };
    let b = Monitor {
        key: "B 1920×1080".to_owned(),
        scale: 1.0,
    };
    editor.set_monitor(b.clone());
    assert!(!editor.calibrated(), "0,5 e 25 são ignorados");
    editor.set_monitor(a.clone());
    assert!(editor.calibrate(340.0));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let list = json["tamanho_real"].as_array().unwrap();
    assert_eq!(
        list.len(),
        1,
        "os de fora da faixa saem na gravação: {list:?}"
    );
    assert_eq!(list[0]["monitor"], "A 2560×1440");
    assert!(close(list[0]["px_por_mm"].as_f64().unwrap(), 4.965, 0.001));
    editor.set_monitor(b.clone());
    assert!(!editor.calibrated(), "B não muda");
    assert!(!editor.calibrate(10.0), "1 a 20: 10 px dá 0,12 px/mm");
    editor.set_monitor(a);
    assert!(editor.forget_calibration());
    assert!(!editor.calibrated());
    editor.set_monitor(b);
    assert!(!editor.calibrated());
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(
        json.get("tamanho_real").is_none(),
        "sem calibração, sem o campo"
    );
}

#[test]
fn zr_b5_ajustar_encaixa_e_centraliza() {
    for how in ["botao", "tecla", "rotulo"] {
        let mut editor = organizing();
        editor.zoom_step(true, None, true);
        editor.zoom_step(true, None, true);
        editor.zoom_step(true, None, true);
        editor.set_view_pan(-300.0, 120.0);
        match how {
            "tecla" => {
                let c = Command::from_shortcut_in(
                    Shortcut::key(Key::Char('0')),
                    crate::commands::Mode::Organize,
                )
                .unwrap();
                assert_eq!(c, Command::ViewFit);
                assert!(editor.run(c));
            }
            _ => assert!(editor.run(Command::ViewFit)),
        }
        assert_eq!(editor.zoom(), Zoom::Fit, "{how}");
        assert_eq!(editor.view_pan(), (0.0, 0.0));
        assert_eq!(editor.zoom_label(), "Ajustar");
    }
}

#[test]
fn zr_b6_lupa_e_passos() {
    let mut editor = organizing();
    let z = Command::from_shortcut_in(
        Shortcut::key(Key::Char('z')),
        crate::commands::Mode::Organize,
    )
    .unwrap();
    assert_eq!(z, Command::Loupe);
    assert!(editor.run(z));
    assert_eq!(editor.tool(), PaletteTool::Magnifier);
    let mut seen = Vec::new();
    for _ in 0..7 {
        editor.zoom_step(true, Some((10.0, 10.0)), true);
        seen.push(editor.zoom_label());
    }
    assert_eq!(seen, ["1,5×", "2,3×", "3,4×", "5,1×", "7,6×", "8×", "8×"]);
    editor.zoom_step(false, Some((10.0, 10.0)), true);
    assert_eq!(editor.zoom_label(), "5,3×", "Alt+clique afasta");
    assert!(editor.run(z));
    assert_eq!(editor.tool(), PaletteTool::Select);
    assert!(
        editor.selected_frames().is_empty(),
        "nenhum clique da lupa seleciona"
    );
    // + e − andam os mesmos passos.
    assert!(editor.run(Command::ViewFit));
    assert!(editor.run(Command::ViewZoomIn));
    assert_eq!(editor.zoom_label(), "1,5×");
    assert!(editor.run(Command::ViewZoomOut));
    assert_eq!(editor.zoom_label(), "Ajustar");
    // Um 1:1 menor que Ajustar não afasta; um maior que 8× não aproxima.
    editor.set_monitor(Monitor {
        key: "pequeno".to_owned(),
        scale: 1.0,
    });
    editor.set_fit_width(606.0 * 3.7795 / 0.8);
    assert!(editor.run(Command::ActualSize));
    assert!(close(editor.view_zoom(), 0.8, 0.001));
    assert!(editor.zoom_step(false, None, true).is_none());
    editor.set_fit_width(606.0 * 3.7795 / 11.0);
    assert!(close(editor.view_zoom(), 11.0, 0.01));
    assert!(editor.zoom_step(true, None, true).is_none());
}

#[test]
fn zr_b7_o_ponto_embaixo_do_mouse() {
    let mut editor = organizing();
    // O ponteiro a (+300, −100) do centro: 80% da largura da lâmina (1.000 px em Ajustar).
    let p = (300.0, -100.0);
    let step = editor.zoom_step(true, Some(p), true).unwrap();
    assert_eq!(step.pan, (-150.0, 50.0));
    assert_eq!(step.ms, 200, "a lupa dura 200 ms");
    // O ponto da lâmina que estava sob o ponteiro continua lá.
    let q = (p.0 - 0.0) / 1.0;
    let after = step.pan.0 + q * step.factor;
    assert!(close(after, p.0, 0.5));
    assert_eq!(zoom_about(p, (0.0, 0.0), 1.0, 1.5), (-150.0, 50.0));
    // O Ctrl+roda volta a Ajustar, com a vista no centro e sem animar.
    let back = editor.zoom_step(false, Some(p), false).unwrap();
    assert_eq!(editor.zoom(), Zoom::Fit);
    assert_eq!(back.pan, (0.0, 0.0));
    assert_eq!(back.ms, 0);
}

#[test]
fn zr_b8_voltar_anima_e_sai() {
    for reduce in [false, true] {
        let mut editor = organizing();
        if reduce {
            assert!(editor.run(Command::ToggleReduceMotion));
        }
        editor.zoom_step(true, None, true);
        editor.zoom_step(true, None, true);
        editor.set_view_pan(40.0, -20.0);
        editor.organize_click(0, false, false);
        let steps = editor.session().unwrap().history_list().steps.len();
        assert!(editor.run(Command::Organize), "Voltar");
        assert!(!editor.organizing(), "o editor sai na hora");
        assert!(editor.selected_frames().is_empty());
        assert_eq!(
            Command::from_shortcut(Shortcut::key(Key::Down)),
            Some(Command::NextLayout),
            "↓ troca o layout"
        );
        assert_eq!(editor.session().unwrap().history_list().steps.len(), steps);
        assert_eq!(editor.zoom(), Zoom::Fit);
        assert_eq!(editor.view_pan(), (0.0, 0.0));
        assert_eq!(editor.view_ms(), if reduce { 0 } else { 200 });
    }
}

#[test]
fn zr_b9_esc_ainda_conclui() {
    let mut editor = organizing();
    editor.organize_click(0, false, false);
    let esc =
        Command::from_shortcut_in(Shortcut::key(Key::Escape), crate::commands::Mode::Organize)
            .unwrap();
    assert!(editor.run(esc));
    assert!(
        editor.organizing() && editor.selected_frames().is_empty(),
        "desmarca"
    );
    assert!(editor.run(esc));
    assert!(!editor.organizing(), "sai como o Voltar");
    let o = Command::from_shortcut(Shortcut::key(Key::Char('o'))).unwrap();
    assert!(editor.run(o));
    assert!(editor.organizing());
    assert_eq!(editor.organize_title(), "Organizar a lâmina 1");
    let menu = editor.context_menu(None);
    assert!(menu_labels(&menu).iter().any(|l| l == "Voltar"));
    assert!(editor.run(o));
    assert!(!editor.organizing());
}

#[test]
fn zr_b10_trocar_de_monitor_usa_a_calibracao_dele() {
    let mut editor = organizing();
    let a = Monitor {
        key: "A 2560×1440".to_owned(),
        scale: 1.25,
    };
    let b = Monitor {
        key: "B 1920×1080".to_owned(),
        scale: 1.0,
    };
    editor.set_monitor(a.clone());
    assert!(editor.calibrate(340.0));
    assert!(editor.run(Command::ActualSize));
    assert!(editor.set_monitor(b));
    assert!(close(editor.view_zoom(), 2.290, 0.0005));
    assert_eq!(editor.zoom_label(), "≈ 1:1");
    assert!(editor.set_monitor(a.clone()));
    assert!(!editor.set_monitor(a), "o mesmo monitor não conta");
    assert!(close(editor.view_zoom(), 2.407, 0.0005));
    assert_eq!(editor.zoom_label(), "1:1");
    assert_eq!(editor.zoom(), Zoom::Real, "o zoom continua no 1:1");
}

#[test]
fn zr_b11_o_zoom_nunca_vai_para_o_arquivo() {
    let dir = tempfile::tempdir().unwrap();
    let album = dir.path().join("album.lamina");
    let strips = dir.path().join("janela.json");
    let mut editor = editor_at(three_spreads(), 0);
    editor.set_strips(StripsStore::load_for(&strips, Install::Existing));
    editor.save_to(Some(&album)).unwrap();
    let before = std::fs::read(&album).unwrap();
    assert!(editor.run(Command::Organize));
    editor.set_fit_width(1000.0);
    assert!(editor.run(Command::Loupe));
    editor.zoom_step(true, Some((50.0, 20.0)), true);
    editor.set_view_pan(30.0, 10.0);
    assert!(editor.run(Command::ActualSize));
    let (zoom, pan) = (editor.zoom(), editor.view_pan());
    assert!(editor.run(Command::NextSpread));
    assert_eq!(
        (editor.zoom(), editor.view_pan()),
        (zoom, pan),
        "trocar de lâmina mantém"
    );
    editor.save_to(Some(&album)).unwrap();
    assert_eq!(
        std::fs::read(&album).unwrap(),
        before,
        "o arquivo salvo é o mesmo"
    );
    assert!(editor.run(Command::Organize));
    assert!(editor.run(Command::Organize));
    assert_eq!(editor.zoom(), Zoom::Fit, "entrar de novo começa em Ajustar");
    assert_eq!(editor.tool(), PaletteTool::Select);
    let text = std::fs::read_to_string(&strips).unwrap_or_default();
    for word in ["zoom", "vista", "ferramenta", "lupa"] {
        assert!(!text.contains(word), "{word} no janela.json: {text}");
    }
}

#[test]
fn zr_b12_esconder_a_faixa_e_o_painel_no_modo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("janela.json");
    std::fs::write(&file, r#"{"versao": 2, "laminas": true, "painel": true}"#).unwrap();
    let mut editor = organizing();
    editor.set_strips(StripsStore::load_for(&file, Install::Existing));
    let mode = crate::commands::Mode::Organize;
    let alt1 = Command::from_shortcut_in(Shortcut::alt('1'), mode).unwrap();
    let alt4 = Command::from_shortcut_in(Shortcut::alt('4'), mode).unwrap();
    assert_eq!((alt1, alt4), (Command::ToggleSpreads, Command::TogglePanel));
    assert!(editor.run(alt1));
    assert!(editor.run(alt4));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(
        (json["laminas"].as_bool(), json["painel"].as_bool()),
        (Some(false), Some(false))
    );
    // Os botões da barra rodam os mesmos comandos, pelo nome.
    for name in ["ver.faixa-laminas", "ver.painel"] {
        assert!(editor.run(Command::from_name(name).unwrap()), "{name}");
    }
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(
        (json["laminas"].as_bool(), json["painel"].as_bool()),
        (Some(true), Some(true))
    );
}
