//! Pré-impressão (spec EX, regra 9): avisos antes de exportar, sem impedir a exportação.

use std::fmt;

use lamina_core::{Frame, FrameContent, Mm, PageSide, Project, Rect};

/// Ampliação a partir da qual a foto vira aviso (120%, como na referência).
const MAX_ENLARGEMENT: f64 = 1.2;
/// Distância ao corte abaixo da qual o quadro "encosta" nele, em mm.
const TOUCH_MM: f64 = 0.1;
/// Folga da zona segura: um quadro exatamente na borda dela não é aviso (contas em ponto
/// flutuante passam por um fio).
const SAFE_TOLERANCE_MM: f64 = 0.01;

/// Um aviso da pré-impressão, numa lâmina (contada a partir de 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub spread: usize,
    /// O quadro do aviso, na ordem dos quadros da lâmina (começando em 0), para "Ir para a
    /// lâmina" selecioná-lo (spec EI, regra 2).
    pub frame: usize,
    pub kind: WarningKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarningKind {
    /// A foto precisa crescer mais de 120% para encher o quadro.
    Enlarged {
        photo: String,
        percent: u32,
        effective_dpi: u32,
    },
    /// Quadro sem foto.
    EmptyFrame,
    /// Quadro fora da zona segura sem encostar no corte, a `distance` dele.
    NearCut { photo: String, distance: Mm },
    /// O texto passa da altura do quadro (spec TX, regra 3).
    TextOverflow { text: String },
    /// O texto sai da zona segura ou atravessa a dobra.
    TextOutsideSafe { text: String },
    /// A fonte do texto não tem o caractere.
    MissingChar { char: char },
    /// A fonte do projeto não é do Lamina nem está neste computador; o texto sai na padrão (spec FS).
    UnknownFont { font: String },
    /// O texto da capa entra na dobra (spec CA, regra 14).
    TextOnWrap,
    /// A lombada foi fixada para outro número de páginas.
    SpineOutdated { fixed_for: usize, pages: usize },
    /// A face da capa é menor que a página.
    SmallFace { face: Mm, page: Mm },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A capa fica fora da numeração: é a "lâmina 0" dos avisos (spec CA, regra 14).
        let at = if self.spread == 0 {
            "Capa".to_owned()
        } else {
            format!("Lâmina {}", self.spread)
        };
        match &self.kind {
            WarningKind::Enlarged {
                photo,
                percent,
                effective_dpi,
            } => write!(
                f,
                "{at}: a foto {photo} está ampliada {percent}% ({effective_dpi} DPI efetivo)."
            ),
            WarningKind::EmptyFrame => write!(f, "{at}: quadro vazio."),
            WarningKind::NearCut { photo, distance } => write!(
                f,
                "{at}: a foto {photo} está fora da zona segura, a {} mm do corte.",
                millimeters(distance.get())
            ),
            WarningKind::TextOverflow { text } => {
                write!(f, "{at}: o texto \"{text}\" não cabe no quadro.")
            }
            WarningKind::TextOutsideSafe { text } => {
                write!(f, "{at}: o texto \"{text}\" está fora da zona segura.")
            }
            WarningKind::MissingChar { char } => {
                write!(f, "{at}: a fonte não tem o caractere \"{char}\".")
            }
            WarningKind::UnknownFont { font } => write!(
                f,
                "{at}: a fonte \"{font}\" não está neste computador; o texto usa {}.",
                lamina_core::DEFAULT_FONT
            ),
            WarningKind::TextOnWrap => write!(f, "{at}: texto sobre a dobra."),
            WarningKind::SpineOutdated { fixed_for, pages } => write!(
                f,
                "{at}: o álbum tinha {fixed_for} páginas quando a lombada foi fixada; agora tem {pages}."
            ),
            WarningKind::SmallFace { face, page } => write!(
                f,
                "{at}: a face ({} cm) é menor que a página ({} cm).",
                centimeters(face.get()),
                centimeters(page.get())
            ),
        }
    }
}

