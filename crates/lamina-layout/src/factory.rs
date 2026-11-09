//! Modelos de fábrica (spec RM, beta 9.1): desenhos calculados pelo motor, embutidos, por
//! quantidade de fotos (misturas de orientação) nas duas configurações de página. A biblioteca é
//! determinística — a mesma em qualquer máquina — e não vai para `modelos.json`: as marcas
//! (favorito, oculto) dos de fábrica ficam nas preferências de layout, com chave `fabrica:`.
//!
//! Geração: para cada mistura (contagem de horizontais, verticais e quadradas, sem ordem) com
//! até 12 fotos no total, nas duas configurações de página (lâmina aberta; ponta com página
//! simples), as listas do motor com `window: true` (sem variantes deslocadas, ordenadas pelo
//! preenchimento): com até 2 fotos, todos os desenho; de 3 a 12, os 12 primeiros com o rodízio.
//! A spec canônica é 30 × 30 cm, modo lâmina, zona segura de 10 mm; noutra proporção o modelo
//! aplica pelas frações (regra 7 da MD).

use lamina_core::{ActivePages, AlbumSpec, DesignMode, Dpi, Mm};

use crate::layouts::{LayoutOptions, SpreadModel, layout_options, model_from_candidate};

/// A spec canônica da fábrica (spec RM, regra 5).
pub fn canonical_spec() -> Option<AlbumSpec> {
    let dpi = Dpi::new(300).ok()?;
    let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), dpi).ok()?;
    let spec = spec.with_safe_margin(Mm(10.0)).ok()?;
    Some(spec.with_mode(DesignMode::Spread))
}

/// Os modelos de fábrica para uma mistura de fotos: `h` horizontais, `v` verticais, `q` quadradas,
/// com as duas páginas (lâmina aberta). Com `left = false`, ponta com página simples.
pub fn factory_models(h: usize, v: usize, q: usize, both: bool) -> Vec<SpreadModel> {
    let aspects: Vec<f64> = std::iter::repeat_n(1.5, h)
        .chain(std::iter::repeat_n(2.0 / 3.0, v))
        .chain(std::iter::repeat_n(1.0, q))
        .collect();
    let total = aspects.len();
    if total == 0 || total > 12 {
        return Vec::new();
    }
    let active = if both {
        ActivePages {
            left: true,
            right: true,
        }
    } else {
        ActivePages {
            left: false,
            right: true,
        }
    };
    let options = LayoutOptions {
        window: true,
        ..LayoutOptions::default()
    };
    let Some(spec) = canonical_spec() else {
        return Vec::new();
    };
    let Ok(lists) = layout_options(&spec, active, &aspects, &options) else {
        return Vec::new();
    };
    let keep = if total <= 2 { usize::MAX } else { 12 };
    lists
        .in_order
        .into_iter()
        .take(keep)
        .enumerate()
        .filter_map(|(i, c)| {
            model_from_candidate(&c, &aspects, &factory_name(h, v, q, i, both), both)
        })
        .collect()
}

/// O nome de fábrica: "2 fotos · lado a lado", com número quando há mais de um por mistura.
fn factory_name(h: usize, v: usize, q: usize, i: usize, both: bool) -> String {
    let total = h + v + q;
    let mix = if v == 0 && q == 0 {
        String::new()
    } else {
        format!(" · {h}H {v}V {q}Q")
    };
    let pages = if both { "" } else { " · página única" };
    if i == 0 {
        format!("{total} fotos{mix}{pages}")
    } else {
        format!("{total} fotos{mix}{pages} · {i}")
    }
}

/// A biblioteca de fábrica inteira: todas as misturas de 1 a 12 fotos nas duas configurações de
/// página (para a montagem automática da janela, que escolhe por lâmina; o casamento filtra).
/// Gerada uma vez e guardada: a montagem não pode pagar a geração a cada álbum (br_b16).
pub fn factory_models_all() -> Vec<SpreadModel> {
    static ALL: std::sync::OnceLock<Vec<SpreadModel>> = std::sync::OnceLock::new();
    ALL.get_or_init(build_all).clone()
}

fn build_all() -> Vec<SpreadModel> {
    let mut out = Vec::new();
    for total in 1..=12usize {
        for h in 0..=total {
            for v in 0..=(total - h) {
                let q = total - h - v;
                for both in [true, false] {
                    out.extend(factory_models(h, v, q, both));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rm_b10_biblioteca_deterministica() {
        let a = factory_models(2, 1, 0, true);
        let b = factory_models(2, 1, 0, true);
        assert!(!a.is_empty(), "há modelos de fábrica");
        assert_eq!(a, b, "a biblioteca é a mesma em qualquer execução");
    }

    #[test]
    fn rm_b5_o_de_fabrica_aparece_para_a_mistura() {
        // 3 fotos H H V: os de fábrica da mistura (3, 0, 0 não é a mistura certa)…
        let models = factory_models(2, 1, 0, true);
        assert!(!models.is_empty());
        for m in &models {
            assert_eq!(m.slots.len(), 3, "cada modelo tem 3 quadros");
        }
    }

    #[test]
    fn rm_b19_sem_fabrica_acima_de_12() {
        assert!(factory_models(12, 1, 0, true).is_empty());
        assert!(factory_models(0, 0, 0, true).is_empty());
    }
}
