//! O Livro (spec FO): como um projeto vira capa e folhas, em milímetros, sem imagens e sem mudar o
//! projeto. A exportação em HTML (spec AH) o usa para o cliente folhear.

use super::{CoverMeasures, Project};
use crate::{AlbumEdge, AlbumSpec, Mm, PageSide, Rect};

/// Um lado de uma folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Uma página de uma lâmina (começando em 0), mesmo em branco (regra 5).
    Spread { index: usize, side: PageSide },
    /// A guarda neutra, que não pertence a nenhuma lâmina.
    Blank,
}

impl Face {
    /// O retângulo da face no corte da lâmina: as duas faces de uma lâmina somam o corte inteiro, e
    /// a foto que cruza a dobra se divide nela (regra 4). A guarda não tem retângulo.
    pub fn rect(self, spec: &AlbumSpec) -> Option<Rect> {
        match self {
            Face::Spread { side, .. } => Some(spec.page_rect(side)),
            Face::Blank => None,
        }
    }
}

/// Uma folha solta: vira inteira, com uma face de cada lado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leaf {
    pub front: Face,
    pub back: Face,
}

/// A capa do livro, em milímetros. `front` e `back` são retângulos a partir do corte da capa aberta,
/// sem a dobra.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BookCover {
    pub front: Rect,
    pub back: Rect,
    pub spine_width: Mm,
    pub face_width: Mm,
    pub face_height: Mm,
    /// Sem capa criada: as medidas são as padrão, e a tela avisa (regra 7).
    pub neutral: bool,
}

/// O álbum como livro: a capa, as guardas e as folhas soltas entre elas.
#[derive(Debug, Clone, PartialEq)]
pub struct Book {
    pub cover: BookCover,
    pub page_width: Mm,
    pub page_height: Mm,
    /// O verso interno da capa: a página esquerda da lâmina 1 ou a guarda (regra 2).
    pub inside_front: Face,
    /// O verso interno da contracapa: a página direita da última lâmina ou a guarda (regra 3).
    pub inside_back: Face,
    pub leaves: Vec<Leaf>,
}

impl Book {
    /// O livro de um projeto. Não falha: um projeto válido sempre vira um livro (seção 6).
    pub fn from_project(project: &Project) -> Book {
        let spec = &project.spec;
        let total = project.spreads.len();
        let edges = spec.edges();
        let side_of = |index: usize, side: PageSide| Face::Spread { index, side };
        let inside_front = match (total, edges.start) {
            (1.., AlbumEdge::FullSpread) => side_of(0, PageSide::Left),
            _ => Face::Blank,
        };
        let inside_back = match (total, edges.end) {
            (1.., AlbumEdge::FullSpread) => side_of(total - 1, PageSide::Right),
            _ => Face::Blank,
        };
        let leaves = (1..total)
            .map(|next| Leaf {
                front: side_of(next - 1, PageSide::Right),
                back: side_of(next, PageSide::Left),
            })
            .collect();
        Book {
            cover: book_cover(project),
            page_width: spec.page_width(),
            page_height: spec.page_height(),
            inside_front,
            inside_back,
            leaves,
        }
    }
}

/// As medidas da capa criada ou, sem ela, as padrão da spec CA (regra 7).
fn book_cover(project: &Project) -> BookCover {
    let created = project.cover.as_ref().map(|cover| cover.measures);
    let neutral = created.is_none();
    let measures = created.unwrap_or_else(|| CoverMeasures::defaults(&project.spec));
    let spine_width = measures.spine_width(project.page_count());
    let (width, height) = (measures.face_width, measures.face_height);
    BookCover {
        back: Rect::new(Mm::ZERO, Mm::ZERO, width, height),
        front: Rect::new(Mm(width.get() + spine_width.get()), Mm::ZERO, width, height),
        spine_width,
        face_width: width,
        face_height: height,
        neutral,
    }
}
