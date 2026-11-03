//! As imagens já convertidas para o Slint: miniaturas (faixa e navegador) e imagens de tela
//! (lâmina grande). Na memória ficam só as imagens de tela da lâmina atual e das vizinhas (spec
//! JP, regra 6), lidas por um trabalhador; a lâmina aparece com as miniaturas e cada foto nítida
//! entra quando chega (spec MV, regra 1). Quem decide o que pedir é o `ScreenPlan` do editor.
//! As miniaturas também vêm de um trabalhador, pedidas por lista, e só um número limitado das
//! que nenhuma lista pede fica na memória (spec TR, regras 6 a 8; `ThumbPlan` do editor).

use std::collections::HashMap;

use lamina_core::{FrameContent, PhotoId, Project};
use lamina_editor::{Arrival, Cache, ScreenPlan, ThumbList, ThumbPlan};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::loader::Loader;
use crate::mailbox::Mailbox;
use crate::state::App;
use crate::thumb_loader::{ThumbLoader, ThumbPixels};

pub type Pixels = SharedPixelBuffer<Rgba8Pixel>;

/// Quantas fotos da seleção o painel de layouts pode mostrar numa prévia (uma lâmina cheia).
const PANEL_PHOTOS: usize = 24;

pub struct Images {
    /// Sobe a cada álbum novo: fotos de um álbum anterior que chegarem são ignoradas.
    epoch: u64,
    /// Sobe a cada álbum novo e a cada troca do lado das miniaturas: as que chegarem lidas no
    /// lado antigo são ignoradas (spec PF, seção 13).
    thumb_epoch: u64,
    /// O lado das miniaturas lidas agora (0 = o do disco).
    thumb_side: u32,
    plan: ScreenPlan,
    loader: Loader,
    screens: HashMap<PhotoId, Image>,
    /// Chegaram durante um deslize e entram quando ele termina (MV-B26).
    held: HashMap<PhotoId, Image>,
    thumbs: HashMap<PhotoId, Image>,
    thumb_plan: ThumbPlan,
    thumb_loader: ThumbLoader,
    /// Miniaturas que chegaram desde a última atualização das listas.
    fresh: Vec<PhotoId>,
    /// A foto original do quadro em modo recorte (spec RC, regra 9).
    original: Option<(PhotoId, Image)>,
    /// Chegou foto nítida nova desde o último desenho da lâmina.
    pub(crate) arrived: bool,
    /// A lâmina está deslizando: as fotos nítidas que chegam esperam o fim (MV-B26).
    sliding: bool,
    /// Com o rolo (spec RL, regra 9): as fotos nítidas da lâmina viva entram uma por quadro depois
    /// de ir para ela; até lá, a miniatura (e a imagem pronta da lâmina por baixo).
    pub(crate) gated: bool,
    revealed: std::collections::HashSet<PhotoId>,
}

impl Images {
    pub fn new(cache: &Cache, mailbox: &Mailbox) -> Self {
        Images {
            epoch: 0,
            thumb_epoch: 0,
            thumb_side: 0,
            plan: ScreenPlan::default(),
            loader: Loader::new(cache, mailbox),
            screens: HashMap::new(),
            held: HashMap::new(),
            thumbs: HashMap::new(),
            thumb_plan: ThumbPlan::default(),
            thumb_loader: ThumbLoader::new(cache, mailbox),
            fresh: Vec::new(),
            original: None,
            arrived: false,
            sliding: false,
            gated: false,
            revealed: std::collections::HashSet::new(),
        }
    }

    /// Esquece tudo (álbum novo).
    pub fn clear(&mut self) {
        self.epoch += 1;
        self.thumb_epoch += 1;
        self.plan = ScreenPlan::default();
        self.screens.clear();
        self.held.clear();
        self.thumbs.clear();
        self.thumb_plan.forget();
        self.fresh.clear();
        self.original = None;
    }

    /// Lê as miniaturas com `side` px no lado maior (o tamanho da peça da grade; spec PF, seção
    /// 13). Trocou: as da memória saem e as listas pedem de novo (`true`).
    pub fn set_thumb_side(&mut self, side: u32) -> bool {
        if side == self.thumb_side {
            return false;
        }
        self.thumb_side = side;
        self.thumb_epoch += 1;
        self.thumb_loader.set_side(side);
        self.thumbs.clear();
        self.thumb_plan.forget();
        self.fresh.clear();
        true
    }

