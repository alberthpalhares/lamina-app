//! O catálogo das ferramentas (spec IA, seção 3): nome, descrição e esquema dos parâmetros de
//! cada uma, numa fonte só. O servidor MCP e o manifesto do pacote do Claude Desktop saem daqui;
//! a lista em ordem está em `list.rs`.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use super::ToolError;
use super::calls::{
    ApplyLayout, BlankPageCall, BuildAlbumCall, BuildSpread, CoverCall, ExportCall, FrameArg,
    LeaveAlbum, ListPhotos, MovePhoto, MoveSpread, NewAlbumCall, NewSpreadCall, NoArgs, OpenAlbum,
    PlacePhoto, RenameAlbum, RunCommand, SaveAlbum, SpreadArg, Steps, SuggestLayouts, SwapPhotos,
    ViewPhotos, ViewSpread,
};
use super::params::{
    AddText, CropCall, EditText, LookCall, RelinkCall, RespecCall, SaveModel, UseModel,
};

mod list;

pub use list::tools;

/// Uma chamada de ferramenta, já com os parâmetros conferidos.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "ferramenta", content = "parametros")]
pub enum ToolCall {
    #[serde(rename = "abrir_album")]
    OpenAlbum(OpenAlbum),
    #[serde(rename = "novo_album")]
    NewAlbum(NewAlbumCall),
    #[serde(rename = "salvar_album")]
    SaveAlbum(SaveAlbum),
    #[serde(rename = "renomear_album")]
    RenameAlbum(RenameAlbum),
    #[serde(rename = "resumo_do_album")]
    Summary(NoArgs),
    #[serde(rename = "ver_lamina")]
    ViewSpread(ViewSpread),
    #[serde(rename = "listar_fotos")]
    ListPhotos(ListPhotos),
    #[serde(rename = "ver_fotos")]
    ViewPhotos(ViewPhotos),
    #[serde(rename = "pre_impressao")]
    Preflight(NoArgs),
    #[serde(rename = "sugerir_layouts")]
    SuggestLayouts(SuggestLayouts),
    #[serde(rename = "aplicar_layout")]
    ApplyLayout(ApplyLayout),
    #[serde(rename = "espelhar")]
    Mirror(SpreadArg),
    #[serde(rename = "colocar_foto")]
    PlacePhoto(PlacePhoto),
    #[serde(rename = "trocar_fotos")]
    SwapPhotos(SwapPhotos),
    #[serde(rename = "mover_foto")]
    MovePhoto(MovePhoto),
    #[serde(rename = "tirar_foto")]
    ClearFrame(FrameArg),
    #[serde(rename = "tirar_do_album")]
    LeaveAlbum(LeaveAlbum),
    #[serde(rename = "montar_lamina")]
    BuildSpread(BuildSpread),
    #[serde(rename = "montar_album")]
    BuildAlbum(BuildAlbumCall),
    #[serde(rename = "nova_lamina")]
    NewSpread(NewSpreadCall),
    #[serde(rename = "pagina_em_branco")]
    BlankPage(BlankPageCall),
    #[serde(rename = "criar_capa")]
    CreateCover(CoverCall),
    #[serde(rename = "medidas_da_capa")]
    CoverMeasures(CoverCall),
    #[serde(rename = "apagar_lamina")]
    DeleteSpread(SpreadArg),
    #[serde(rename = "mover_lamina")]
    MoveSpread(MoveSpread),
    #[serde(rename = "desfazer")]
    Undo(Steps),
    #[serde(rename = "refazer")]
    Redo(Steps),
    #[serde(rename = "historico")]
    History(NoArgs),
    #[serde(rename = "exportar")]
    Export(ExportCall),
    #[serde(rename = "executar_comando")]
    RunCommand(RunCommand),
    #[serde(rename = "recortar")]
    Crop(CropCall),
    #[serde(rename = "aparencia")]
    Look(LookCall),
    #[serde(rename = "adicionar_texto")]
    AddText(AddText),
    #[serde(rename = "editar_texto")]
    EditText(EditText),
    #[serde(rename = "usar_modelo")]
    UseModel(UseModel),
    #[serde(rename = "salvar_modelo")]
    SaveModel(SaveModel),
    #[serde(rename = "trocar_especificacao")]
    Respec(RespecCall),
    #[serde(rename = "reencontrar_fotos")]
    Relink(RelinkCall),
}

impl ToolCall {
    /// A chamada da ferramenta `name` com os `args` do cliente (`null` = sem parâmetros).
    pub fn parse(name: &str, args: Value) -> Result<ToolCall, ToolError> {
        if !tools().iter().any(|t| t.name == name) {
            return Err(ToolError::UnknownTool(name.to_owned()));
        }
        let args = if args.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            args
        };
        let call = serde_json::json!({ "ferramenta": name, "parametros": args });
        serde_json::from_value(call).map_err(|e| ToolError::BadArguments(e.to_string()))
    }
}

/// Uma ferramenta do catálogo.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub description: &'static str,
    /// Esquema JSON dos parâmetros.
    pub schema: Value,
}

fn info<T: JsonSchema>(name: &'static str, description: &'static str) -> ToolInfo {
    ToolInfo {
        name,
        description,
        schema: schemars::schema_for!(T).to_value(),
    }
}