/// "29,0", sempre com uma casa.
fn centimeters(mm: f64) -> String {
    format!("{:.1}", mm / 10.0).replace('.', ",")
}

/// "2" para 2,0 e "1,5" para 1,5.
fn millimeters(mm: f64) -> String {
    let rounded = (mm * 10.0).round() / 10.0;
    if (rounded - rounded.round()).abs() < 1e-9 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}").replace('.', ",")
    }
}

/// Avisos das lâminas `spreads` (começando em 0), na ordem do álbum e dos quadros.
pub fn preflight(project: &Project, spreads: &[usize]) -> Vec<Warning> {
    let mut out = Vec::new();
    for &i in spreads {
        let Some(spread) = project.spreads.get(i) else {
            continue;
        };
        for (f, frame) in spread.frames.iter().enumerate() {
            out.extend(check(project, frame).into_iter().map(|kind| Warning {
                spread: i + 1,
                frame: f,
                kind,
            }));
        }
    }
    out
}

pub(crate) fn check(project: &Project, frame: &Frame) -> Vec<WarningKind> {
    let photo = match &frame.content {
        FrameContent::Photo { photo, .. } => photo,
        FrameContent::Text(text) => {
            return crate::preflight_text::text_warnings(project, text, project.frame_rect(frame));
        }
        FrameContent::Empty => return vec![WarningKind::EmptyFrame],
    };
    // Foto que não existe no projeto: a validação do projeto já recusa isso ao abrir.
    let Ok(photo) = project.photo(*photo) else {
        return Vec::new();
    };
    let name = photo
        .path
        .file_name()
        .unwrap_or(photo.path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut out = Vec::new();
    let target = f64::from(project.spec.dpi().get());
    if let Ok(Some(dpi)) = project.effective_dpi(frame)
        && dpi > 0.0
        && target / dpi > MAX_ENLARGEMENT
    {
        out.push(WarningKind::Enlarged {
            photo: name.clone(),
            percent: whole(target / dpi * 100.0),
            effective_dpi: whole(dpi),
        });
    }
    if let Some(mm) = near_cut(project, project.frame_rect(frame)) {
        out.push(WarningKind::NearCut {
            photo: name,
            distance: Mm(mm),
        });
    }
    out
}

/// Distância até o corte, se o quadro sai da zona segura pela borda externa, de cima ou de
/// baixo sem encostar no corte. A margem da lombada não conta.
fn near_cut(project: &Project, rect: Rect) -> Option<f64> {
    let spec = &project.spec;
    let trim = spec.trim_rect();
    let (left, right) = (
        spec.safe_rect(PageSide::Left),
        spec.safe_rect(PageSide::Right),
    );
    // (distância até o corte, saiu da zona segura) para cada borda que importa.
    let sides = [
        (
            rect.x.get() - trim.x.get(),
            rect.x.get() < left.x.get() - SAFE_TOLERANCE_MM,
        ),
        (
            trim.right().get() - rect.right().get(),
            rect.right().get() > right.right().get() + SAFE_TOLERANCE_MM,
        ),
        (
            rect.y.get() - trim.y.get(),
            rect.y.get() < left.y.get() - SAFE_TOLERANCE_MM,
        ),
        (
            trim.bottom().get() - rect.bottom().get(),
            rect.bottom().get() > left.bottom().get() + SAFE_TOLERANCE_MM,
        ),
    ];
    sides
        .iter()
        .filter(|(distance, outside)| *outside && *distance > TOUCH_MM)
        .map(|(distance, _)| *distance)
        .reduce(f64::min)
}

fn whole(value: f64) -> u32 {
    // Arredondado e preso entre 0 e o maior u32, então o cast não trunca.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = value.round().clamp(0.0, f64::from(u32::MAX)) as u32;
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milimetros_sem_casas_quando_inteiro() {
        assert_eq!(millimeters(2.0), "2");
        assert_eq!(millimeters(1.54), "1,5");
    }
}
