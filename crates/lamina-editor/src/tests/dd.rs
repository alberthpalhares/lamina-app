//! Cenários da spec `design-dialogos` (DD) que são decisão do editor: a qualidade por formato, a
//! pergunta de substituir, a falta de pasta como dica e o caminho curto.

use std::path::Path;

use lamina_core::{ColorSpace, PhotoId};
use lamina_export::CoverExport;

use crate::exporting::ExportFormat;
use crate::forms::{
    ExportForm, album_name_error, quality_applies, replace_question, replace_title, short_path,
};
use crate::motion::{TimelineKey, timeline_step};
use crate::new_project::{PICK_PHOTOS, StepThreeState};

fn form(format: ExportFormat) -> ExportForm {
    ExportForm {
        quality: 80,
        all_spreads: true,
        range: String::new(),
        pages: false,
        format,
        crop_marks: false,
        space: ColorSpace::Srgb,
        cover: CoverExport::WithSpreads,
    }
}

#[test]
fn dd_b7_a_qualidade_aparece_em_jpeg_e_pdf_some_em_tiff_e_continua_no_numero() {
    assert!(quality_applies(ExportFormat::Jpeg));
    assert!(quality_applies(ExportFormat::Pdf));
    assert!(!quality_applies(ExportFormat::Tiff));
    // O número não muda ao passar pelo TIFF, e o PDF sai com a qualidade escolhida.
    for format in [
        ExportFormat::Jpeg,
        ExportFormat::Pdf,
        ExportFormat::Tiff,
        ExportFormat::Jpeg,
    ] {
        assert_eq!(form(format).options(false).quality, 80);
    }
}

#[test]
fn dd_b8_a_pergunta_de_substituir_fala_de_arquivos_deste_album() {
    let folder = Path::new("Fotos").join("exportacao");
    assert_eq!(replace_title(5), "Substituir os 5 arquivos?");
    assert_eq!(
        replace_question(5, &folder),
        "A pasta exportacao já tem 5 arquivos deste álbum. Os novos entram no lugar deles, e os \
         que sobrarem de uma exportação maior também saem. Seus outros arquivos continuam lá."
    );
    assert_eq!(replace_title(1), "Substituir o arquivo?");
    let one = replace_question(1, &folder);
    assert!(one.starts_with("A pasta exportacao já tem 1 arquivo deste álbum. O novo entra"));
    for text in [one, replace_question(5, &folder)] {
        assert!(!text.contains("lamina-NN") && !text.contains("pagina-NN"));
    }
}

#[test]
fn dd_b14_falta_de_pasta_e_dica_e_nome_que_nao_vale_e_erro() {
    let no_folder = StepThreeState::new(None, true);
    assert_eq!(no_folder.hint, PICK_PHOTOS);
    assert_eq!(no_folder.error, "");
    assert!(!no_folder.can_create(), "Criar fica apagado");

    let no_name = StepThreeState::new(album_name_error("  ").map(str::to_owned), false);
    assert_eq!(no_name.hint, "");
    assert_eq!(no_name.error, "Dê um nome ao álbum.");
    assert!(!no_name.can_create());

    assert!(StepThreeState::new(None, false).can_create());
}

#[test]
fn dd_b15_o_caminho_curto_corta_o_meio() {
    let long = r"C:\Users\ana\Pictures\Trabalhos\2026\Casamentos\Ensaio na praia.lamina";
    let short = short_path(long, 40);
    assert_eq!(short, r"C:\…\Casamentos\Ensaio na praia.lamina");
    assert!(short.chars().count() <= 40);
    assert_eq!(short_path(r"D:\a.lamina", 40), r"D:\a.lamina");
}

#[test]
fn dd_b18_a_sequencia_anda_so_pelas_fotos() {
    // O id não importa aqui: só onde há foto e onde há corte.
    let id: PhotoId = serde_json::from_str("1").unwrap();
    let photo = TimelineKey::Photo(id, 0);
    // Lâmina 0: duas fotos; corte; lâmina 1: uma foto; corte; lâmina 2: sem fotos.
    let rows = vec![
        (photo, 0),
        (photo, 0),
        (TimelineKey::Cut(0), 0),
        (photo, 1),
        (TimelineKey::Cut(1), 1),
    ];
    // Sem marca: a primeira foto da lâmina atual; numa lâmina vazia no fim, a última foto.
    assert_eq!(timeline_step(&rows, None, true, 1), Some(3));
    assert_eq!(timeline_step(&rows, None, true, 2), Some(3));
    // → pula o corte, ← também, e nas pontas fica onde está.
    assert_eq!(timeline_step(&rows, Some(1), true, 0), Some(3));
    assert_eq!(timeline_step(&rows, Some(3), false, 0), Some(1));
    assert_eq!(timeline_step(&rows, Some(3), true, 0), Some(3));
    assert_eq!(timeline_step(&rows, Some(0), false, 0), Some(0));
    assert_eq!(timeline_step(&[], None, true, 0), None);
}

#[test]
fn caminho_curto_de_nome_comprido_e_sem_barra_nao_passa_do_limite() {
    let name = "x".repeat(60);
    let short = short_path(&format!("D:\\{name}"), 40);
    assert!(short.starts_with("D:\\…"));
    assert_eq!(short.chars().count(), 40);
    let bare = short_path(&name, 40);
    assert!(bare.starts_with('…'));
    assert_eq!(bare.chars().count(), 40);
}
