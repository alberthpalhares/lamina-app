//! A ordem personalizada pelo editor (spec OF): mover e ver em shown_photos.

use super::tl_common::{H, built, editor_at, spec};

#[test]
fn of_b22_mover_e_ver_a_ordem() {
    let project = built(spec(30.0, 30.0), &[H; 8], 2);
    let mut editor = editor_at(project, 0);
    editor.set_photo_sort(crate::ordering::SortBy::Custom);
    let before = editor.shown_photos();
    // Mover a foto da posição 3 para 0.
    let id = before[3];
    assert!(editor.move_photo_to(id, 0));
    let after = editor.shown_photos();
    assert_ne!(before, after);
    assert_eq!(after[0], id, "a foto movida está na posição 0");
}
