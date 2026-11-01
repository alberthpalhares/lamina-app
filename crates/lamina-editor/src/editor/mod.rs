//! O editor: a sessão aberta (ou nenhuma), o estado da tela que não vai para o arquivo (guias,
//! lista de layouts ativa), as preferências de layout e a pergunta antes de sair com alterações
//! não salvas (spec JP, regras 7, 8 e 10; spec TL).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use lamina_core::{Crop, PhotoId, Project};
use lamina_layout::{LayoutLists, Preferences};

use crate::photos::{Filter, photo_list};
use crate::prefs_store::PrefsStore;
use crate::strips::{Strip, Strips, StripsStore};
use crate::{EditorError, Session};

mod adjust;
mod arrange_cmd;
mod backup;
mod band;
mod blank;
mod bottom_strips;
mod build_album;
mod clear_all;
mod clipboard;
mod context;
pub(crate) mod cover;
mod crop;
mod details;
mod dispatch;
mod dispatch_view;
mod dropping;
mod frames;
mod group_fields;
mod group_gesture;
mod hand;
mod hand_frames;
mod layouts;
mod leave_album;
mod models;
mod mounting;
mod moving;
mod neutral;
mod organize;
mod organize_cmd;
mod organize_moves;
mod organize_zoom;
mod panel_need;
mod photo_panel;
mod photo_sliders;
mod photo_style;
mod recents;
mod rename;
mod report;
mod sequence_move;
mod style;
mod text_edit;

pub use adjust::{AdjustChange, BAD_ANGLE, BAD_SCALE, CropGrid, parse_angle};
pub use arrange_cmd::NEED_THREE;
pub use band::{Band, Card};
pub use context::ContextEntry;
pub use crop::{ZOOM_STEP, crop_for, set_crop};
pub use details::{Details, album_rows, album_sections};
pub use dropping::DropPlace;
pub use group_fields::{Field, GROUP_NOT_A_NUMBER};
pub use layouts::Panel;
pub use moving::CrossTarget;
pub use neutral::AdjustControl;
pub use organize::{DoubleClick, PaletteTool, SidePanel};
pub use organize_zoom::{
    MAX_VIEW_ZOOM, VIEW_STEP, Zoom, ZoomMove, real_factor, step_factor, times_label, zoom_about,
};
pub use panel_need::TextFocus;
pub use photo_style::PhotoStyle;
pub use rename::{MAX_ALBUM_NAME, clean_album_name};
pub use style::Look;
pub use text_edit::TextChange;

/// Resposta à pergunta "Salvar as alterações?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    Save,
    DontSave,
    Cancel,
}

/// Listas do motor já calculadas, pelas proporções das fotos e páginas ativas (regra 10 do TL).
type ListsKey = (Vec<u64>, bool, bool);

#[derive(Debug)]
pub struct Editor {
    session: Option<Session>,
    guides: bool,
    /// O guia da capa escondido: é só da vista, não entra no histórico (spec CA, regra 18).
    guide_hidden: bool,
    reordered: bool,
    prefs: PrefsStore,
    lists: RefCell<HashMap<ListsKey, LayoutLists>>,
    notice: Option<String>,
    /// A página do último clique direito no fundo da lâmina (spec PB).
    context_side: Option<lamina_core::PageSide>,
    selected_frame: Option<usize>,
    selection: Vec<PhotoId>,
    /// Última foto clicada sem Shift: começo do intervalo do Shift+clique.
    anchor: Option<PhotoId>,
    /// A aba do painel de fotos e a primeira foto visível da grade (spec PF, regra 7).
    photo_filter: Filter,
    grid_anchor: Option<PhotoId>,
    /// Quadro em modo recorte e o recorte provisório do gesto (spec RC).
    crop_frame: Option<usize>,
    pending_crop: Option<Crop>,
    /// O ângulo da foto está mudando: a grade fina aparece no recorte (spec AF, seção 4).
    turning: bool,
    /// A janela deve abrir a aba Detalhes (spec AF, regra 10).
    wants_details: bool,
    /// A janela deve abrir a aba Quadro do texto selecionado, com ou sem o cursor (spec TX).
    text_focus: Option<TextFocus>,
    /// A última leitura da câmera (spec AF, regra 12): o arquivo é lido ao selecionar a foto, não
    /// a cada redesenho.
    camera: std::cell::RefCell<Option<(std::path::PathBuf, lamina_media::CameraDetails)>>,
    /// Trabalho em segundo plano (novo álbum, exportar, procurar fotos): os comandos esperam.
    busy: bool,
    /// As faixas de baixo e o arquivo delas (spec PN).
    strips: StripsStore,
    /// Quadro sendo movido ou redimensionado (spec TX).
    gesture: Option<hand::Gesture>,
    /// Modelos do fotógrafo (spec MD).
    models: crate::models::ModelsStore,
    /// O registro de erros da janela (spec SR); nenhum nos testes e na linha de comando.
    log: Option<crate::ErrorLog>,
    /// A lista de recentes (spec TI); só na memória nos testes e na linha de comando.
    recents: crate::recents::RecentsStore,
    /// O modo organizar a lâmina (spec EG) e o gesto do grupo.
    organize: organize::Organizing,
    group: Option<group_gesture::GroupGesture>,
    /// A área de cópia do modo organizar (spec EG, regra 12); não vai para o arquivo.
    clipboard: Option<clipboard::Clip>,
    /// O pedido do painel temporário (spec JE, regra 5); some sozinho quando acaba.
    panel_need: std::cell::Cell<Option<panel_need::PanelNeed>>,
    /// A faixa de layouts e a prévia na lâmina (spec FL); não vão para o arquivo.
    band: band::BandState,
    /// A largura de Ajustar e o monitor, para o tamanho real (spec ZR, regras 7 e 10).
    view: ViewArea,
    /// O álbum recebido (duplo clique) que espera "Salvar as alterações?" (spec MA, regra 4).
    pub(crate) received: Option<std::path::PathBuf>,
    /// As cópias de segurança (spec CS); nenhuma nos testes e na linha de comando.
    backups: Option<crate::backup::Backups>,
}

