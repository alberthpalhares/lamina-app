//! # lamina-editor
//!
//! A lógica do aplicativo, sem janela (spec `janela-principal`; regra 1 e ADR 0001). A janela
//! Slint (`lamina-app`) só desenha estes dados e chama estes comandos; a linha de comando usa o
//! mesmo caminho para montar um álbum novo.
//!
//! - [`album`]: novo álbum a partir de uma pasta (importar → sugerir lâminas → Auto Build).
//! - [`building`]: "Montar o álbum…", as fotos não usadas em lâminas novas (spec BR).
//! - [`cache`]: catálogo, miniaturas e imagens de tela no cache do sistema.
//! - [`session`]: o projeto aberto, o arquivo dele, a lâmina atual e as alterações não salvas.
//! - [`editor`]: a sessão (ou nenhuma), os comandos e a pergunta antes de sair.
//! - [`commands`] e [`keys`]: o registro de comandos, com nome, rótulo e atalho.
//! - [`suggestions`]: as sugestões de layout da lâmina, trocar e espelhar (spec TL).
//! - [`prefs_store`]: favoritos e rejeitados no arquivo de preferências do fotógrafo.
//! - [`text`]: nome das famílias e nota explicada, para o painel de layouts.
//! - [`view`]: a lâmina em pixels da tela, igual à impressão.
//! - [`photos`]: o painel de fotos (as abas e os usos de cada foto).
//! - [`photo_grid`]: a grade do painel de fotos, o zoom, a altura e o laço (spec PF).
//! - [`presets`] e [`respec`]: presets e trocar a especificação de um álbum pronto (spec PR).
//! - [`strips`]: as três faixas de baixo, com título e recolhíveis (spec PN).
//! - [`recents`]: a lista de recentes e o que a tela inicial mostra (spec TI).
//! - [`received`]: os arquivos que o sistema entrega ao Lamina (duplo clique; spec MA).
//! - [`forms`]: campos, passos e textos das janelas (novo álbum, exportar, aparência).
//! - [`exporting`]: exportar pela janela, com a pré-impressão antes (spec EI).
//! - [`relink`]: reencontrar fotos que mudaram de pasta (spec EI).
//! - [`screens`]: as imagens de tela na memória, só da lâmina atual e das vizinhas.
//! - [`agent`]: o controle por IA, as ferramentas que o servidor MCP oferece (spec IA).
//! - [`human`]: cada erro dito ao fotógrafo, com o que fazer e o detalhe (spec EH).
//! - [`importing`]: fotos entrando num álbum aberto (spec IM).
//! - [`error_log`] e [`feedback`]: o registro de erros e o formulário de sugestões (spec SR).
//! - [`motion`]: movimento e fluidez, a medição de tempos e as vagas da lâmina (spec MV); o plano
//!   e as contas do rolo de lâminas (spec RL).
//! - [`roll_image`]: a imagem de cada lâmina do rolo, só com fotos do cache (spec RL).

pub mod adjusted;
pub mod agent;
pub mod album;
pub mod appearance;
mod applying;
pub mod arrange;
pub mod backup;
pub mod bar_tips;
pub mod blank;
pub mod building;
pub mod cache;
pub mod calibration;
pub mod commands;
pub mod cover_form;
pub mod editor;
pub mod edits;
pub mod edits_cross;
pub mod empty_import;
pub mod error_log;
pub mod example;
mod export_form;
pub mod exporting;
pub mod feedback;
pub mod filming;
pub mod font_picker;
pub mod forms;
pub mod frame_form;
pub mod group;
pub mod hand;
mod hand_click;
pub mod human;
pub mod importing;
pub mod keys;
pub mod lab_presets;
pub mod leaving;
mod look_form;
pub mod magnets;
pub mod marks;
pub mod models;
pub mod motion;
pub mod naming;
pub mod new_project;
pub mod number_step;
pub mod organize_bar;
pub mod page_meter;
pub mod palette_tips;
pub mod panel_width;
pub mod photo_grid;
pub mod photos;
pub mod prefs_store;
pub mod preset_names;
pub mod presets;
pub mod received;
pub mod recents;
pub mod relink;
pub mod relink_discover;
pub mod relink_folders;
pub mod relink_moved;
pub mod relink_text;
pub mod removing;
pub mod respec;
pub mod reveal;
pub mod roll_image;
pub mod rulers;
pub mod screens;
pub mod sequence;
pub mod session;
pub mod shortcuts;
pub mod sliders;
pub mod snap;
pub mod spreads;
pub mod strips;
pub mod suggestions;
pub mod text;
pub mod text_scale;
pub mod thumbs;
pub mod updates;
pub mod view;

