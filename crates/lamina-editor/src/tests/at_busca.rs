//! Cenários da spec `atalhos` (AT): a busca (AT-B6), as teclas do Mac (AT-B8) e as linhas fixas
//! (AT-B11).

use super::at::{all_rows, keys};
use crate::shortcuts::{
    CROP, DRAG_AND_CLICK, NO_MATCH_HINT, ORGANIZE, fold, no_match, search, sheet,
};
use crate::{Command, Platform};

#[test]
fn at_b6_busca_sem_acento_e_sem_maiuscula() {
    let groups = sheet(Platform::Windows);
    let lamina = search(&groups, "LAMINA");
    assert!(!lamina.is_empty());
    for g in &lamina {
        for r in &g.rows {
            let text = format!("{} {} {}", g.title, r.does, r.keys.join(" "));
            assert!(fold(&text).contains("lamina"), "{text}");
        }
    }
    assert!(all_rows(&lamina).contains(&"Próxima lâmina"));
    assert_eq!(
        all_rows(&search(&groups, "proxima lamina")),
        ["Próxima lâmina"]
    );
    assert_eq!(all_rows(&search(&groups, "ctrl+s")), ["Salvar"]);
    assert_eq!(all_rows(&search(&groups, " Ctrl + S ")), ["Salvar"]);
    assert_eq!(all_rows(&search(&groups, "f1")), ["Atalhos do teclado…"]);
    let arrows = search(&groups, "seta");
    let titles: Vec<&str> = arrows.iter().map(|g| g.title).collect();
    assert_eq!(titles, ["Lâmina", "Layout", ORGANIZE]);
    let rows = all_rows(&arrows);
    for does in [
        "Mover lâmina para a esquerda",
        "Próximo layout",
        "Próxima lâmina",
        "Mover 10 mm para baixo",
        "Mover o quadro selecionado 1 mm",
        "Mover o quadro selecionado 10 mm",
    ] {
        assert!(rows.contains(&does), "{does}");
    }
    assert!(!rows.contains(&"Salvar"));
    assert!(all_rows(&search(&groups, "trocar")).contains(&"Trocar duas fotos de lugar"));
    assert!(search(&groups, "xyzzy").is_empty());
    assert_eq!(no_match("xyzzy"), "Nenhum atalho com “xyzzy”.");
    assert!(NO_MATCH_HINT.contains("“ctrl+z”"));
    assert_eq!(search(&groups, "   "), groups);
    assert_eq!(search(&groups, ""), groups);
    // Grupo sem resultado some, e a ordem é a da lista inteira.
    let save = search(&groups, "salvar");
    let titles: Vec<&str> = save.iter().map(|g| g.title).collect();
    assert_eq!(titles, ["Arquivo"]);
    assert_eq!(all_rows(&save), ["Salvar", "Salvar como…"]);
    let moves = search(&groups, "mover");
    let titles: Vec<&str> = moves.iter().map(|g| g.title).collect();
    assert_eq!(titles, ["Lâmina", ORGANIZE, CROP], "a ordem da lista");
}

#[test]
fn at_b8_teclas_do_mac() {
    let mac = sheet(Platform::Mac);
    assert_eq!(keys(&mac, "Salvar como…"), ["⇧⌘S"]);
    assert_eq!(keys(&mac, "Faixa de lâminas"), ["⌥1"]);
    assert_eq!(keys(&mac, "Atalhos do teclado…"), ["⌘/", "F1"]);
    assert_eq!(keys(&mac, "Tirar foto ou apagar quadro"), ["⌦", "⌫"]);
    assert_eq!(keys(&mac, "Montar lâmina"), ["↩"]);
    assert_eq!(keys(&mac, "Endireitar −1°"), ["⇧["]);
    assert!(keys(&mac, "Escolher várias fotos")[0].starts_with("⌘+clique"));
    assert_eq!(keys(&mac, "Apagar lâmina"), ["⌘⌦"]);
    for pc in [Platform::Windows, Platform::Linux] {
        let list = sheet(pc);
        assert_eq!(keys(&list, "Salvar como…"), ["Ctrl+Shift+S"]);
        assert_eq!(keys(&list, "Faixa de lâminas"), ["Alt+1"]);
        assert_eq!(keys(&list, "Atalhos do teclado…"), ["Ctrl+/", "F1"]);
        assert_eq!(
            keys(&list, "Tirar foto ou apagar quadro"),
            ["Delete", "Backspace"]
        );
        assert_eq!(keys(&list, "Montar lâmina"), ["Enter"]);
        assert_eq!(keys(&list, "Endireitar −1°"), ["Shift+["]);
        assert!(keys(&list, "Escolher várias fotos")[0].starts_with("Ctrl+clique"));
    }
    // No Windows e no Linux, o texto de hoje.
    for c in Command::ALL {
        if let Some(s) = c.shortcut() {
            assert_eq!(s.text_on(Platform::Windows), s.text());
            assert_eq!(s.text_on(Platform::Linux), s.text());
        }
    }
    for query in ["cmd+s", "⌘s", "⌘ S"] {
        assert_eq!(all_rows(&search(&mac, query)), ["Salvar"], "{query}");
    }
    assert_eq!(all_rows(&search(&mac, "shift+cmd+s")), ["Salvar como…"]);
}

#[test]
fn at_b11_arrastar_e_clicar_no_topo() {
    let groups = sheet(Platform::Windows);
    let first = &groups[0];
    assert_eq!(first.title, DRAG_AND_CLICK);
    let rows: Vec<&str> = first.rows.iter().map(|r| r.does).collect();
    assert_eq!(
        rows,
        [
            "Trocar duas fotos de lugar",
            "Pôr uma foto num quadro",
            "Acrescentar fotos a uma lâmina",
            "Criar uma lâmina com fotos",
            "Importar fotos do computador",
            "Mudar uma lâmina de lugar",
            "Passar fotos para a lâmina vizinha",
            "Escolher várias fotos",
            "Recortar a foto",
            "Organizar a lâmina à mão, com réguas",
            "Ver as opções do quadro",
        ]
    );
    assert_eq!(
        first.rows[0].keys,
        ["arrastar uma foto da lâmina sobre a outra"]
    );
    assert!(first.rows.iter().all(|r| r.command.is_none()));
    let tail = |title: &str, n: usize| -> Vec<&'static str> {
        let g = groups.iter().find(|g| g.title == title).unwrap();
        let rows = &g.rows[g.rows.len() - n..];
        assert!(rows.iter().all(|r| r.command.is_none()), "{title}");
        rows.iter().map(|r| r.does).collect()
    };
    assert_eq!(
        tail("Lâmina", 2),
        [
            "Mover o quadro selecionado 1 mm",
            "Mover o quadro selecionado 10 mm"
        ]
    );
    assert_eq!(
        tail(ORGANIZE, 7),
        [
            "Mover os quadros selecionados",
            "Selecionar vários arrastando no vazio",
            "Somar ou tirar um quadro da seleção",
            "Ampliar o grupo",
            "Encaixar sem o ímã",
            "Mover a vista",
            "Aproximar e afastar a lâmina",
        ]
    );
    assert_eq!(tail(CROP, 2), ["Mover a foto no quadro", "Zoom da foto"]);
}
