//! O projeto inteiro e o formato JSON versionado.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AlbumLook, Border, ExportPrefs, Frame, FrameContent, FrameId, Mounting, PageLimits, Photo,
    PhotoId, Spread, SpreadId, migration,
};
use crate::{ActivePages, AlbumSpec, CoreError, Mm, Rect, RelRect};

/// Versão atual do formato do arquivo de projeto. Suba ao mudar a estrutura e
/// escreva a migração correspondente em [`Project::from_json`].
pub const CURRENT_SCHEMA_VERSION: u32 = 8;

/// Espaço entre fotos padrão: 8 pontos tipográficos (spec TP; spec AP, regra 2).
pub const DEFAULT_GAP: Mm = Mm(8.0 * 25.4 / 72.0);

fn default_gap() -> Mm {
    DEFAULT_GAP
}

/// O projeto inteiro: o que é salvo em disco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub spec: AlbumSpec,
    /// Espaço entre fotos do álbum (as lâminas podem ter o seu; spec AP).
    #[serde(default = "default_gap")]
    pub gap: Mm,
    pub photos: Vec<Photo>,
    pub spreads: Vec<Spread>,
    /// A capa fotográfica, fora das lâminas e da numeração (spec CA; formato 8).
    #[serde(rename = "capa", default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<super::Cover>,
    /// Mínimo e máximo de páginas do laboratório (spec PR; opcional no arquivo).
    #[serde(default, skip_serializing_if = "PageLimits::is_empty")]
    pub page_limits: PageLimits,
    /// Formato, cor e páginas soltas que o laboratório pede (spec PR; opcional no arquivo).
    #[serde(default, skip_serializing_if = "ExportPrefs::is_default")]
    pub export: ExportPrefs,
    /// Quanto o gerador pode recortar e quais famílias usa (spec NP; formato 4).
    #[serde(rename = "montagem", default)]
    pub mounting: Mounting,
    /// Fundo das lâminas novas, borda e cantos das fotos escolhidos ao criar (spec NP).
    #[serde(rename = "aparencia", default)]
    pub look: AlbumLook,
    /// Próximo identificador livre (compartilhado por fotos, lâminas e quadros).
    pub(super) next_id: u64,
}

impl Project {
    /// Iguais em tudo, menos nas preferências de exportação (spec LV, regra 1): "tem alteração?"
    /// não precisa copiar o álbum para trocar só esse campo. Lista todos os campos de propósito:
    /// um campo novo não compila enquanto não entrar aqui.
    pub fn same_except_export(&self, other: &Project) -> bool {
        let Project {
            schema_version,
            name,
            spec,
            gap,
            photos,
            spreads,
            cover,
            page_limits,
            export: _,
            mounting,
            look,
            next_id,
        } = self;
        *schema_version == other.schema_version
            && *next_id == other.next_id
            && *gap == other.gap
            && name == &other.name
            && spec == &other.spec
            && page_limits == &other.page_limits
            && mounting == &other.mounting
            && look == &other.look
            && spreads == &other.spreads
            && cover == &other.cover
            && photos == &other.photos
    }

    /// As lâminas que mudaram entre `before` e este projeto, se só as lâminas mudaram; `None`
    /// quando a mudança pode valer para o álbum todo (outra especificação, outras fotos, lâminas
    /// a mais ou a menos). A pré-impressão usa isso para conferir só o que mudou (spec LV, regra 2).
    pub fn spreads_changed_since(&self, before: &Project) -> Option<Vec<usize>> {
        if self.spreads.len() != before.spreads.len()
            || self.spec != before.spec
            || self.gap != before.gap
            || self.look != before.look
            || self.photos != before.photos
        {
            return None;
        }
        let pairs = self.spreads.iter().zip(&before.spreads);
        let changed = pairs.enumerate().filter(|(_, (a, b))| a != b);
        Some(changed.map(|(i, _)| i).collect())
    }

