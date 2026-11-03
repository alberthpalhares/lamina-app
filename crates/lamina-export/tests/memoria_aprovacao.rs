//! AC-B25 (spec `aprovacao-cliente`): o pico de memória dos JPG para web, medido pelo `dhat`. Fica
//! num binário de teste só dele, porque o `dhat` troca o alocador do programa.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, Project, RelRect};
use lamina_export::{ApprovalOptions, export_approval};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const MIB: u64 = 1024 * 1024;

/// Até aqui a fonte da aprovação usa a cópia de tela (spec AC, regra 17).
const COPY_LIMIT: u32 = 2400;

/// 10 lâminas de 60 × 30 cm, com 12 fotos declaradas de 24 MP em cada.
fn album() -> Project {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut project = Project::new("teste", spec);
    for s in 0..10 {
        let spread = project.add_spread();
        for n in 0..12u32 {
            let photo = project.add_photo(format!("f{s}_{n}.jpg"), 6000, 4000);
            let (col, row) = (f64::from(n % 4), f64::from(n / 4));
            let rect = RelRect::new(0.02 + col * 0.245, 0.03 + row * 0.32, 0.22, 0.29).unwrap();
            let content = FrameContent::Photo {
                photo,
                crop: Crop::default(),
            };
            project.add_frame(spread, rect, content).unwrap();
        }
    }
    project
}

#[test]
fn ac_b25_gerar_fica_abaixo_de_1_gb_e_nao_pede_o_original() {
    let _profiler = dhat::Profiler::builder().testing().build();
    let project = album();
    let dir = tempfile::tempdir().unwrap();
    let source = |_: &Photo, needed: (u32, u32)| {
        if needed.0.max(needed.1) > COPY_LIMIT {
            return Err(format!("pediu o original: {needed:?}"));
        }
        // Um vetor preenchido de verdade: a memória de uma cópia de tela aberta.
        Ok(Arc::new(RgbaImage::from_pixel(
            1600,
            1067,
            image::Rgba([90, 120, 150, 255]),
        )))
    };
    let report = export_approval(
        &project,
        dir.path(),
        &source,
        &ApprovalOptions::default(),
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(report.files.len(), 10);
    let peak = dhat::HeapStats::get().max_bytes as u64 / MIB;
    println!("pico dos JPG para web: {peak} MiB");
    assert!(peak < 1024, "pico de {peak} MiB");
}
