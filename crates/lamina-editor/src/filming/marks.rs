//! As marcas da filmagem: onde ficam os alvos do roteiro na janela (spec FI, regra 6).

use serde::Serialize;

/// As marcas repetidas: `quadro-2`, `foto-12` e `menu-3`.
const REPEATED: [&str; 3] = ["quadro-", "foto-", "menu-"];
/// Os prefixos repetidos das opções de um grupo: a opção i, como `pref-tema-op-2` (a 1ª é 0) e a
/// linha i de um menu, como `menu-alinhar-1` (a 1ª é 1).
const GROUP_PREFIXES: [&str; 4] = [
    "pref-tema-op-",
    "pref-texto-op-",
    "pref-fundo-op-",
    "menu-alinhar-",
];

/// O nome é de uma marca conhecida: da lista, repetida, ou de um grupo, como `quadro-2`.
pub fn is_known_mark(name: &str, known: &[String]) -> bool {
    known.iter().any(|k| k == name)
        || REPEATED.iter().chain(GROUP_PREFIXES.iter()).any(|prefix| {
            name.strip_prefix(prefix)
                .and_then(|n| n.parse::<u32>().ok())
                .is_some_and(|n| n > 0 || GROUP_PREFIXES.contains(prefix))
        })
}

/// O retângulo de uma marca, em pixels lógicos da janela.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MarkBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MarkBox {
    pub(super) fn center(self) -> (f64, f64) {
        (
            f64::from(self.x) + f64::from(self.w) / 2.0,
            f64::from(self.y) + f64::from(self.h) / 2.0,
        )
    }
}

/// O que a janela sabe responder à linha do tempo.
pub trait Scene {
    /// O retângulo da marca agora, se ela está na tela.
    fn mark(&mut self, name: &str) -> Option<MarkBox>;
    /// O comando pode rodar agora.
    fn available(&self, command: &str) -> bool;
}