    pub fn new(name: impl Into<String>, spec: AlbumSpec) -> Self {
        Project {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: name.into(),
            spec,
            gap: DEFAULT_GAP,
            photos: Vec::new(),
            spreads: Vec::new(),
            cover: None,
            page_limits: PageLimits::default(),
            export: ExportPrefs::default(),
            mounting: Mounting::default(),
            look: AlbumLook::default(),
            next_id: 1,
        }
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_photo(
        &mut self,
        path: impl Into<PathBuf>,
        width_px: u32,
        height_px: u32,
    ) -> PhotoId {
        let id = PhotoId(self.take_id());
        self.photos.push(Photo {
            id,
            path: path.into(),
            width_px,
            height_px,
            fingerprint: None,
            taken_at: None,
            camera: None,
            adjust: super::PhotoAdjust::default(),
        });
        id
    }

    pub fn add_spread(&mut self) -> SpreadId {
        let id = SpreadId(self.take_id());
        self.spreads.push(Spread {
            id,
            background: self.look.background,
            background_right: None,
            gap: None,
            frames: Vec::new(),
            manual: false,
            model: None,
            blank: super::BlankPages::default(),
        });
        id
    }

    /// As páginas da lâmina `index` onde entra foto: as ativas menos as em branco (spec PB,
    /// regra 2). O que é impresso continua com `active_pages`.
    pub fn content_pages(&self, index: usize) -> ActivePages {
        let active = self.spec.edges().active_pages(index, self.spreads.len());
        let blank = self
            .spreads
            .get(index)
            .map_or_else(super::BlankPages::default, |s| s.blank);
        ActivePages {
            left: active.left && !blank.left,
            right: active.right && !blank.right,
        }
    }

    pub fn add_frame(
        &mut self,
        spread: SpreadId,
        rect: RelRect,
        content: FrameContent,
    ) -> Result<FrameId, CoreError> {
        // Um retângulo que veio de fora sem validação (um modelo editado à mão) não pode entrar:
        // o projeto salvaria e não abriria de novo (auditoria do Marco 3, M3).
        rect.validate()?;
        self.check_content(&content)?;
        let id = FrameId(self.take_id());
        let spread = self.spread_mut(spread)?;
        spread.frames.push(Frame {
            id,
            rect,
            content,
            border: Border::default(),
            radius: Mm(0.0),
            opacity: super::Opacity::FULL,
        });
        Ok(id)
    }

    pub fn photo(&self, id: PhotoId) -> Result<&Photo, CoreError> {
        // As fotos entram em ordem de identificador: a busca binária resolve quase sempre, e um
        // arquivo editado à mão, fora de ordem, cai na busca item a item (spec LV).
        let found = self.photos.binary_search_by_key(&id, |p| p.id).ok();
        found
            .and_then(|i| self.photos.get(i))
            .or_else(|| self.photos.iter().find(|p| p.id == id))
            .ok_or(CoreError::UnknownPhoto(id))
    }

    pub fn spread(&self, id: SpreadId) -> Result<&Spread, CoreError> {
        self.spreads
            .iter()
            .find(|s| s.id == id)
            .ok_or(CoreError::UnknownSpread(id))
    }

    pub fn spread_mut(&mut self, id: SpreadId) -> Result<&mut Spread, CoreError> {
        self.spreads
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or(CoreError::UnknownSpread(id))
    }

    /// Espaço entre fotos da lâmina `spread` (o dela ou o do álbum).
    pub fn spread_gap(&self, spread: &Spread) -> Mm {
        spread.gap.unwrap_or(self.gap)
    }

    /// Posição absoluta de um quadro no canvas da lâmina, em mm.
    pub fn frame_rect(&self, frame: &Frame) -> Rect {
        frame.rect.to_absolute(&self.spec.trim_rect())
    }

    /// Resolução efetiva da foto impressa no quadro, em DPI.
    ///
    /// É a base do aviso de resolução: abaixo do DPI da especificação, a foto está sendo
    /// ampliada. Retorna `None` se o quadro não tem foto.
    pub fn effective_dpi(&self, frame: &Frame) -> Result<Option<f64>, CoreError> {
        let FrameContent::Photo { photo, crop } = &frame.content else {
            return Ok(None);
        };
        let photo = self.photo(*photo)?;
        let rect = self.frame_rect(frame);
        // Com zoom 1 a foto cobre o quadro: vale o eixo que precisa de mais ampliação. O giro
        // aproxima a foto pelo fator de cobertura (spec AF, regra 3).
        let (pw, ph) = (f64::from(photo.width_px), f64::from(photo.height_px));
        let dpi_x = pw / rect.width.to_inches();
        let dpi_y = ph / rect.height.to_inches();
        let k = super::turn::cover_factor(
            (pw, ph),
            (rect.width.get(), rect.height.get()),
            photo.adjust.degrees(),
        );
        Ok(Some(dpi_x.min(dpi_y) / crop.zoom / k))
    }

    fn check_content(&self, content: &FrameContent) -> Result<(), CoreError> {
        if let FrameContent::Photo { photo, crop } = content {
            self.photo(*photo)?;
            crop.validate()?;
        }
        Ok(())
    }

    /// Confere todas as invariantes. Chamado ao carregar um arquivo.
    pub fn validate(&self) -> Result<(), CoreError> {
        self.spec.validate()?;
        if let Some(cover) = &self.cover {
            cover.measures.validate()?;
            for frame in &cover.content.frames {
                if !frame.opacity.is_valid() {
                    return Err(CoreError::InvalidCover {
                        field: "opacidade",
                        value: f64::from(frame.opacity.get()),
                    });
                }
                frame.rect.validate()?;
                self.check_content(&frame.content)?;
            }
        }
        if let Some(bad) = self.photos.iter().find(|p| !p.adjust.is_valid()) {
            let name = bad.path.file_name().unwrap_or(bad.path.as_os_str());
            return Err(CoreError::InvalidAdjust {
                photo: name.to_string_lossy().into_owned(),
            });
        }
        for (index, spread) in self.spreads.iter().enumerate() {
            for frame in &spread.frames {
                if !frame.opacity.is_valid() {
                    return Err(CoreError::InvalidOpacity {
                        value: frame.opacity.get(),
                        spread: index + 1,
                    });
                }
                frame.rect.validate()?;
                self.check_content(&frame.content)?;
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, CoreError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Lê um projeto salvo. Versões antigas do formato são migradas; versões mais novas que
    /// este programa são recusadas.
    pub fn from_json(json: &str) -> Result<Self, CoreError> {
        #[derive(Deserialize)]
        struct VersionOnly {
            schema_version: u32,
        }
        let value: serde_json::Value = serde_json::from_str(json)?;
        let VersionOnly { schema_version } = serde_json::from_value(value.clone())?;
        if schema_version > CURRENT_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchema {
                found: schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let value = migration::migrate(value, schema_version)?;
        let project: Project = serde_json::from_value(value)?;
        project.validate()?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Crop, Dpi, Mm};

    fn sample_project() -> (Project, SpreadId, PhotoId) {
        let spec = AlbumSpec::new(Mm::from_cm(30.0), Mm::from_cm(30.0), Dpi::new(300).unwrap())
            .unwrap()
            .with_bleed(Mm(3.0))
            .unwrap();
        let mut project = Project::new("Casamento Ana & Bruno", spec);
        let photo = project.add_photo("fotos/IMG_0001.jpg", 6000, 4000);
        let spread = project.add_spread();
        project
            .add_frame(
                spread,
                RelRect::new(0.0, 0.0, 0.5, 1.0).unwrap(),
                FrameContent::Photo {
                    photo,
                    crop: Crop::default(),
                },
            )
            .unwrap();
        (project, spread, photo)
    }

    #[test]
    fn ida_e_volta_em_json() {
        let (project, _, _) = sample_project();
        let json = project.to_json().unwrap();
        let loaded = Project::from_json(&json).unwrap();
        assert_eq!(loaded, project);
    }

    /// Defeito achado pelo Auto Build (AB-B6): sem `float_roundtrip`, o serde_json lia
    /// 0.21321367521367526 de volta como 0.2132136752136753, e o projeto aberto ficava diferente
    /// do salvo.
    #[test]
    fn ida_e_volta_em_json_preserva_numeros_exatos() {
        let (mut project, spread, photo) = sample_project();
        let rect = RelRect::new(0.213_213_675_213_675_26, 0.1, 0.3, 0.4).unwrap();
        let content = FrameContent::Photo {
            photo,
            crop: Crop::default(),
        };
        project.add_frame(spread, rect, content).unwrap();
        let loaded = Project::from_json(&project.to_json().unwrap()).unwrap();
        assert_eq!(loaded, project);
    }

    #[test]
    fn quadro_relativo_vira_posicao_absoluta() {
        let (project, spread, _) = sample_project();
        let frame = &project.spread(spread).unwrap().frames[0];
        assert_eq!(
            project.frame_rect(frame),
            Rect::new(Mm(3.0), Mm(3.0), Mm(300.0), Mm(300.0))
        );
    }

    #[test]
    fn resolucao_efetiva_de_foto_6000x4000_numa_pagina_de_30cm() {
        let (project, spread, _) = sample_project();
        let frame = &project.spread(spread).unwrap().frames[0];
        // 4000 px de altura em 300 mm (11,81 pol) ≈ 338,7 DPI; o eixo vertical é o limitante.
        let dpi = project.effective_dpi(frame).unwrap().unwrap();
        assert!((dpi - 338.67).abs() < 0.01, "dpi = {dpi}");
    }

    #[test]
    fn zoom_reduz_a_resolucao_efetiva() {
        let (mut project, spread, photo) = sample_project();
        let frame_id = project
            .add_frame(
                spread,
                RelRect::new(0.5, 0.0, 0.5, 1.0).unwrap(),
                FrameContent::Photo {
                    photo,
                    crop: Crop {
                        zoom: 2.0,
                        ..Crop::default()
                    },
                },
            )
            .unwrap();
        let frame = project
            .spread(spread)
            .unwrap()
            .frames
            .iter()
            .find(|f| f.id == frame_id)
            .unwrap();
        let dpi = project.effective_dpi(frame).unwrap().unwrap();
        assert!((dpi - 169.33).abs() < 0.01, "dpi = {dpi}");
    }

    #[test]
    fn quadro_com_foto_inexistente_e_rejeitado() {
        let (mut project, spread, _) = sample_project();
        let result = project.add_frame(
            spread,
            RelRect::full(),
            FrameContent::Photo {
                photo: PhotoId(999),
                crop: Crop::default(),
            },
        );
        assert!(matches!(result, Err(CoreError::UnknownPhoto(_))));
    }

    #[test]
    fn versao_de_formato_mais_nova_e_recusada() {
        let (project, _, _) = sample_project();
        let json = project.to_json().unwrap().replace(
            &format!("\"schema_version\": {CURRENT_SCHEMA_VERSION}"),
            "\"schema_version\": 999",
        );
        assert!(matches!(
            Project::from_json(&json),
            Err(CoreError::UnsupportedSchema { found: 999, .. })
        ));
    }

    #[test]
    fn json_com_quadro_invalido_e_recusado() {
        let (project, _, _) = sample_project();
        let json = project
            .to_json()
            .unwrap()
            .replace("\"width\": 0.5", "\"width\": 1.5");
        assert!(matches!(
            Project::from_json(&json),
            Err(CoreError::InvalidRelRect(_))
        ));
    }
}
