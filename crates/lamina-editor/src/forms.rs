//! Os campos e textos das janelas, decididos aqui e não no `lamina-app` (regra 1; auditoria do
//! Marco 2, M4): os campos do "Novo álbum" e da "Exportar", os passos do painel de aparência, a
//! dica da barra de baixo e os textos do painel de informações.

use lamina_core::{PageLimits, Project};

use crate::Editor;
use crate::album::SpecFields;
use crate::photos::PhotoInfo;

pub use crate::export_form::{ExportForm, format_index, replace_question};
pub use crate::look_form::{BORDER_FIRST, LookField};

/// Número com no máximo uma casa e vírgula: 30 → "30", 2,5 → "2,5".
pub fn decimal(v: f64) -> String {
    format!("{}", (v * 10.0).round() / 10.0).replace('.', ",")
}

/// "2,5 mm".
pub fn mm_text(v: f64) -> String {
    format!("{} mm", decimal(v))
}

fn parse_decimal(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok()
}

/// Os campos da janela "Novo álbum" como o fotógrafo digitou (spec JP, seção 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecText {
    pub width_cm: String,
    pub height_cm: String,
    pub dpi: String,
    pub bleed_mm: String,
    pub safe_mm: String,
    /// Vazio = igual à zona segura.
    pub spine_mm: String,
    pub starts_single: bool,
    pub ends_single: bool,
}

impl From<&SpecFields> for SpecText {
    fn from(f: &SpecFields) -> Self {
        SpecText {
            width_cm: decimal(f.width_cm),
            height_cm: decimal(f.height_cm),
            dpi: f.dpi.to_string(),
            bleed_mm: decimal(f.bleed_mm),
            safe_mm: decimal(f.safe_mm),
            spine_mm: f.spine_mm.map(decimal).unwrap_or_default(),
            starts_single: f.starts_single,
            ends_single: f.ends_single,
        }
    }
}

impl SpecText {
    /// Os campos como medidas; `Err` com a mensagem do primeiro campo errado.
    pub fn parse(&self) -> Result<SpecFields, String> {
        let field = |text: &str, name: &str| {
            parse_decimal(text).ok_or_else(|| format!("{name}: digite um número."))
        };
        Ok(SpecFields {
            width_cm: field(&self.width_cm, "Largura")?,
            height_cm: field(&self.height_cm, "Altura")?,
            dpi: self
                .dpi
                .trim()
                .parse()
                .map_err(|_| "DPI: digite um número inteiro.".to_owned())?,
            bleed_mm: field(&self.bleed_mm, "Sangria")?,
            bleed_top_mm: None,
            bleed_spine_mm: None,
            safe_mm: field(&self.safe_mm, "Zona segura")?,
            safe_top_mm: None,
            spine_mm: if self.spine_mm.trim().is_empty() {
                None
            } else {
                Some(field(&self.spine_mm, "Lombada")?)
            },
            starts_single: self.starts_single,
            ends_single: self.ends_single,
        })
    }
}

/// A faixa de páginas como o fotógrafo digitou ("" = sem limite; spec PR).
pub fn parse_limits(min: &str, max: &str) -> Result<PageLimits, String> {
    let field = |text: &str, name: &str| -> Result<Option<u32>, String> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        match text.parse::<u32>() {
            Ok(n) if n > 0 => Ok(Some(n)),
            _ => Err(format!("{name}: digite um número inteiro maior que zero.")),
        }
    };
    let limits = PageLimits {
        min: field(min, "Páginas mínimas")?,
        max: field(max, "Páginas máximas")?,
    };
    match (limits.min, limits.max) {
        (Some(a), Some(b)) if a > b => Err("O mínimo de páginas passa do máximo.".to_owned()),
        _ => Ok(limits),
    }
}

/// Os dois campos da faixa de páginas ("" = sem limite).
pub fn limits_text(limits: PageLimits) -> (String, String) {
    let text = |v: Option<u32>| v.map(|n| n.to_string()).unwrap_or_default();
    (text(limits.min), text(limits.max))
}

/// "Dê um nome ao álbum." quando o nome está vazio.
pub fn album_name_error(name: &str) -> Option<&'static str> {
    name.trim().is_empty().then_some("Dê um nome ao álbum.")
}

impl Editor {
    /// A dica da barra de baixo, conforme o que está selecionado.
    pub fn hint(&self) -> &'static str {
        if self.crop_mode().is_some() {
            "Roda do mouse ou + e − para o zoom · arraste para mover · 0 encaixa · Esc sai"
        } else if self.selected_frame().is_some() {
            "Delete tira a foto · C ou duplo clique recorta · arraste para outro quadro para trocar"
        } else if self.on_cover() {
            // Na capa não há troca de layout (spec CA, regra 5).
            "Arraste uma foto para Frente, Verso ou Lombada · T cria um texto · → vai para as lâminas"
        } else if self.organizing() && self.selected_frames().is_empty() {
            // No modo, a tarefa é escolher os quadros (spec DG, regra 6).
            "Clique num quadro · Ctrl+clique soma · arraste no vazio para selecionar vários · Esc volta"
        } else if self.session().is_some() {
            // Sem nada selecionado, o caminho das tarefas mais comuns (auditoria da Fase 4, 5 e 18).
            "Arraste uma foto da faixa de baixo para um quadro · ↑ ↓ trocam o layout · T cria um texto"
        } else {
            ""
        }
    }
}

/// "30 × 30 cm por página" e "300 DPI · sangria 3 mm".
pub fn album_info(project: &Project) -> String {
    let spec = &project.spec;
    format!(
        "{} × {} cm por página\n{} DPI · sangria {} mm",
        decimal(spec.page_width().get() / 10.0),
        decimal(spec.page_height().get() / 10.0),
        spec.dpi().get(),
        decimal(spec.bleed().outer.get())
    )
}

/// Nome, medidas, captura, câmera e quantas vezes a foto foi usada.
pub fn photo_text(info: &PhotoInfo) -> String {
    let uses = match info.uses {
        0 => "Não usada".to_owned(),
        1 => "Usada 1 vez".to_owned(),
        n => format!("Usada {n} vezes"),
    };
    format!(
        "{}\n{} × {} px\nCaptura: {}\nCâmera: {}\n{uses}",
        info.name, info.width, info.height, info.taken_at, info.camera
    )
}