    /// O cache ganhou imagens: as fotos que não tinham imagem de tela ou miniatura podem ser
    /// pedidas de novo.
    pub fn recheck(&mut self, project: &Project) {
        self.plan.recheck();
        let again = self.thumb_plan.recheck();
        self.ask_thumbs(project, &again);
    }

    /// A miniatura da foto, se já chegou (a lista pede antes, por [`Images::thumbs_for`]).
    pub fn thumb(&self, id: PhotoId) -> Option<Image> {
        self.thumbs.get(&id).cloned()
    }

    /// A lista `list` mostra agora as fotos `ids`: as que faltam vão para o trabalhador e as que
    /// passaram do limite saem (spec TR, regras 6 e 7). Devolve as que já chegaram.
    pub fn thumbs_for(
        &mut self,
        list: ThumbList,
        project: &Project,
        ids: &[PhotoId],
    ) -> HashMap<PhotoId, Image> {
        let orders = self.thumb_plan.want(list, ids);
        for id in &orders.drop {
            self.thumbs.remove(id);
        }
        // Pedidas que ninguém mais mostra saem da fila (rolar rápido; spec PF, regra 14).
        self.thumb_loader.cancel(&orders.cancel);
        self.ask_thumbs(project, &orders.load);
        ids.iter()
            .filter_map(|id| Some((*id, self.thumbs.get(id)?.clone())))
            .collect()
    }

    /// As miniaturas das fotos das lâminas e da capa (faixa, linha do tempo e lâmina) e de `extra`
    /// (a seleção, para as prévias do painel de layouts; até uma lâmina cheia). O painel de fotos
    /// pede as dele à parte, só perto da tela (spec PF, regra 14).
    pub fn album_thumbs(
        &mut self,
        project: &Project,
        extra: &[PhotoId],
    ) -> HashMap<PhotoId, Image> {
        let mut ids: Vec<PhotoId> = project
            .all_content()
            .flat_map(|s| &s.frames)
            .filter_map(|f| match &f.content {
                FrameContent::Photo { photo, .. } => Some(*photo),
                _ => None,
            })
            .collect();
        ids.extend(extra.iter().take(PANEL_PHOTOS));
        ids.sort_unstable();
        ids.dedup();
        self.thumbs_for(ThumbList::Album, project, &ids)
    }

    fn ask_thumbs(&self, project: &Project, ids: &[PhotoId]) {
        let items = ids
            .iter()
            .filter_map(|id| Some((*id, project.photo(*id).ok()?.clone())))
            .collect();
        self.thumb_loader.ask(self.thumb_epoch, items);
    }

    /// Uma miniatura chegou do trabalhador (`None`: não há no cache).
    pub fn thumb_arrived(&mut self, epoch: u64, id: PhotoId, pixels: Option<ThumbPixels>) {
        if epoch != self.thumb_epoch {
            return;
        }
        let (keep, drop) = self.thumb_plan.arrived(id, pixels.is_some());
        for gone in &drop {
            self.thumbs.remove(gone);
        }
        if let (true, Some(pixels)) = (keep, pixels) {
            self.thumbs.insert(id, Image::from_rgb8(pixels));
            self.fresh.push(id);
        }
    }

    /// Todas as miniaturas que já chegaram.
    pub fn thumbs_map(&self) -> &HashMap<PhotoId, Image> {
        &self.thumbs
    }

    /// As miniaturas que chegaram desde a última vez.
    pub fn take_fresh(&mut self) -> Vec<PhotoId> {
        std::mem::take(&mut self.fresh)
    }

    /// As fotos de tela já foram todas lidas (a filmagem espera por isto).
    #[cfg(any(test, feature = "filmagem"))]
    pub fn screens_idle(&self) -> bool {
        self.loader.idle()
    }

    /// Quantas miniaturas foram pedidas e ainda não chegaram.
    pub fn thumbs_pending(&self) -> usize {
        self.thumb_plan.pending()
    }

    /// As imagens das fotos da lâmina `index`: a de tela, ou a miniatura enquanto ela não chega.
    /// Pede ao trabalhador as que faltam (a lâmina atual primeiro) e solta as que saíram.
    pub fn spread(&mut self, project: &Project, index: usize) -> HashMap<PhotoId, Image> {
        let asked = self.plan.go(project, index);
        for id in &asked.release {
            self.screens.remove(id);
            self.held.remove(id);
        }
        if !asked.photos.is_empty() {
            let items = asked
                .photos
                .iter()
                .filter_map(|id| Some((*id, project.photo(*id).ok()?.clone())))
                .collect();
            self.loader.ask(self.epoch, index, items);
        }
        let mut images = HashMap::new();
        for id in photos_of(project, index) {
            let original = self
                .original
                .as_ref()
                .filter(|(o, _)| *o == id)
                .map(|(_, img)| img.clone());
            let img = original
                .or_else(|| self.sharp(id).cloned())
                .or_else(|| self.thumb(id));
            if let Some(img) = img {
                images.insert(id, img);
            }
        }
        images
    }

