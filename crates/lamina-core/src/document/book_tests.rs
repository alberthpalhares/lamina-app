//! Cenários do Livro na spec `folhear` (FO-B1 a FO-B10).

use super::{Book, CoverMeasures, Face, Leaf, Project, Spine};
use crate::{AlbumEdge, AlbumSpec, Dpi, Mm, PageSide};

const FULL: AlbumEdge = AlbumEdge::FullSpread;
const SINGLE: AlbumEdge = AlbumEdge::SinglePage;

fn left(index: usize) -> Face {
    Face::Spread {
        index,
        side: PageSide::Left,
    }
}

fn right(index: usize) -> Face {
    Face::Spread {
        index,
        side: PageSide::Right,
    }
}

fn spec() -> AlbumSpec {
    AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap()).unwrap()
}

fn album(spreads: usize, start: AlbumEdge, end: AlbumEdge) -> Project {
    let mut project = Project::new("teste", spec().with_edges(start, end));
    for _ in 0..spreads {
        project.add_spread();
    }
    project
}

fn middle_leaves() -> Vec<Leaf> {
    vec![
        Leaf {
            front: right(0),
            back: left(1),
        },
        Leaf {
            front: right(1),
            back: left(2),
        },
    ]
}

#[test]
fn fo_b1_pontas_inteiras() {
    let book = Book::from_project(&album(3, FULL, FULL));
    assert_eq!(book.inside_front, left(0));
    assert_eq!(book.leaves, middle_leaves());
    assert_eq!(book.inside_back, right(2));
}

#[test]
fn fo_b2_pontas_de_pagina_simples() {
    let book = Book::from_project(&album(3, SINGLE, SINGLE));
    assert_eq!(book.inside_front, Face::Blank);
    assert_eq!(book.inside_back, Face::Blank);
    assert_eq!(book.leaves, middle_leaves());
}

#[test]
fn fo_b3_pontas_diferentes() {
    let book = Book::from_project(&album(3, SINGLE, FULL));
    assert_eq!(book.inside_front, Face::Blank);
    assert_eq!(book.inside_back, right(2));
    let book = Book::from_project(&album(3, FULL, SINGLE));
    assert_eq!(book.inside_front, left(0));
    assert_eq!(book.inside_back, Face::Blank);
}

#[test]
fn fo_b4_uma_lamina_so() {
    let book = Book::from_project(&album(1, FULL, FULL));
    assert!(book.leaves.is_empty());
    assert_eq!(book.inside_front, left(0));
    assert_eq!(book.inside_back, right(0));
}

#[test]
fn fo_b5_foto_que_cruza_a_dobra() {
    let spec = spec().with_bleed(Mm(3.0)).unwrap();
    let a = left(0).rect(&spec).unwrap();
    let b = right(0).rect(&spec).unwrap();
    let trim = spec.trim_rect();
    assert_eq!((a.width.get(), a.height.get()), (300.0, 300.0));
    assert!((a.width.get() + b.width.get() - trim.width.get()).abs() < 1e-9);
    assert!((a.x.get() - trim.x.get()).abs() < 1e-9, "começa no corte");
    assert!(
        (a.right().get() - b.x.get()).abs() < 1e-9,
        "as duas na dobra"
    );
    assert!((b.right().get() - trim.right().get()).abs() < 1e-9);
    assert_eq!(Face::Blank.rect(&spec), None);
}

#[test]
fn fo_b6_capa_maior_que_a_pagina() {
    let mut project = album(15, FULL, FULL);
    let measures = CoverMeasures::defaults(&project.spec);
    project.create_cover(measures).unwrap();
    let book = Book::from_project(&project);
    let cover = book.cover;
    assert!(!cover.neutral);
    assert_eq!(cover.face_width, Mm(303.0));
    assert_eq!(cover.face_height, Mm(306.0));
    assert_eq!(cover.spine_width, Mm(15.0));
    assert_eq!(book.page_width, Mm(300.0));
    // Verso, lombada e frente, a partir do corte da capa, sem a dobra.
    assert_eq!(cover.back.x, Mm::ZERO);
    assert_eq!(cover.back.width, Mm(303.0));
    assert_eq!(cover.front.x, Mm(318.0));
    assert_eq!(cover.front.width, Mm(303.0));
    assert_eq!(cover.front.y, Mm::ZERO);
    assert_eq!(cover.front.height, Mm(306.0));
}

#[test]
fn fo_b7_capa_menor_e_de_outra_proporcao() {
    let mut project = album(3, FULL, FULL);
    let mut measures = CoverMeasures::defaults(&project.spec);
    measures.face_width = Mm(290.0);
    measures.face_height = Mm(320.0);
    measures.spine = Spine::Fixed {
        mm: Mm(10.0),
        pages: 6,
    };
    project.create_cover(measures).unwrap();
    let book = Book::from_project(&project);
    assert_eq!(book.cover.face_width, Mm(290.0));
    assert_eq!(book.cover.face_height, Mm(320.0));
    assert_eq!(book.page_width, Mm(300.0));
    assert_eq!(book.page_height, Mm(300.0));
}

#[test]
fn fo_b8_pagina_em_branco() {
    let mut project = album(3, FULL, FULL);
    project.spreads[1].blank.right = true;
    let book = Book::from_project(&project);
    assert_eq!(book.leaves[1].front, right(1));
    assert_eq!(book.leaves.len(), 2);
}

#[test]
fn fo_b9_sem_capa() {
    let book = Book::from_project(&album(15, FULL, FULL));
    assert!(book.cover.neutral);
    assert_eq!(book.cover.face_width, Mm(303.0));
    assert_eq!(book.cover.face_height, Mm(306.0));
    assert_eq!(book.cover.spine_width, Mm(15.0));
}

#[test]
fn fo_b10_sem_laminas() {
    let book = Book::from_project(&album(0, FULL, FULL));
    assert!(book.leaves.is_empty());
    assert_eq!(book.inside_front, Face::Blank);
    assert_eq!(book.inside_back, Face::Blank);
    assert!(book.cover.neutral);
}