/// O que a janela informa para o tamanho real: a largura de Ajustar (pixels lógicos, sem a
/// animação) e o monitor em que ela está.
#[derive(Debug, Default)]
struct ViewArea {
    fit_px: f64,
    monitor: crate::calibration::Monitor,
}

impl Default for Editor {
    fn default() -> Self {
        Editor::new()
    }
}

impl Editor {
    /// Editor com as preferências só na memória (testes e linha de comando).
    pub fn new() -> Self {
        Editor::with_prefs(PrefsStore::default())
    }

    /// Editor com as preferências do arquivo do fotógrafo. O aviso de leitura, se houver, vira
    /// o primeiro aviso da janela.
    pub fn with_prefs(mut prefs: PrefsStore) -> Self {
        let notice = prefs.warning.take();
        Editor {
            session: None,
            guides: true,
            guide_hidden: false,
            reordered: false,
            prefs,
            lists: RefCell::new(HashMap::new()),
            notice,
            context_side: None,
            selected_frame: None,
            selection: Vec::new(),
            anchor: None,
            photo_filter: Filter::All,
            grid_anchor: None,
            crop_frame: None,
            pending_crop: None,
            turning: false,
            wants_details: false,
            text_focus: None,
            camera: std::cell::RefCell::new(None),
            busy: false,
            strips: StripsStore::default(),
            gesture: None,
            models: crate::models::ModelsStore::default(),
            log: None,
            recents: crate::recents::RecentsStore::default(),
            organize: organize::Organizing::default(),
            group: None,
            clipboard: None,
            panel_need: std::cell::Cell::new(None),
            band: band::BandState::default(),
            view: ViewArea {
                fit_px: 1000.0,
                monitor: crate::calibration::Monitor::default(),
            },
            received: None,
            backups: None,
        }
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    pub fn guides(&self) -> bool {
        self.guides
    }

    /// A lista ativa no painel é a das reordenadas?
    pub fn reordered(&self) -> bool {
        self.reordered
    }

    pub fn preferences(&self) -> &Preferences {
        &self.prefs.prefs
    }

    /// Aviso para a janela mostrar (uma vez): preferências que não leram ou não gravaram.
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    /// Abre o arquivo, na lâmina em que o fotógrafo parou (spec TI, regra 3). Se não der, a
    /// sessão anterior continua como estava (JP-B11).
    pub fn open(&mut self, file: &Path) -> Result<(), EditorError> {
        let session = Session::open(file)?;
        self.remember_current();
        self.forget_view();
        self.session = Some(session);
        self.after_open();
        Ok(())
    }

    /// Passa a editar um álbum novo, ainda sem arquivo.
    pub fn adopt(&mut self, project: Project) {
        self.remember_current();
        self.forget_view();
        self.session = Some(Session::new_unsaved(project));
    }

    /// Outro projeto na tela: nada do anterior sobra (quadro, seleção, modo recorte e os gestos
    /// pendentes, de recorte e de quadro, que não podem virar edição no projeto novo; auditorias
    /// dos Marcos 2 e 3, M2).
    fn forget_view(&mut self) {
        self.gesture = None;
        self.panel_need.set(None);
        self.selected_frame = None;
        self.selection.clear();
        self.anchor = None;
        self.grid_anchor = None;
        self.crop_frame = None;
        self.pending_crop = None;
        self.turning = false;
        // Abrir outro álbum sai do modo organizar (spec EG).
        self.organize = organize::Organizing::default();
        self.group = None;
        // As fotos copiadas são do álbum anterior.
        self.clipboard = None;
        // A faixa mostrava as opções de uma lâmina do álbum anterior (spec FL).
        self.band = band::BandState::default();
    }

    /// Passa a usar o estado das faixas de `store` (lido de `janela.json`). O aviso de leitura,
    /// se houver, vira o próximo aviso da janela.
    pub fn set_strips(&mut self, mut store: StripsStore) {
        if let Some(warning) = store.warning.take() {
            self.notice = Some(warning);
        }
        self.strips = store;
    }

    /// Quais faixas estão abertas.
    pub fn strips(&self) -> Strips {
        self.strips.strips.clone()
    }

    /// "Reduzir movimento" ligado (spec MV, regra 14).
    pub fn reduce_motion(&self) -> bool {
        self.strips.strips.reduce_motion
    }

    /// As durações dos movimentos agora (zero com "Reduzir movimento").
    pub fn durations(&self) -> crate::motion::Durations {
        crate::motion::Durations::new(self.reduce_motion())
    }

    /// Liga ou desliga "Reduzir movimento" e grava em `janela.json`; se não gravar, vale nesta
    /// sessão, com aviso.
    fn toggle_reduce_motion(&mut self) -> bool {
        self.strips.strips.reduce_motion = !self.strips.strips.reduce_motion;
        if let Err(message) = self.strips.save() {
            self.notice = Some(message);
        }
        true
    }

    /// A sombra da dobra aparece na lâmina grande agora (spec DJ, regra 2): com a preferência
    /// ligada, numa lâmina de duas páginas, fora do modo organizar e do tamanho real.
    pub fn gutter_shadow_shown(&self) -> bool {
        let Some(s) = self.session.as_ref() else {
            return false;
        };
        let edges = s.project().spec.edges();
        let pages = edges.active_pages(s.current(), s.spread_count());
        self.strips.strips.gutter_shadow
            && pages.left
            && pages.right
            && !self.organizing()
            && !s.on_cover()
    }

    /// A preferência da sombra da dobra, que vale também na tela inicial (spec DI).
    pub fn gutter_shadow_on(&self) -> bool {
        self.strips.strips.gutter_shadow
    }

    /// Liga ou desliga a sombra da dobra e grava; se não gravar, vale nesta sessão, com aviso.
    fn toggle_gutter_shadow(&mut self) -> bool {
        self.strips.strips.gutter_shadow = !self.strips.strips.gutter_shadow;
        self.save_strips();
        true
    }

    /// Recolhe ou abre a faixa e grava; se não gravar, vale nesta sessão, com aviso (spec PN,
    /// seção 6).
    fn toggle_strip(&mut self, strip: Strip) -> bool {
        self.strips.strips.toggle(strip);
        self.save_strips();
        true
    }

    /// Grava `janela.json`; se não gravar, vale nesta sessão, com aviso (spec PN, seção 6).
    fn save_strips(&mut self) {
        if let Err(message) = self.strips.save() {
            self.notice = Some(message);
        }
    }

    /// Há trabalho em segundo plano? Enquanto houver, só "Sobre" fica disponível (auditoria do
    /// Marco 2, M8): abrir ou criar outro álbum no meio dele trocaria o projeto sem perguntar.
    pub fn busy(&self) -> bool {
        self.busy
    }

    pub fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
    }

