//! Controle por IA (spec IA; ADR 0010): o catálogo das ferramentas, a execução sobre o
//! [`Editor`] e as respostas. Síncrono e sem protocolo: o `lamina-mcp` só traduz o MCP para cá,
//! e o modo ao vivo vai chamar a mesma execução na linha da janela.
//!
//! Toda edição de uma ferramenta é um passo do histórico com o rótulo "IA: …" (regra 4); lâmina,
//! quadro, foto e opção contam a partir de 1, como na tela (regra 2).

use std::collections::HashMap;
use std::path::PathBuf;

use lamina_core::Spread;
use serde_json::json;

use crate::{Cache, Editor, Part, Preset};

use lookup::{edit_on, frame_index, photo_frame, photo_id, session, session_mut, spread_index};

mod album;
mod building;
mod calls;
mod catalog;
mod commands;
mod crop;
mod cross;
mod error;
mod export;
mod frames;
mod layout;
mod leave;
mod listing;
mod look;
mod lookup;
mod models;
mod params;
mod paths;
mod prompts;
mod read;
mod relink;
mod reply;
mod respec;
mod roots;
mod setup;
mod spreads;
mod texts;
mod values;

pub use album::SPREADS_NEED_BUILD;
pub use calls::{
    ApplyLayout, BlankPageCall, BlankSide, BuildAlbumCall, BuildSpread, ColorArg, ExportCall,
    FormatArg, FrameArg, LeaveAlbum, ListPhotos, MovePhoto, MoveSpread, NewAlbumCall,
    NewSpreadCall, NoArgs, OpenAlbum, OrderArg, PartArg, PhotoFilter, PlacePhoto, RenameAlbum,
    RunCommand, SaveAlbum, SpreadArg, Steps, SuggestLayouts, SwapPhotos, ViewPhotos, ViewSpread,
};
pub use catalog::{ToolCall, ToolInfo, tools};
pub use error::ToolError;
pub use listing::commands_data;
pub use params::{
    AddText, AlignArg, CropCall, EditText, LookCall, PageArg, PlaceMm, RelinkCall, RespecCall,
    SaveModel, UseModel, VAlignArg,
};
pub use prompts::{READY_PROMPTS, ReadyPrompt, prompt_text};
pub use reply::{Progress, ToolImage, ToolReply};
pub use roots::AllowedRoots;
pub use setup::photographer;

/// O começo do rótulo das edições da IA no histórico (spec IA, seção 4).
pub const AI_LABEL: &str = "IA: ";

/// Como o servidor foi ligado (`lamina mcp`, ou o pacote do Claude Desktop).
#[derive(Debug, Clone)]
pub struct AgentOptions {
    /// As pastas de `--permitir`.
    pub roots: Vec<PathBuf>,
    /// `false` com `--sem-imagens`: nenhuma resposta leva imagem (regra 8).
    pub images: bool,
    pub cache: Cache,
    /// Os presets do Lamina e os do fotógrafo, para o `novo_album`.
    pub presets: Vec<Preset>,
}

/// O estado da IA ao lado do editor: pastas, imagens, a última lista de layouts de cada lâmina e
/// o arquivo como estava no disco ao abrir ou salvar (regras 3, 5 e 6).
#[derive(Debug)]
pub struct Agent {
    roots: AllowedRoots,
    images: bool,
    cache: Cache,
    presets: Vec<Preset>,
    lists: HashMap<(usize, Part), Listed>,
    disk: Option<Vec<u8>>,
}

/// Uma lista de `sugerir_layouts`: a lâmina como estava e quantas opções foram mostradas.
#[derive(Debug)]
struct Listed {
    spread: Spread,
    /// A posição no painel de cada opção mostrada, na ordem da lista que a IA viu.
    picks: Vec<usize>,
}

impl Agent {
    pub fn new(options: AgentOptions) -> Self {
        Agent {
            roots: AllowedRoots::new(&options.roots),
            images: options.images,
            cache: options.cache,
            presets: options.presets,
            lists: HashMap::new(),
            disk: None,
        }
    }

