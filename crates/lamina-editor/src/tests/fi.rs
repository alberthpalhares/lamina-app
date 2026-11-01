//! Cenários da spec `filmagem-da-interface` (FI) sem janela: o roteiro, a linha do tempo, a abertura
//! de cada modo, as pastas isoladas e as páginas dos vídeos.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::filming::{
    Action, Button, Cue, FilmError, Frame, MarkBox, Mode, Opening, Scene, Script, Startup, Step,
    Timeline, ease_in_out, empty_album, frame_time, is_known_mark, isolated, write_frame,
};
use crate::updates::news::video_address;

const ALL_STEPS: &str = r#"{
    "nome": "janela-nova",
    "abrir": "exemplo",
    "janela": [1600, 900],
    "escala": 2,
    "passos": [
        {"esperar_fotos": true},
        {"pausa_ms": 600},
        {"camera": "lamina", "zoom": 2.0},
        {"mover": "botao-layout", "ms": 700},
        {"clicar": true},
        {"clicar": "direito"},
        {"tecla": "ArrowDown"},
        {"arrastar": "foto-3", "ate": "quadro-2", "ms": 900},
        {"comando": "ajuda.atalhos"},
        {"legenda": "As setas trocam o layout", "ms": 1800},
        {"camera": "geral"}
    ]
}"#;

#[test]
fn fi_b1_o_roteiro_le_os_passos() {
    let script = Script::parse(ALL_STEPS).unwrap();
    assert_eq!(script.name, "janela-nova");
    assert_eq!(script.opening, Opening::Example);
    assert_eq!(script.window, (1600, 900));
    assert_eq!(script.scale, 2);
    let camera = |target: &str, zoom| Step::Camera {
        target: target.to_owned(),
        zoom,
        ms: 700,
    };
    assert_eq!(
        script.steps,
        vec![
            Step::WaitPhotos,
            Step::Pause { ms: 600 },
            camera("lamina", 2.0),
            Step::Move {
                mark: "botao-layout".to_owned(),
                ms: 700
            },
            Step::Click {
                button: Button::Left
            },
            Step::Click {
                button: Button::Right
            },
            Step::Key {
                key: "ArrowDown".to_owned()
            },
            Step::Drag {
                from: "foto-3".to_owned(),
                to: "quadro-2".to_owned(),
                ms: 900
            },
            Step::Command {
                name: "ajuda.atalhos".to_owned()
            },
            Step::Caption {
                text: "As setas trocam o layout".to_owned(),
                ms: 1800
            },
            camera("geral", 1.0),
        ]
    );
}

