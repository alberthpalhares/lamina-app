//! Cenários da spec `janela-enxuta` (JE) que olham as telas: a barra de cima enxuta (JE-B9) e o X
//! do quadro vazio no `Slot` da lâmina (JE-B1).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

use lamina_editor::{BAR_COMMANDS, Command};

fn ui(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
    std::fs::read_to_string(path).unwrap()
}

/// O trecho do componente `name` no arquivo (até o próximo `export component`).
fn component<'a>(text: &'a str, name: &str) -> &'a str {
    let start = text
        .find(&format!("component {name} "))
        .unwrap_or_else(|| panic!("{name} não está no arquivo"));
    let rest = &text[start..];
    let end = rest[1..]
        .find("\nexport component")
        .map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

/// Os nomes passados a `root.command("…")`, na ordem do arquivo.
fn commands(text: &str) -> Vec<String> {
    text.split("root.command(\"")
        .skip(1)
        .map(|part| part.split('"').next().unwrap().to_owned())
        .collect()
}

#[test]
fn je_b9_a_barra_de_cima_chama_cinco_comandos() {
    let topbar = ui("topbar.slint");
    let bar = component(&topbar, "TopBar");
    let names = commands(bar);
    // O botão da faixa de modelos entrou na barra (pedido do dono 2026-11-08, entre as faixas de
    // baixo e o fechar; mudança registrada na spec JE).
    assert_eq!(
        names,
        [
            "arquivo.fechar",
            "ver.faixa-layouts",
            "ver.faixas",
            "ver.painel",
            "arquivo.exportar"
        ],
        "um botão novo na barra pede mudar a spec JE"
    );
    let expected: Vec<&str> = BAR_COMMANDS.iter().map(|c| c.name()).collect();
    assert_eq!(names, expected);
    for name in &names {
        assert!(
            Command::from_name(name).is_some(),
            "{name} está no registro"
        );
    }
    // As dicas vêm do editor, nunca escritas à mão: nenhum atalho no texto da barra.
    assert!(!bar.contains("(Alt+") && !bar.contains("(Ctrl+"));
}

#[test]
fn je_b1_o_slot_liga_o_x_so_no_quadro_vazio_sem_imagem() {
    let canvas = ui("canvas.slint");
    let slot = component(&canvas, "Slot");
    let marks: Vec<&str> = slot.lines().filter(|l| l.contains("Path {")).collect();
    assert_eq!(marks.len(), 1, "um X só no Slot: {marks:?}");
    assert!(
        marks[0].contains("if f.kind == 0 && !f.has-img: Path {"),
        "o X só no quadro vazio, sem imagem: {}",
        marks[0]
    );
    assert!(slot.contains("commands: \"M0 0 L1 1 M1 0 L0 1\";"));
    assert!(
        slot.contains("clip: true;"),
        "o Slot recorta o X pelos cantos"
    );
}
