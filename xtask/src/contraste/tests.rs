use super::*;

const CYAN_DARK: [u8; 3] = [0x19, 0xac, 0xc9];

/// Uma paleta escura que passa em tudo; cada teste estraga um token.
fn good() -> Palette {
    let text = "    out property <Colors> dark: {\n\
        stage: #1c1c1c, strip: #262626, panel: #303030, bar: #222222, raised: #383838, field: #1e1e1e,\n\
        line-strong: #858585, text: #e6e6e6, text-2: #a6a6a6, text-3: #858585,\n\
        accent: #19acc9, accent-hover: #3dbbd4, on-accent: #111111, focus: #19acc9, link: #2ebcd8,\n\
        warning: #e3b341, error: #ef7d75, success: #5cc283,\n\
        tag-new-bg: #6b6b6b, tag-bg: #4a4a4a, tag-text: #ffffff,\n\
    };\n";
    palettes(text).remove("dark").unwrap()
}

#[test]
fn le_os_tokens_e_ignora_o_alfa() {
    let p = palettes("out property <Colors> light: {\n a: #ffffff80, b: #000000,\n};\n");
    assert_eq!(p["light"]["a"], [255, 255, 255]);
    assert_eq!(p["light"]["b"], [0, 0, 0]);
}

#[test]
fn contraste_de_preto_e_branco_e_21() {
    assert!((ratio([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.01);
}

#[test]
fn df_b3_contraste_do_texto_nos_dois_temas() {
    assert!(problems("dark", &good(), CYAN_DARK).is_empty());
    let mut p = good();
    p.insert("text-2".into(), [0x55, 0x55, 0x55]);
    let found = problems("dark", &p, CYAN_DARK);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("dark: text-2 sobre raised"))
    );
}

#[test]
fn df_b4_icones_fios_foco_e_texto_no_destaque() {
    let mut p = good();
    p.insert("line-strong".into(), [0x44, 0x44, 0x44]);
    p.insert("on-accent".into(), [0xff, 0xff, 0xff]);
    let found = problems("dark", &p, CYAN_DARK);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("dark: line-strong sobre stage"))
    );
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("dark: on-accent sobre accent"))
    );
}

#[test]
fn df_b5_cinzas_sem_matiz() {
    let mut p = good();
    p.insert("panel".into(), [0x30, 0x30, 0x36]);
    let found = problems("dark", &p, CYAN_DARK);
    assert!(found.contains(&"dark: panel tem matiz (R, G e B diferentes)".to_owned()));
}

#[test]
fn df_b6_o_destaque_e_o_ciano_da_marca() {
    let guide = "| Ciano de processo | `#0095B6` | `#19ACC9` | página direita |";
    assert_eq!(brand_cyan(guide), Some(([0x00, 0x95, 0xb6], CYAN_DARK)));
    let mut p = good();
    p.insert("accent".into(), [0x2f, 0x7f, 0xc1]);
    let found = problems("dark", &p, CYAN_DARK);
    assert!(found.contains(&"dark: o destaque não é o ciano da marca".to_owned()));
}

#[test]
fn df_b7_as_duas_paletas_tem_os_mesmos_tokens() {
    let dark = good();
    let mut light = good();
    assert!(missing(&dark, &light).is_empty());
    light.remove("field");
    assert_eq!(missing(&dark, &light), ["field"]);
}

#[test]
fn df_b8_superficies_distinguiveis() {
    assert!(surface_problems("dark", &good()).is_empty());
    let mut p = good();
    p.insert("strip".into(), [0x1e, 0x1e, 0x1e]);
    assert_eq!(
        surface_problems("dark", &p),
        ["dark: stage e strip não se distinguem (mínimo 1,1:1)"]
    );
}

#[test]
fn df_b4_link_e_botao_principal_sob_o_mouse_tambem_contam() {
    let mut p = good();
    p.insert("link".into(), [0x19, 0x6a, 0x7a]);
    p.insert("accent-hover".into(), [0x00, 0x60, 0x70]);
    let found = problems("dark", &p, CYAN_DARK);
    assert!(found.iter().any(|f| f.starts_with("dark: link sobre")));
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("dark: on-accent sobre accent-hover"))
    );
}

#[test]
fn df_b8_superficies_valem_nos_dois_temas() {
    let mut p = good();
    p.insert("strip".into(), [0x1e, 0x1e, 0x1e]);
    assert_eq!(
        surface_problems("light", &p),
        ["light: stage e strip não se distinguem (mínimo 1,1:1)"]
    );
}