fn with_steps(steps: &str) -> String {
    format!(r#"{{"nome":"t","abrir":"vazio","janela":[800,600],"escala":1,"passos":[{steps}]}}"#)
}

#[test]
fn fi_b1_o_album_vazio_tambem_abre() {
    let script = Script::parse(&with_steps(r#"{"pausa_ms":10}"#)).unwrap();
    assert_eq!(script.opening, Opening::Empty);
}

fn script_error(text: &str) -> (Option<usize>, String) {
    match Script::parse(text) {
        Err(FilmError::Script { step, message }) => (step, message),
        other => panic!("esperava FilmError::Script, veio {other:?}"),
    }
}

#[test]
fn fi_b2_roteiro_errado_reprova_com_o_motivo() {
    assert_eq!(script_error("isto não é json").0, None);
    let (step, message) = script_error(&with_steps(r#"{"pausa_ms":1},{"voar":true}"#));
    assert_eq!(step, Some(2));
    assert!(message.contains("voar"), "{message}");
    let (step, message) = script_error(&with_steps(r#"{"mover":"lamina"}"#));
    assert_eq!(step, Some(1));
    assert!(message.contains("ms"), "{message}");
    let (step, message) = script_error(&with_steps(r#"{"tecla":"Teletransporte"}"#));
    assert_eq!(step, Some(1));
    assert!(message.contains("Teletransporte"), "{message}");
}

#[test]
fn fi_b2_o_erro_diz_o_passo_em_portugues() {
    let error = FilmError::Script {
        step: Some(2),
        message: "\"voar\" não existe".to_owned(),
    };
    assert_eq!(
        error.to_string(),
        "passo 2: roteiro inválido: \"voar\" não existe"
    );
}

/// A tela de mentira dos testes: as marcas num mapa.
#[derive(Default)]
struct FakeScene {
    marks: HashMap<String, MarkBox>,
    unavailable: Vec<String>,
}

impl FakeScene {
    fn with(mut self, name: &str, x: f32, y: f32) -> FakeScene {
        self.marks.insert(
            name.to_owned(),
            MarkBox {
                x: x - 5.0,
                y: y - 5.0,
                w: 10.0,
                h: 10.0,
            },
        );
        self
    }
}

impl Scene for FakeScene {
    fn mark(&mut self, name: &str) -> Option<MarkBox> {
        self.marks.get(name).copied()
    }
    fn available(&self, command: &str) -> bool {
        !self.unavailable.iter().any(|c| c == command)
    }
}

fn all_frames(script: &Script, scene: &mut dyn Scene) -> Vec<Frame> {
    let mut timeline = Timeline::new(script);
    let mut frames = Vec::new();
    while let Some(cue) = timeline.next(scene).unwrap() {
        if let Cue::Frame(frame) = cue {
            frames.push(frame);
        }
    }
    frames
}

#[test]
fn fi_b3_o_tempo_vem_do_numero_do_quadro() {
    assert_eq!(frame_time(0), Duration::ZERO);
    assert_eq!(frame_time(1), Duration::from_nanos(33_333_333));
    assert_eq!(frame_time(30), Duration::from_secs(1));
    assert_eq!(frame_time(90), Duration::from_secs(3));

    let text = with_steps(r#"{"pausa_ms":1000},{"mover":"alvo","ms":500},{"pausa_ms":100}"#);
    let script = Script::parse(&text).unwrap();
    // O cursor começa no centro da janela (400, 300) e vai 400 px para a direita.
    let scene = || FakeScene::default().with("alvo", 800.0, 300.0);
    let first = all_frames(&script, &mut scene());
    assert_eq!(
        first,
        all_frames(&script, &mut scene()),
        "a segunda vez mudou"
    );

    let x = |n: usize| first[n].cursor.0;
    assert_eq!(first[30].time, Duration::from_secs(1));
    for n in 0..=30 {
        assert!((x(n) - 400.0).abs() < f64::EPSILON, "quadro {n} andou");
    }
    assert!(
        (x(45) - 800.0).abs() < f64::EPSILON,
        "não chegou: {}",
        x(45)
    );
    for n in 30..45 {
        assert!(x(n + 1) >= x(n), "voltou no quadro {n}");
    }
    // 15 quadros não têm um do meio: o 37 e o 38 ficam, à mesma distância, um de cada lado.
    let middle = 600.0;
    assert!(x(37) < middle && x(38) > middle);
    assert!(((middle - x(37)) - (x(38) - middle)).abs() <= 1.0);
    let step = |n: usize| x(n + 1) - x(n);
    assert!(
        step(30) < step(37) && step(44) < step(37),
        "não acelera e freia"
    );
}

#[test]
fn fi_b3_a_curva_comeca_e_termina_parada() {
    assert!(ease_in_out(0.0).abs() < 1e-9);
    assert!((ease_in_out(0.5) - 0.5).abs() < 1e-9);
    assert!((ease_in_out(1.0) - 1.0).abs() < 1e-9);
    assert!(ease_in_out(0.1) < 0.1 && ease_in_out(0.9) > 0.9);
}

#[test]
fn fi_b3_clicar_aperta_e_solta_em_quadros_diferentes() {
    let text = with_steps(r#"{"clicar":"direito"},{"tecla":"Enter"},{"comando":"ajuda.sobre"}"#);
    let script = Script::parse(&text).unwrap();
    let frames = all_frames(&script, &mut FakeScene::default());
    let actions: Vec<Action> = frames.iter().flat_map(|f| f.actions.clone()).collect();
    assert_eq!(
        actions,
        vec![
            Action::Press(Button::Right),
            Action::Release(Button::Right),
            Action::Key("Enter".to_owned()),
            Action::Command("ajuda.sobre".to_owned()),
        ]
    );
    let at = |action: Action| frames.iter().position(|f| f.actions == [action.clone()]);
    assert!(at(Action::Press(Button::Right)) < at(Action::Release(Button::Right)));
}

#[test]
fn fi_b3_arrastar_vai_ate_a_origem_aperta_e_solta_no_destino() {
    let script = Script::parse(&with_steps(r#"{"arrastar":"a","ate":"b","ms":300}"#)).unwrap();
    let mut scene = FakeScene::default()
        .with("a", 100.0, 100.0)
        .with("b", 700.0, 500.0);
    let frames = all_frames(&script, &mut scene);
    let at = |action: &Action| {
        frames
            .iter()
            .find(|f| f.actions.contains(action))
            .map(|f| f.cursor)
    };
    assert_eq!(at(&Action::Press(Button::Left)), Some((100.0, 100.0)));
    assert_eq!(at(&Action::Release(Button::Left)), Some((700.0, 500.0)));
}

#[test]
fn fi_b3_esperar_as_fotos_nao_gasta_quadro() {
    let text = with_steps(r#"{"pausa_ms":100},{"esperar_fotos":true},{"pausa_ms":100}"#);
    let script = Script::parse(&text).unwrap();
    let mut timeline = Timeline::new(&script);
    let mut scene = FakeScene::default();
    let mut cues = Vec::new();
    while let Some(cue) = timeline.next(&mut scene).unwrap() {
        cues.push(cue);
    }
    assert_eq!(cues.len(), 7);
    assert_eq!(cues[3], Cue::WaitPhotos);
    match &cues[4] {
        Cue::Frame(frame) => assert_eq!(frame.number, 3),
        other => panic!("{other:?}"),
    }
}

#[test]
fn fi_b5_marca_desconhecida_reprova_antes_do_primeiro_quadro() {
    let text = with_steps(r#"{"pausa_ms":10},{"camera":"geral"},{"mover":"nao-existe","ms":100}"#);
    let script = Script::parse(&text).unwrap();
    assert_eq!(
        Timeline::new(&script).check_marks(&["lamina".to_owned()]),
        Err(FilmError::UnknownMark {
            step: 3,
            name: "nao-existe".to_owned()
        })
    );
    let text = with_steps(r#"{"arrastar":"foto-3","ate":"quadro-2","ms":100},{"camera":"lamina"}"#);
    let script = Script::parse(&text).unwrap();
    assert_eq!(
        Timeline::new(&script).check_marks(&["lamina".to_owned()]),
        Ok(())
    );
}

#[test]
fn fi_b5_as_marcas_repetidas_tem_numero() {
    let known = vec!["lamina".to_owned()];
    for good in ["lamina", "quadro-1", "foto-12", "menu-3"] {
        assert!(is_known_mark(good, &known), "{good}");
    }
    for bad in ["quadro-", "quadro-0", "foto-x", "laminas", ""] {
        assert!(!is_known_mark(bad, &known), "{bad}");
    }
}

fn next_error(script: &Script, scene: &mut dyn Scene) -> FilmError {
    let mut timeline = Timeline::new(script);
    loop {
        match timeline.next(scene) {
            Ok(Some(_)) => {}
            Ok(None) => panic!("o roteiro terminou sem erro"),
            Err(error) => return error,
        }
    }
}

#[test]
fn fi_b5_marca_que_sumiu_reprova_com_o_passo() {
    let text = with_steps(r#"{"pausa_ms":100},{"mover":"sumiu","ms":100}"#);
    let script = Script::parse(&text).unwrap();
    let missing = FilmError::MissingMark {
        step: 2,
        name: "sumiu".to_owned(),
    };
    assert_eq!(next_error(&script, &mut FakeScene::default()), missing);
    // Tamanho zero é a mesma coisa que não estar na tela.
    let mut scene = FakeScene::default();
    scene.marks.insert(
        "sumiu".to_owned(),
        MarkBox {
            x: 10.0,
            y: 10.0,
            w: 0.0,
            h: 0.0,
        },
    );
    assert_eq!(next_error(&script, &mut scene), missing);
}

#[test]
fn fi_b5_comando_indisponivel_reprova_com_o_passo() {
    let script = Script::parse(&with_steps(r#"{"comando":"arquivo.exportar"}"#)).unwrap();
    let mut scene = FakeScene {
        unavailable: vec!["arquivo.exportar".to_owned()],
        ..FakeScene::default()
    };
    assert_eq!(
        next_error(&script, &mut scene),
        FilmError::Unavailable {
            step: 1,
            command: "arquivo.exportar".to_owned()
        }
    );
}

#[test]
fn fi_b10_o_marcas_json_traz_cursor_camera_e_legendas() {
    let text = with_steps(
        r#"{"pausa_ms":100},{"camera":"alvo","zoom":2.0,"ms":300},{"legenda":"Olá","ms":1000},{"mover":"alvo","ms":100},{"clicar":true},{"tecla":"Enter"}"#,
    );
    let script = Script::parse(&text).unwrap();
    let mut scene = FakeScene::default().with("alvo", 200.0, 100.0);
    let mut timeline = Timeline::new(&script);
    while timeline.next(&mut scene).unwrap().is_some() {}
    let json: serde_json::Value = serde_json::from_str(&timeline.marks_json()).unwrap();
    assert_eq!(json["nome"], "t");
    assert_eq!(json["fps"], 30);
    assert_eq!(json["janela"], serde_json::json!([800, 600]));
    assert_eq!(json["escala"], 1);
    let frames = json["quadros"].as_array().unwrap();
    assert_eq!(frames[0]["x"], 400.0);
    assert!(frames.iter().any(|f| f["botao"] == 1));
    assert!(
        frames
            .iter()
            .any(|f| f["teclas"] == serde_json::json!(["Enter"]))
    );
    let camera = &json["cameras"][0];
    assert_eq!(camera["quadro"], 3);
    assert_eq!(camera["x"], 200.0);
    assert_eq!(camera["zoom"], 2.0);
    assert_eq!(camera["ms"], 300);
    let caption = &json["legendas"][0];
    assert_eq!(caption["de"], 3);
    assert_eq!(caption["ate"], 33);
    assert_eq!(caption["texto"], "Olá");
    let steps = json["passos"].as_array().unwrap();
    let moved = steps.iter().find(|p| p["passo"] == 4).unwrap();
    assert_eq!(moved["marcas"]["alvo"]["w"], 10.0);
}

#[test]
fn fi_b9_filmar_nao_vai_a_internet_nem_abre_nada_sozinho() {
    let all = Startup {
        news: true,
        backup_ask: true,
        backup_timer: true,
        recents_check: true,
        update_check: true,
    };
    assert_eq!(Startup::for_mode(Mode::Normal), all);
    assert_eq!(
        Startup::for_mode(Mode::Film),
        Startup {
            news: false,
            backup_ask: false,
            backup_timer: false,
            recents_check: false,
            update_check: false,
        }
    );
    assert_eq!(
        Startup::for_mode(Mode::OpenFile),
        Startup {
            update_check: false,
            ..all
        }
    );
    let smoke = Startup {
        news: false,
        backup_ask: false,
        update_check: false,
        ..all
    };
    assert_eq!(Startup::for_mode(Mode::Verify), smoke);
    assert_eq!(Startup::for_mode(Mode::Import), smoke);
}

fn vars(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

#[test]
fn fi_b12_pasta_do_usuario_nao_isolada_e_recusada() {
    assert_eq!(
        isolated(&vars(&[("APPDATA", r"C:\Users\a\AppData")]), r"C:\saida"),
        Err(vec!["APPDATA".to_owned()])
    );
    assert_eq!(
        isolated(
            &vars(&[
                ("APPDATA", r"C:\saida\perfil\AppData"),
                ("LOCALAPPDATA", r"c:/saida/perfil/Local"),
                ("USERPROFILE", r"C:\saida\perfil\Home"),
            ]),
            r"C:\saida"
        ),
        Ok(())
    );
    assert_eq!(
        isolated(
            &vars(&[
                ("XDG_CONFIG_HOME", "/tmp/saida/perfil/config"),
                ("HOME", "/tmp/saida/perfil"),
            ]),
            "/tmp/saida"
        ),
        Ok(())
    );
}

#[test]
fn fi_b12_pasta_vizinha_com_o_mesmo_comeco_nao_conta() {
    assert_eq!(
        isolated(&vars(&[("HOME", "/tmp/saida-outra")]), "/tmp/saida"),
        Err(vec!["HOME".to_owned()])
    );
}

#[test]
fn fi_b12_sem_nenhuma_variavel_nao_passa() {
    assert_eq!(
        isolated(&vars(&[("PATH", "/bin")]), "/tmp/saida"),
        Err(vec!["APPDATA".to_owned(), "HOME".to_owned()])
    );
}

#[test]
fn fi_b10_o_album_vazio_nao_tem_fotos_nem_laminas() {
    let project = empty_album().unwrap();
    assert_eq!((project.photos.len(), project.spreads.len()), (0, 0));
    assert_eq!(project.name, "Álbum vazio");
}

#[test]
fn fi_b12_o_quadro_sai_em_png_com_o_tamanho_certo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quadro-00001.png");
    write_frame(&path, 2, 1, &[255, 0, 0, 0, 0, 255]).unwrap();
    let image = image::open(&path).unwrap().to_rgb8();
    assert_eq!(image.dimensions(), (2, 1));
    assert_eq!(image.get_pixel(1, 0).0, [0, 0, 255]);
}

#[test]
fn fi_b12_pasta_que_nao_existe_da_erro_de_gravacao() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nao-existe").join("q.png");
    assert!(matches!(
        write_frame(&path, 1, 1, &[0, 0, 0]),
        Err(FilmError::Write { .. })
    ));
}

/// Cada endereço `http` ou `https` do texto.
fn addresses(text: &str) -> Vec<&str> {
    text.match_indices("http")
        .map(|(at, _)| {
            let rest = &text[at..];
            let end = rest.find(['"', '\'', ' ', '<', '>']).unwrap_or(rest.len());
            &rest[..end]
        })
        .collect()
}

#[test]
fn fi_b13_a_pagina_do_video_nao_esta_quebrada() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site/v");
    let pages: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(!pages.is_empty(), "nenhuma página em site/v");
    for page in pages {
        let name = page.file_name().unwrap().to_str().unwrap();
        let address = format!("https://alberthpalhares.github.io/lamina-app/v/{name}");
        assert!(
            video_address(&address).is_some(),
            "nome fora do formato: {name}"
        );
        let text = std::fs::read_to_string(&page).unwrap();
        assert!(text.contains("<title>"), "{name} sem título");
        assert!(
            text.contains(r#"href="../""#),
            "{name} sem o link de volta ao site"
        );
        for address in addresses(&text) {
            let youtube = address.starts_with("https://www.youtube.com/")
                || address.starts_with("https://youtu.be/");
            let site = address.starts_with("https://alberthpalhares.github.io/lamina-app/");
            assert!(youtube || site, "{name}: destino estranho: {address}");
        }
    }
}

#[test]
fn fi_b13_o_endereco_de_fora_reprova_na_conferencia() {
    let text = r#"<a href="http://x.org/a">x</a> <meta content="0;url=https://youtu.be/abc">"#;
    assert_eq!(
        addresses(text),
        vec!["http://x.org/a", "https://youtu.be/abc"]
    );
}