mod error;
#[cfg(test)]
mod tests;

pub use album::{
    Imported, NewAlbum, SpecFields, Step, check_spreads, count_photos, create_album, folder_name,
    import_project, project_from, suggestion, suggestion_for, suggestion_text, suggestion_within,
};
pub use bar_tips::{BAR_COMMANDS, bar_tip};
pub use cache::Cache;
pub use commands::{Command, Menu};
pub use editor::{
    CrossTarget, DropPlace, Editor, Leave, MAX_ALBUM_NAME, Panel, TextChange, clean_album_name,
};
pub use error::EditorError;
pub use error_log::ErrorLog;
pub use example::{EXAMPLE_COPY, EXAMPLE_DIR, EXAMPLE_FILE, example_target, prepare_example};
pub use exporting::{
    CoverExport, DEFAULT_QUALITY, ExportCheck, ExportFormat, ExportOptions, Warning,
    export_progress, export_project, export_summary,
};
pub use feedback::{FEEDBACK_FORM, feedback_address};
pub use human::{Human, HumanMessage, guarded};
pub use keys::{Key, Platform, Shortcut};
pub use lamina_render::text_image;
pub use models::{MODELS_FILE, ModelError, ModelsStore, model_from_spread};
pub use photos::{
    Filter, PhotoInfo, PhotoItem, photo_info, photo_list, same_photo_list, used_photos,
};
pub use prefs_store::{PREFS_FILE, PrefsStore, config_dir};
pub use presets::{PRESETS_FILE, Preset, PresetError, PresetsStore, builtin_presets};
pub use recents::{RECENTS_FILE, Recent, RecentsStore};
pub use relink::{
    Found, RELINK_LATER, not_found_message, relinked_message, search_folder, search_near,
};
pub use relink_discover::{Discovery, discover};
pub use relink_folders::{MissingFolder, disconnected, missing_folders};
pub use relink_moved::{Located, locate, search_moved};
pub use roll_image::{
    ImageSize, ROLL_IMAGES_BYTES, RollImages, RollPainter, RollSource, SpreadImage,
    render_spread_image,
};
pub use screens::{
    Arrival, ScreenPlan, load_original, load_screen, load_thumb, load_thumb_sized, prepare_images,
};
pub use session::{HistoryList, HistoryStep, Session, missing_among, photo_paths, still_missing};
pub use snap::{Grip, SnapLine};
pub use strips::{
    Install, STRIPS_FILE, STRIPS_UNREADABLE, STRIPS_VERSION, Strip, Strips, StripsStore,
    photos_title, spread_strip_height, strip_titles,
};
pub use suggestions::{List, Part, Unavailable};
pub use text::{
    BAD_COLOR, PALETTE, color_hex, family_name, frame_dpi_text, option_title, parse_color,
    score_text,
};
pub use thumbs::{THUMBS_KEPT, ThumbList, ThumbOrders, ThumbPlan};
pub use updates::{UPDATES_FILE, Updates, UpdatesStore};
pub use view::{
    Area, FrameKind, FrameView, PxRect, SpreadView, frame_at, layers, photo_extent, spread_view,
};
