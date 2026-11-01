//! A aba Quadro (spec TX, seção 3): as medidas do quadro selecionado na unidade da tela (spec NP,
//! regra 3; a pendência foi para a EG), contadas do canto do corte, e os campos do texto. Ler o que o fotógrafo digitou também mora aqui (regra 12).

use lamina_core::{LINE_SPACING, Mm, PageSide, Project, Rect, Rgb, TEXT_SIZE_PT, TextBox};
use lamina_render::font_styles;

use crate::font_picker::MISSING;

use crate::Editor;
use crate::hand::frame_mm;
use crate::new_project::Unit;

/// Mensagem de medida inválida.
pub const BAD_NUMBER: &str = "Use um número, como 12,5.";

/// O que a aba mostra.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameForm {
    /// X, Y, largura e altura na unidade da tela, com até duas casas e vírgula.
    pub x: String,
    pub y: String,
    pub width: String,
    pub height: String,
    /// "mm", "cm" ou "pol".
    pub unit: &'static str,
    pub text: Option<TextForm>,
}

/// Os campos de um quadro de texto.
#[derive(Debug, Clone, PartialEq)]
pub struct TextForm {
    pub text: String,
    /// O nome guardado no álbum.
    pub font: String,
    /// O que o campo Fonte mostra: o nome, com "(não está neste computador)" quando falta
    /// (spec FS, regra 5).
    pub font_label: String,
    pub size: String,
    pub bold: bool,
    pub italic: bool,
    /// A família tem negrito e itálico (senão, a caixa fica desligada).
    pub can_bold: bool,
    pub can_italic: bool,
    pub color: Rgb,
    pub align: usize,
    pub valign: usize,
    /// O giro do texto: 0 = deitado, 1 = para cima, 2 = para baixo (spec CA, regra 9).
    pub rotation: usize,
    pub spacing: String,
}

/// Número com uma casa e vírgula: 12,5; inteiro sai sem casa.
pub fn decimal(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}").replace('.', ",")
    }
}

/// Lê um número com vírgula ou ponto.
pub fn parse_number(s: &str) -> Option<f64> {
    s.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn text_form(t: &TextBox) -> TextForm {
    let styles = font_styles(&t.font);
    let font_label = match styles {
        Some(_) => t.font.clone(),
        None => format!("{}{MISSING}", t.font),
    };
    TextForm {
        text: t.text.clone(),
        font: t.font.clone(),
        font_label,
        size: decimal(t.size_pt),
        bold: t.bold,
        italic: t.italic,
        can_bold: styles.is_some_and(|f| f.bold),
        can_italic: styles.is_some_and(|f| f.italic),
        color: t.color,
        align: t.align as usize,
        valign: t.valign as usize,
        rotation: match t.rotation {
            lamina_core::TextRotation::Zero => 0,
            lamina_core::TextRotation::Up => 1,
            lamina_core::TextRotation::Down => 2,
        },
        spacing: decimal(t.line_spacing),
    }
}

impl Editor {
    /// A aba Quadro do quadro selecionado, se houver.
    pub fn frame_form(&self) -> Option<FrameForm> {
        let s = self.session()?;
        let frame = self.selected_frame()?;
        let r = frame_mm(s.project(), s.current(), frame)?;
        let trim = s.project().spec.trim_rect();
        let unit = self.strips().unit;
        Some(FrameForm {
            x: unit.show(r.x.get() - trim.x.get()),
            y: unit.show(r.y.get() - trim.y.get()),
            width: unit.show(r.width.get()),
            height: unit.show(r.height.get()),
            unit: unit.label(),
            text: self.selected_text().map(text_form),
        })
    }
}

/// As medidas digitadas na unidade `unit` (X, Y a partir do corte; largura, altura) → retângulo
/// da lâmina, em mm. X e Y podem ser negativos (o corte prende depois).
pub fn parse_frame(project: &Project, fields: [&str; 4], unit: Unit) -> Result<Rect, &'static str> {
    let mut v = [0.0; 4];
    for (slot, s) in v.iter_mut().zip(fields) {
        *slot = parse_number(s).ok_or(BAD_NUMBER)? * unit.mm_per_unit();
    }
    let trim = project.spec.trim_rect();
    Ok(Rect::new(
        Mm(trim.x.get() + v[0]),
        Mm(trim.y.get() + v[1]),
        Mm(v[2]),
        Mm(v[3]),
    ))
}

/// O tamanho do texto digitado, em pontos.
pub fn parse_size(s: &str) -> Result<f64, String> {
    parse_number(s)
        .filter(|v| (TEXT_SIZE_PT.0..=TEXT_SIZE_PT.1).contains(v))
        .ok_or_else(|| {
            format!(
                "O tamanho vai de {} a {} pt.",
                decimal(TEXT_SIZE_PT.0),
                decimal(TEXT_SIZE_PT.1)
            )
        })
}

/// A entrelinha digitada.
pub fn parse_spacing(s: &str) -> Result<f64, String> {
    parse_number(s)
        .filter(|v| (LINE_SPACING.0..=LINE_SPACING.1).contains(v))
        .ok_or_else(|| "A entrelinha vai de 0,8 a 3,0.".to_owned())
}

/// A página sob o ponto `fx` (fração da largura da lâmina com a sangria), para o texto novo.
pub fn page_at(project: &Project, fx: f64) -> Option<PageSide> {
    if !(0.0..=1.0).contains(&fx) {
        return None;
    }
    let canvas = project.spec.canvas_rect();
    let x = canvas.x.get() + fx * canvas.width.get();
    let spine = project.spec.page_rect(PageSide::Left).right().get();
    Some(if x < spine {
        PageSide::Left
    } else {
        PageSide::Right
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeros_com_virgula_ou_ponto() {
        assert_eq!(parse_number("12,5"), Some(12.5));
        assert_eq!(parse_number(" 3.25 "), Some(3.25));
        assert_eq!(parse_number("doze"), None);
        assert_eq!(decimal(12.0), "12");
        assert_eq!(decimal(1.25), "1,3");
    }

    #[test]
    fn tamanho_e_entrelinha_fora_da_faixa() {
        assert_eq!(parse_size("24"), Ok(24.0));
        assert_eq!(
            parse_size("500"),
            Err("O tamanho vai de 4 a 300 pt.".to_owned())
        );
        assert_eq!(parse_spacing("1,5"), Ok(1.5));
        assert_eq!(
            parse_spacing("5"),
            Err("A entrelinha vai de 0,8 a 3,0.".to_owned())
        );
    }
}
