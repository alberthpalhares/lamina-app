//! Tipos públicos do gerador de layouts (specs TP e VL).

use lamina_core::{Mm, PageSide, RelRect};
use thiserror::Error;

use super::DEFAULT_GAP;

/// Como as fotos de um bloco são arrumadas (layout justificado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arrangement {
    /// Linhas horizontais; fotos da mesma linha com a mesma altura.
    Rows,
    /// Colunas verticais; fotos da mesma coluna com a mesma largura.
    Columns,
}

/// Arranjo de um bloco: o tipo e quantas fotos vão em cada linha (ou coluna), em ordem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageLayout {
    pub arrangement: Arrangement,
    pub groups: Vec<usize>,
}

/// Onde fica a faixa de apoio no destaque com apoio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportSide {
    Left,
    Right,
    Below,
}

/// Área de um destaque com apoio ou de um mosaico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Todas as fotos numa página.
    Page(PageSide),
    /// A lâmina inteira, cruzando a lombada (só no modo lâmina).
    Spread,
}

/// Família que gerou o candidato.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Family {
    /// Uma foto ocupando a lâmina inteira, por cima da lombada.
    Across,
    /// Fotos divididas entre as páginas; `None` = página sem fotos.
    PerPage {
        left: Option<PageLayout>,
        right: Option<PageLayout>,
    },
    /// Linhas ou colunas justificadas pela lâmina inteira, cruzando a lombada.
    WholeSpread(PageLayout),
    /// A primeira foto num bloco grande e as outras justificadas na faixa que sobra.
    Hero {
        scope: Scope,
        support: SupportSide,
        layout: PageLayout,
    },
    /// O mesmo arranjo nas duas páginas, a direita espelhando a esquerda.
    Mirrored(PageLayout),
    /// Uma foto cobre a página inteira até o corte; as outras ficam na outra página.
    BleedPage {
        side: PageSide,
        rest: Option<PageLayout>,
    },
    /// Células que enchem a área sem sobra, com recorte pequeno.
    Mosaic { scope: Scope, layout: PageLayout },
    /// As fotos chegam ao corte por fora; só o espaço do álbum as separa (spec SB).
    FullBleed(FullBleed),
    /// Um modelo salvo pelo fotógrafo (spec MD), talvez espelhado.
    Model { name: String, mirrored: bool },
}

/// Formato de uma lâmina sem borda (spec SB).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullBleed {
    /// Cada página ativa cheia até o corte; `None` = página inativa.
    Pages {
        left: Option<PageLayout>,
        right: Option<PageLayout>,
    },
    /// Linhas ou colunas no corte inteiro.
    Spread(PageLayout),
    /// A primeira foto grande até o corte e as outras enchendo a faixa de apoio.
    Hero {
        support: SupportSide,
        layout: PageLayout,
    },
}

/// Família sem os detalhes, usada no rodízio (spec VL, regra 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FamilyKind {
    Across,
    PerPage,
    WholeSpread,
    Hero,
    Mirrored,
    BleedPage,
    Mosaic,
    FullBleed,
    Model,
}

impl Family {
    pub fn kind(&self) -> FamilyKind {
        match self {
            Family::Across => FamilyKind::Across,
            Family::PerPage { .. } => FamilyKind::PerPage,
            Family::WholeSpread(_) => FamilyKind::WholeSpread,
            Family::Hero { .. } => FamilyKind::Hero,
            Family::Mirrored(_) => FamilyKind::Mirrored,
            Family::BleedPage { .. } => FamilyKind::BleedPage,
            Family::Mosaic { .. } => FamilyKind::Mosaic,
            Family::FullBleed(_) => FamilyKind::FullBleed,
            Family::Model { .. } => FamilyKind::Model,
        }
    }
}

/// Respiro de um arranjo sem recorte (spec VL, regra 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    /// Bloco centralizado na área.
    #[default]
    Center,
    /// Encostado em cima (ou à esquerda, se o que sobra é largura).
    Start,
    /// Encostado embaixo (ou à direita).
    End,
}

/// Margem extra (0, 1 = 8%, 2 = 16%, 3 = 24% de cada lado) e alinhamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Breathing {
    pub margin_level: u8,
    pub align: Align,
}

/// Nota de um candidato e as partes que a formam (spec VL, regra 6).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Score {
    /// Soma das partes, de 0 a 100.
    pub total: f64,
    /// Até 65: área das fotos ÷ área do corte das páginas ativas (spec SB, regra 6).
    pub coverage: f64,
    /// Até 20: menor quadro ÷ quadro médio.
    pub smallest: f64,
    /// Até 15: áreas parecidas nas duas páginas.
    pub balance: f64,
    /// De −15 a 0: recorte médio.
    pub crop: f64,
}

/// Uma opção de arranjo para as fotos de uma lâmina.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutCandidate {
    pub family: Family,
    pub breathing: Breathing,
    /// Um quadro por foto, na ordem **original** das fotos, relativo ao corte da lâmina.
    pub frames: Vec<RelRect>,
    /// Sequência das fotos (índices originais) usada para montar o arranjo.
    pub order: Vec<usize>,
    /// Fração de cada foto que fica de fora do quadro (0 = inteira), na ordem original.
    pub crop: Vec<f64>,
    /// Área dos quadros ÷ área útil das páginas ativas (no máximo 1).
    pub coverage: f64,
    pub score: Score,
    /// Borda, cantos e textos, quando o candidato é um modelo do fotógrafo (spec MD).
    pub model: Option<super::ModelLook>,
}

/// Opções da variedade de layouts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOptions {
    pub gap: Mm,
    /// Máximo de candidatos em cada lista (padrão 200).
    pub max_candidates: usize,
    /// Calcular também as sugestões reordenadas (padrão sim; a montagem automática usa só "na ordem").
    pub reordered: bool,
    /// Quanto recortar e quais famílias opcionais usar (spec NP; padrão = o de sempre).
    pub mounting: lamina_core::Mounting,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            gap: DEFAULT_GAP,
            max_candidates: 200,
            reordered: true,
            mounting: lamina_core::Mounting::default(),
        }
    }
}

/// As duas listas de sugestões (spec VL, seção 4).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutLists {
    /// As fotos na ordem recebida.
    pub in_order: Vec<LayoutCandidate>,
    /// Fotos reordenadas (só com orientações misturadas).
    pub reordered: Vec<LayoutCandidate>,
}

/// Erros do gerador, com as mensagens da spec.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum LayoutError {
    #[error("A lâmina não tem fotos.")]
    NoPhotos,
    #[error("São {photos} fotos para {pages} páginas: cabem no máximo {capacity}.")]
    TooManyPhotos {
        photos: usize,
        pages: usize,
        capacity: usize,
    },
    #[error("Proporção inválida na foto {0}.")]
    InvalidAspect(usize),
    #[error("O espaço entre fotos não cabe na página.")]
    GapTooLarge,
}