/// As cores da barra de atualização (spec AV).
fn attention_colors() -> Palette {
    attention::fixed(
        "export global Fixed {\n\
         out property <color> attention: #ffd400;\n\
         out property <color> on-attention: #000000;\n\
         out property <color> attention-hover: #e6bf00;\n\
         out property <color> on-attention-hover: #333333;\n\
         }\n",
    )
}

fn file(name: &str, text: &str) -> (String, String) {
    (name.to_owned(), text.to_owned())
}

#[test]
fn av_b9_a_barra_e_amarela_com_letras_pretas_e_passa_no_contraste() {
    let good = attention_colors();
    assert_eq!(good["attention"], [0xff, 0xd4, 0x00]);
    assert_eq!(attention::color_problems(&good), Vec::<String>::new());
    let with = |token: &str, color: [u8; 3]| {
        let mut p = good.clone();
        p.insert(token.into(), color);
        attention::color_problems(&p)
    };
    let found = with("attention", CYAN_DARK);
    assert!(
        found.iter().any(|f| f.contains("não é amarelo")),
        "{found:?}"
    );
    let found = with("on-attention", [0x22, 0x22, 0x22]);
    assert!(found.iter().any(|f| f.contains("não é preto")), "{found:?}");
    let found = with("attention-hover", [0x40, 0x40, 0x40]);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("on-attention sobre attention-hover")),
        "{found:?}"
    );
    let found = with("on-attention-hover", [0xc0, 0xa0, 0x00]);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("attention sobre on-attention-hover")),
        "{found:?}"
    );
    assert!(
        !attention::color_problems(&Palette::new()).is_empty(),
        "token faltando reprova"
    );
}

#[test]
fn av_b9_a_barra_nao_usa_o_tema_e_o_token_fica_so_nela() {
    let bar = "// Sobre o Theme, em comentário.\nbackground: Fixed.attention;\n";
    let palette = "out property <color> attention: #ffd400;\n";
    let gallery = "UpdateBar { }\nFixed.attention\n";
    let good = [
        file("update_bar.slint", bar),
        file("palette.slint", palette),
        file("gallery.slint", gallery),
        file("app.slint", "UpdateBar { }\n"),
    ];
    assert_eq!(attention::use_problems(&good), Vec::<String>::new());
    let themed = [file("update_bar.slint", "background: Theme.bar;\n")];
    let found = attention::use_problems(&themed);
    assert!(found.iter().any(|f| f.contains("usa Theme")), "{found:?}");
    let plain = [file("update_bar.slint", "background: Fixed.paper;\n")];
    let found = attention::use_problems(&plain);
    assert!(found.iter().any(|f| f.contains("não usa")), "{found:?}");
    let elsewhere = [
        file("update_bar.slint", bar),
        file("stage.slint", "color: Fixed.on-attention;\n"),
    ];
    assert_eq!(
        attention::use_problems(&elsewhere),
        ["attention fora da barra de atualização: stage.slint"]
    );
}

#[test]
fn pv_b15_as_etiquetas_passam_no_contraste_nos_dois_temas() {
    let text = include_str!("../../../crates/lamina-app/ui/theme/palette.slint");
    let all = palettes(text);
    for theme in ["dark", "light"] {
        let p = &all[theme];
        for bg in ["tag-new-bg", "tag-bg"] {
            assert!(p.contains_key(bg), "{theme}: falta {bg}");
            assert!(
                ratio(p["tag-text"], p[bg]) >= 4.5,
                "{theme}: tag-text sobre {bg}"
            );
        }
    }
    assert!(alpha_problems(text).is_empty());

    let mut bad = good();
    bad.insert("tag-text".into(), [0x80, 0x80, 0x80]);
    let found = problems("dark", &bad, CYAN_DARK);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("dark: tag-text sobre tag-new-bg"))
    );

    let translucent =
        "out property <Colors> dark: {\n tag-new-bg: #6b6b6b80, tag-bg: #4a4a4a,\n};\n";
    assert_eq!(
        alpha_problems(translucent),
        ["dark: tag-new-bg tem alfa: o fundo da etiqueta precisa ser opaco"]
    );
}

#[test]
fn av_b10_a_barra_nao_fica_escondida_atras_da_pagina_de_novidades() {
    let app = include_str!("../../../crates/lamina-app/ui/app.slint");
    assert!(app.contains("update-bar-shown: root.update-bar != \"\" && !NewsUi.show"));
    assert!(app.contains("if root.update-bar-shown: UpdateBar"));
    assert!(app.contains("(root.update-bar-shown ? 32px : 0px)"));
}