    /// Uma foto de tela chegou do trabalhador (`None`: não há imagem de tela no cache).
    pub fn screen_arrived(&mut self, epoch: u64, id: PhotoId, pixels: Option<Pixels>) {
        if epoch != self.epoch {
            return;
        }
        let Some(pixels) = pixels else {
            self.plan.failed(id);
            return;
        };
        match self.plan.arrived(id, self.sliding) {
            Arrival::Show => {
                self.screens.insert(id, Image::from_rgba8(pixels));
                self.arrived = true;
            }
            Arrival::Hold => {
                self.held.insert(id, Image::from_rgba8(pixels));
            }
            Arrival::Discard => {}
        }
    }

    /// A versão da imagem que a lâmina grande mostra para a foto (spec MV, regra 2): muda quando
    /// a foto muda e quando a miniatura dá lugar à foto nítida (ou à original do recorte).
    pub fn version(&self, id: PhotoId) -> i32 {
        let kind = if self.original_id() == Some(id) {
            2
        } else if self.sharp(id).is_some() {
            1
        } else {
            0
        };
        i32::try_from(id.get() * 4 + kind).unwrap_or(-1)
    }

    /// O número do álbum mostrado (sobe a cada álbum novo).
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// A foto nítida que a lâmina grande pode mostrar agora.
    fn sharp(&self, id: PhotoId) -> Option<&Image> {
        self.screens
            .get(&id)
            .filter(|_| !self.gated || self.revealed.contains(&id))
    }

    /// Outra lâmina viva (rolo): as fotos nítidas voltam a entrar uma por quadro.
    pub fn reset_reveal(&mut self) {
        self.revealed.clear();
    }

    /// A próxima foto nítida de `ids` (a lâmina viva) que já chegou entra: a foto e a imagem dela.
    pub fn reveal_next(&mut self, ids: &[PhotoId]) -> Option<(PhotoId, Image)> {
        if !self.gated || self.sliding {
            return None;
        }
        let id = *ids
            .iter()
            .find(|id| self.screens.contains_key(id) && !self.revealed.contains(id))?;
        self.revealed.insert(id);
        Some((id, self.screens.get(&id)?.clone()))
    }

    /// A lâmina começou a deslizar (navegação).
    pub fn start_slide(&mut self) {
        self.sliding = true;
    }

    /// O deslize terminou: as fotos nítidas guardadas entram agora. `true` se alguma entrou.
    pub fn slide_ended(&mut self) -> bool {
        self.sliding = false;
        let ids = self.plan.slide_ended();
        for id in &ids {
            if let Some(img) = self.held.remove(id) {
                self.screens.insert(*id, img);
            }
        }
        !ids.is_empty()
    }

    pub fn original_id(&self) -> Option<PhotoId> {
        self.original.as_ref().map(|(id, _)| *id)
    }

    /// A foto original do recorte, já convertida pelo trabalhador.
    pub fn set_original(&mut self, id: PhotoId, pixels: Pixels) {
        self.original = Some((id, Image::from_rgba8(pixels)));
    }

    pub fn clear_original(&mut self) {
        self.original = None;
    }
}

/// Depois de tratar as mensagens: as linhas das miniaturas que chegaram são trocadas e, se chegou
/// foto nítida, a lâmina é redesenhada uma vez só.
pub fn after_arrivals(app: &mut App) {
    let fresh = app.images.take_fresh();
    if !fresh.is_empty() {
        crate::thumbs::show(app, &fresh);
    }
    if std::mem::take(&mut app.images.arrived) {
        crate::crop::refresh_spread(app);
    }
}

/// Fotos dos quadros da lâmina `index`.
fn photos_of(project: &Project, index: usize) -> Vec<PhotoId> {
    project.spreads.get(index).map_or_else(Vec::new, |s| {
        s.frames
            .iter()
            .filter_map(|f| match &f.content {
                FrameContent::Photo { photo, .. } => Some(*photo),
                _ => None,
            })
            .collect()
    })
}
