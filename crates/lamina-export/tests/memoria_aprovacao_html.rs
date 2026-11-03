//! AH-B20 (spec `aprovacao-cliente-html`): o pico de memória do HTML de 40 lâminas, medido pelo
//! `dhat`. Fica num binário de teste só dele, porque o `dhat` troca o alocador do programa.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use image::RgbaImage;
use lamina_core::{AlbumSpec, Crop, Dpi, FrameContent, Mm, Photo, Project, RelRect};
use lamina_export::{ApprovalHtml, ApprovalOptions, export_approval};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const MIB: u64 = 1024 * 1024;

fn album() -> Project {
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
        .unwrap()
        .with_bleed(Mm(3.0))
        .unwrap();
    let mut project = Project::new("teste", spec);
    for s in 0..40 {
        let spread = project.add_spread();
        for n in 0..6u32 {
            let photo = project.add_photo(format!("f{s}_{n}.jpg"), 6000, 4000);
            let (col, row) = (f64::from(n % 2), f64::from(n / 2));
            let rect = RelRect::new(0.03 + col * 0.5, 0.04 + row * 0.32, 0.44, 0.29).unwrap();
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
fn ah_b20_gerar_o_html_fica_abaixo_de_1_gb() {
    let _profiler = dhat::Profiler::builder().testing().build();
    let project = album();
    let dir = tempfile::tempdir().unwrap();
    let source = |_: &Photo, _: (u32, u32)| {
        // Um vetor preenchido de verdade: a memória de uma cópia de tela aberta.
        Ok(Arc::new(RgbaImage::from_pixel(
            1600,
            1067,
            image::Rgba([90, 120, 150, 255]),
        )))
    };
    let options = ApprovalOptions {
        jpg: false,
        html: Some(ApprovalHtml {
            name: "Ana".to_owned(),
            whatsapp: None,
            email: None,
        }),
        ..ApprovalOptions::default()
    };
    let report = export_approval(&project, dir.path(), &source, &options, &|_, _| {}).unwrap();
    assert_eq!(report.files.len(), 1);
    let peak = dhat::HeapStats::get().max_bytes as u64 / MIB;
    println!("pico do HTML: {peak} MiB");
    assert!(peak < 1024, "pico de {peak} MiB");
}
