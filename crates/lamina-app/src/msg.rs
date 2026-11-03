//! As mensagens do trabalho em segundo plano para a linha da janela (a caixa de mensagens,
//! `mailbox`). Saíram do `state.rs` quando ele passou do tamanho (spec TR; AGENTS, regra 7).

use std::path::PathBuf;

use lamina_core::{PhotoId, Project};
use lamina_editor::{Found, HumanMessage, Step};

/// Mensagens do trabalho em segundo plano para a linha da janela.
#[derive(Debug)]
pub enum Msg {
    Step(Step),
    Album(Result<(Project, usize), HumanMessage>),
    ImagesReady,
    /// O que o sistema diz do movimento (spec AJ, regra 18); `None` = sem resposta.
    SystemMotion(Option<bool>),
    /// Uma foto de tela lida pelo trabalhador (spec MV, regra 1); `None` sem imagem de tela.
    Screen {
        epoch: u64,
        id: PhotoId,
        pixels: Option<crate::images::Pixels>,
    },
    /// A foto original do quadro em modo recorte (spec RC, regra 9), já em pixels da janela.
    Original(PhotoId, crate::images::Pixels),
    /// Andamento e fim da exportação (spec EI): o resumo ou o erro, e a pasta.
    Exporting(usize, usize),
    Exported(Result<String, HumanMessage>, PathBuf),
    /// Andamento e fim da aprovação do cliente (spec AC): o resumo ou o erro, e a pasta.
    Approving(usize, usize),
    Approved(Result<String, HumanMessage>, PathBuf),
    /// Andamento e resultado da procura de fotos ausentes (spec EI). `file` é o projeto em que a
    /// procura começou; `folder`, a pasta escolhida (nenhuma = a procura ao abrir).
    Searching(usize, usize),
    Relinked {
        file: Option<PathBuf>,
        found: Vec<Found>,
        folder: Option<PathBuf>,
    },
    /// "Tentar de novo" e a volta para a janela (spec RF, regra 6): as fotos que ainda faltam nos
    /// mesmos caminhos do projeto `file`.
    Rechecked {
        file: Option<PathBuf>,
        still: Vec<lamina_core::PhotoId>,
    },
    /// O que aconteceu com as fotos que sumiram com o álbum aberto (spec RF, regra 11).
    Discovered {
        file: Option<PathBuf>,
        found: Vec<lamina_editor::Discovery>,
    },
    /// "Localizar esta foto…" (spec RF, regra 4) no projeto `file`.
    Located {
        file: Option<PathBuf>,
        result: Result<lamina_editor::Located, HumanMessage>,
    },
    /// A resposta de uma checagem ou download da atualização (spec DS).
    Update(lamina_editor::updates::Outcome),
    /// Uma tarefa em segundo plano entrou em pânico (spec EH, regra 6): a janela se libera.
    Failed(HumanMessage),
    /// Os recentes cujo arquivo sumiu e a miniatura gravada (spec TI, regras 4 e 5).
    RecentsChecked(Vec<PathBuf>),
    ThumbReady,
    /// Uma imagem de lâmina do rolo (spec RL, regra 10), já em colunas; ou a falha, só para o
    /// registro de erros.
    SpreadImage(Box<Result<crate::roll_worker::Painted, HumanMessage>>),
    /// Uma miniatura lida pelo trabalhador (spec TR, regra 6); `None` sem miniatura no cache.
    Thumb {
        epoch: u64,
        id: PhotoId,
        pixels: Option<crate::thumb_loader::ThumbPixels>,
    },
    /// "Montar o álbum…" terminou (spec BR): o álbum montado, as lâminas novas e as fotos.
    Built(Result<(Project, usize, usize), HumanMessage>),
    /// Andamento e fim de uma importação no álbum aberto (spec IM); `file` é o álbum em que ela
    /// começou.
    Importing(usize, usize),
    /// `Some(n)`: soltas na área de lâmina nova, viram a lâmina nova no vão `n` (spec IM, regra 11).
    Imported(
        Result<lamina_editor::importing::ImportReport, HumanMessage>,
        Option<PathBuf>,
        Option<usize>,
    ),
}
