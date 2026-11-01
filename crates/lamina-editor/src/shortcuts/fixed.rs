//! As linhas fixas da lista (spec AT, regra 4): os gestos do mouse e as teclas que não estão na
//! tabela de comandos (Alt+setas e Espaço). Uma spec que cria um gesto novo acrescenta a linha
//! dela aqui, no mesmo PR.

use super::{CROP, DRAG_AND_CLICK, ORGANIZE, Row};
use crate::keys::Platform;

/// Grupo, o que faz e como, na ordem da seção 4. "Ctrl", "Alt" e "Shift" viram ⌘, ⌥ e ⇧ no Mac.
const FIXED: [(&str, &str, &str); 22] = [
    (
        DRAG_AND_CLICK,
        "Trocar duas fotos de lugar",
        "arrastar uma foto da lâmina sobre a outra",
    ),
    (
        DRAG_AND_CLICK,
        "Pôr uma foto num quadro",
        "arrastar do painel de fotos para o quadro",
    ),
    (
        DRAG_AND_CLICK,
        "Acrescentar fotos a uma lâmina",
        "arrastar fotos para a lâmina, fora dos quadros",
    ),
    (
        DRAG_AND_CLICK,
        "Criar uma lâmina com fotos",
        "arrastar fotos para “+ Lâmina nova” ou entre duas lâminas",
    ),
    (
        DRAG_AND_CLICK,
        "Importar fotos do computador",
        "arrastar arquivos ou pastas do Explorador (Finder no Mac) para a janela",
    ),
    (
        DRAG_AND_CLICK,
        "Mudar uma lâmina de lugar",
        "arrastar a lâmina na faixa de lâminas",
    ),
    (
        DRAG_AND_CLICK,
        "Passar fotos para a lâmina vizinha",
        "arrastar o corte na linha do tempo",
    ),
    (
        DRAG_AND_CLICK,
        "Escolher várias fotos",
        "Ctrl+clique (uma a uma) ou Shift+clique (um trecho) no painel de fotos",
    ),
    (DRAG_AND_CLICK, "Recortar a foto", "duplo clique na foto"),
    (
        DRAG_AND_CLICK,
        "Organizar a lâmina à mão, com réguas",
        "duplo clique fora dos quadros",
    ),
    (
        DRAG_AND_CLICK,
        "Ver as opções do quadro",
        "clique direito no quadro ou no fundo",
    ),
    ("Lâmina", "Mover o quadro selecionado 1 mm", "Alt+setas"),
    (
        "Lâmina",
        "Mover o quadro selecionado 10 mm",
        "Alt+Shift+setas",
    ),
    (
        ORGANIZE,
        "Mover os quadros selecionados",
        "arrastar um deles",
    ),
    (
        ORGANIZE,
        "Selecionar vários arrastando no vazio",
        "arrastar no vazio (com Shift, soma)",
    ),
    (
        ORGANIZE,
        "Somar ou tirar um quadro da seleção",
        "Ctrl+clique ou Shift+clique",
    ),
    (
        ORGANIZE,
        "Ampliar o grupo",
        "puxar um canto (com Shift, muda a proporção)",
    ),
    (ORGANIZE, "Encaixar sem o ímã", "Alt durante o arrasto"),
    (ORGANIZE, "Mover a vista", "Espaço + arrastar"),
    (
        ORGANIZE,
        "Aproximar e afastar a lâmina",
        "Ctrl+roda do mouse",
    ),
    (CROP, "Mover a foto no quadro", "arrastar"),
    (CROP, "Zoom da foto", "roda do mouse"),
];

/// As linhas fixas do grupo `title`, com os modificadores do sistema (regra 5).
pub(super) fn rows(title: &str, platform: Platform) -> impl Iterator<Item = Row> {
    FIXED
        .iter()
        .filter(move |(group, _, _)| *group == title)
        .map(move |&(_, does, how)| Row {
            does,
            keys: vec![how_on(how, platform)],
            command: None,
        })
}

/// O "como" de uma linha fixa no sistema: no Mac, os símbolos com "+" ("⌘+clique").
fn how_on(how: &str, platform: Platform) -> String {
    if platform == Platform::Mac {
        how.replace("Ctrl", "⌘")
            .replace("Alt", "⌥")
            .replace("Shift", "⇧")
    } else {
        how.to_owned()
    }
}
