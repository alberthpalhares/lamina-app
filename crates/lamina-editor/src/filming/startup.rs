//! O que roda na abertura do Lamina, em cada modo (spec FI, regra 5).

/// Como o programa foi chamado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Aberto pelo fotógrafo.
    Normal,
    /// A verificação da abertura de um arquivo (spec MA).
    OpenFile,
    /// O teste de fumaça do álbum (`--verificar`).
    Verify,
    /// O teste de fumaça da importação (`--verificar-importacao`).
    Import,
    /// A filmagem da interface.
    Film,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Startup {
    /// As novidades da versão nova.
    pub news: bool,
    /// A pergunta da cópia de segurança.
    pub backup_ask: bool,
    /// O temporizador da cópia de segurança.
    pub backup_timer: bool,
    /// A conferência dos recentes.
    pub recents_check: bool,
    /// A checagem de atualização.
    pub update_check: bool,
}

impl Startup {
    pub fn for_mode(mode: Mode) -> Startup {
        let on = matches!(mode, Mode::Normal | Mode::OpenFile);
        Startup {
            news: on,
            backup_ask: on,
            backup_timer: mode != Mode::Film,
            recents_check: mode != Mode::Film,
            update_check: mode == Mode::Normal,
        }
    }
}