    pub fn images(&self) -> bool {
        self.images
    }

    pub fn roots(&self) -> &AllowedRoots {
        &self.roots
    }

    /// Roda a ferramenta. As edições entram no histórico como "IA: …".
    pub fn run(
        &mut self,
        editor: &mut Editor,
        call: ToolCall,
        progress: Progress<'_>,
    ) -> Result<ToolReply, ToolError> {
        editor.set_selection(Vec::new());
        if let Some(s) = editor.session_mut() {
            s.set_label_prefix(AI_LABEL);
        }
        let result = self.dispatch(editor, call, progress);
        if let Some(s) = editor.session_mut() {
            s.set_label_prefix("");
        }
        result
    }

    fn dispatch(
        &mut self,
        editor: &mut Editor,
        call: ToolCall,
        progress: Progress<'_>,
    ) -> Result<ToolReply, ToolError> {
        match call {
            ToolCall::OpenAlbum(a) => self.open_album(editor, &a),
            ToolCall::NewAlbum(a) => self.new_album(editor, &a, progress),
            ToolCall::SaveAlbum(a) => self.save_album(editor, &a),
            ToolCall::RenameAlbum(a) => album::rename_album(editor, &a.nome),
            ToolCall::Summary(_) => self.summary(editor),
            ToolCall::ViewSpread(a) => self.view_spread(editor, &a),
            ToolCall::ListPhotos(a) => listing::list_photos(editor, a.filtro),
            ToolCall::ViewPhotos(a) => self.view_photos(editor, &a),
            ToolCall::Preflight(_) => listing::preflight(editor),
            ToolCall::SuggestLayouts(a) => self.suggest(editor, &a),
            ToolCall::ApplyLayout(a) => self.apply(editor, &a),
            ToolCall::Mirror(a) => self.mirror(editor, a.lamina),
            ToolCall::PlacePhoto(a) => self.place(editor, &a),
            ToolCall::SwapPhotos(a) => self.swap(editor, &a),
            ToolCall::MovePhoto(a) => self.move_photo(editor, &a),
            ToolCall::ClearFrame(a) => self.clear(editor, &a),
            ToolCall::LeaveAlbum(a) => self.leave_album(editor, &a),
            ToolCall::BuildSpread(a) => self.build_spread(editor, &a),
            ToolCall::BuildAlbum(a) => self.build_album(editor, &a, progress),
            ToolCall::NewSpread(a) => self.new_spread(editor, &a),
            ToolCall::BlankPage(a) => self.blank_page(editor, &a),
            ToolCall::DeleteSpread(a) => self.delete_spread(editor, a.lamina),
            ToolCall::MoveSpread(a) => self.move_spread(editor, &a),
            ToolCall::Undo(a) => spreads::undo_redo(editor, a.passos, true),
            ToolCall::Redo(a) => spreads::undo_redo(editor, a.passos, false),
            ToolCall::History(_) => listing::history(editor),
            ToolCall::Export(a) => self.export(editor, &a, progress),
            ToolCall::RunCommand(a) => self.run_command(editor, &a),
            ToolCall::Crop(a) => self.crop(editor, &a),
            ToolCall::Look(a) => self.look(editor, &a),
            ToolCall::AddText(a) => self.add_text(editor, &a),
            ToolCall::EditText(a) => self.edit_text(editor, &a),
            ToolCall::UseModel(a) => self.use_model(editor, &a),
            ToolCall::SaveModel(a) => self.save_model(editor, &a),
            ToolCall::Respec(a) => self.respec(editor, &a),
            ToolCall::Relink(a) => self.relink(editor, &a, progress),
        }
    }

    /// Resposta de uma edição: a lâmina atualizada e "alterações não salvas" (seção 4).
    fn edited(&self, editor: &Editor, index: usize, text: &str) -> Result<ToolReply, ToolError> {
        let s = session(editor)?;
        let data = json!({
            "lamina": read::spread_data(s, index),
            "alteracoes_nao_salvas": s.is_dirty(),
        });
        Ok(ToolReply::new(text, data))
    }
}
