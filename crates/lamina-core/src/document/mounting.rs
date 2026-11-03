//! A montagem do álbum (spec NP, regra 6): quanto o gerador pode recortar as fotos e quais
//! famílias opcionais ele usa. O lay-flat fica no `DesignMode` da especificação.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Border, Rgb};
use crate::Mm;

/// Quanto o gerador pode recortar as fotos para encher a lâmina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum CropLevel {
    /// Fotos inteiras: nenhuma família que recorta.
    #[serde(rename = "inteiras")]
    Whole,
    /// Recorte leve: até 15% de cada foto.
    #[serde(rename = "leve")]
    Light,
    /// Livre: como o gerador sempre fez (até 35% na foto atravessada e na página sangrada).
    #[default]
    #[serde(rename = "livre")]
    Free,
}

/// A montagem do álbum, guardada no projeto: vale para a montagem automática, o painel de layouts e
/// "Montar lâmina", hoje e depois de reabrir o álbum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Mounting {
    #[serde(rename = "recorte")]
    pub crop: CropLevel,
    /// Uma foto cobrindo uma página até o corte.
    #[serde(rename = "pagina_sangrada")]
    pub bleed_page: bool,
    #[serde(rename = "mosaicos")]
    pub mosaics: bool,
}

impl Default for Mounting {
    /// O comportamento de sempre: livre, com página sangrada e mosaicos.
    fn default() -> Self {
        Mounting {
            crop: CropLevel::Free,
            bleed_page: true,
            mosaics: true,
        }
    }
}

impl Mounting {
    /// O maior recorte aceito numa foto (fração do lado que sai). `None` = o de cada família.
    pub fn max_crop(self) -> Option<f64> {
        match self.crop {
            CropLevel::Whole => Some(0.0),
            CropLevel::Light => Some(LIGHT_MAX_CROP),
            CropLevel::Free => None,
        }
    }
}

/// A aparência escolhida ao criar o álbum (spec NP, etapa 2): o fundo das lâminas novas, a borda
/// e os cantos das fotos. O espaço entre fotos fica no `gap` do projeto. Guardada no projeto para
/// um álbum que começou sem fotos usar quando elas chegarem.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AlbumLook {
    #[serde(rename = "fundo")]
    pub background: Rgb,
    #[serde(rename = "borda")]
    pub border: Border,
    #[serde(rename = "cantos")]
    pub radius: Mm,
}

impl Default for AlbumLook {
    /// Fundo branco, sem borda, cantos retos: o de sempre.
    fn default() -> Self {
        AlbumLook {
            background: Rgb::WHITE,
            border: Border::default(),
            radius: Mm(0.0),
        }
    }
}

/// O recorte leve: até 15% de cada foto.
pub const LIGHT_MAX_CROP: f64 = 0.15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn montagem_padrao_e_a_de_sempre_e_grava_em_portugues() {
        let m = Mounting::default();
        assert_eq!(m.max_crop(), None);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(
            json,
            r#"{"recorte":"livre","pagina_sangrada":true,"mosaicos":true}"#
        );
        let whole = Mounting {
            crop: CropLevel::Whole,
            ..m
        };
        assert_eq!(whole.max_crop(), Some(0.0));
        let light = Mounting {
            crop: CropLevel::Light,
            ..m
        };
        assert_eq!(light.max_crop(), Some(0.15));
    }
}