    /// Vai para a lâmina `index` (clique na faixa): sai do modo recorte, gravando o gesto, e
    /// desmarca o quadro (spec RC, regra 1). `false` se a lâmina não existe.
    pub fn go_to_spread(&mut self, index: usize) -> bool {
        let count = self.session.as_ref().map_or(0, |s| s.album().spreads.len());
        if index >= count {
            return false;
        }
        self.exit_crop();
        self.gesture = None;
        self.selected_frame = None;
        self.session.as_mut().is_some_and(|s| s.go_to(index))
    }

    /// Fechar, abrir outro ou criar outro precisa perguntar antes?
    pub fn needs_confirmation(&self) -> bool {
        self.session.as_ref().is_some_and(Session::is_dirty)
    }

    /// Aplica a resposta à pergunta. `true` = pode seguir (fechar, abrir, criar). Salvar um
    /// projeto sem arquivo usa `save_to`, escolhido no diálogo "Salvar como".
    pub fn leave(&mut self, choice: Leave, save_to: Option<&Path>) -> Result<bool, EditorError> {
        match choice {
            Leave::Cancel => {
                // Cancelar também esquece o álbum recebido que esperava (spec MA, regra 4).
                self.received = None;
                Ok(false)
            }
            Leave::DontSave => Ok(true),
            Leave::Save => {
                let session = self.session.as_ref().ok_or(EditorError::NoProject)?;
                if session.file().is_none() && save_to.is_none() {
                    return Err(EditorError::NoFile);
                }
                self.save_to(save_to)?;
                Ok(true)
            }
        }
    }

    /// Vai para a primeira lâmina da foto (duplo clique no navegador). `false` se não é usada.
    pub fn go_to_photo(&mut self, id: PhotoId) -> bool {
        let first = self.session.as_ref().and_then(|s| {
            photo_list(s.album(), Filter::All)
                .into_iter()
                .find(|item| item.id == id)
                .and_then(|item| item.first_spread)
        });
        first.is_some_and(|index| self.go_to_spread(index))
    }
}
