//! Monta o álbum do exemplo (spec PC, regra 6) com a mesma lógica da janela: 15 × 15 cm a
//! 300 DPI, 6 lâminas pela montagem automática; nas lâminas com aviso de pré-impressão ou com foto
//! sobre a dobra, o primeiro layout sem nenhum dos dois; e o título na primeira lâmina, pela
//! ferramenta da IA (`adicionar_texto`).
//! Sem as preferências do fotógrafo: o mesmo resultado em qualquer máquina.

use std::path::Path;

use lamina_core::FrameContent;
use lamina_editor::agent::{Agent, AgentOptions, ToolCall};
use lamina_editor::{
    Cache, Command, EXAMPLE_COPY, EXAMPLE_FILE, Editor, NewAlbum, Part, Session, SpecFields,
    create_album,
};
use serde_json::json;

const SPREADS: usize = 6;
const TITLE: &str = "Álbum de exemplo";

/// Monta e grava `exemplo.lamina` com as fotos de `dir/fotos`.
pub fn build(dir: &Path) -> Result<(), String> {
    let dir = std::path::absolute(dir).map_err(|e| e.to_string())?;
    let cache = Cache::new(std::env::temp_dir().join("lamina-exemplo-cache"));
    let fields = SpecFields {
        width_cm: 15.0,
        height_cm: 15.0,
        dpi: 300,
        ..SpecFields::default()
    };
    let new = NewAlbum {
        folder: dir.join("fotos"),
        name: Some(EXAMPLE_COPY.into()),
        spec: fields.to_spec().map_err(|e| e.to_string())?,
        spreads: Some(SPREADS),
        reuse: lamina_layout::Reuse::Medium,
        preferences: lamina_layout::Preferences::default(),
        learned: lamina_layout::Learned::default(),
        models: Vec::new(),
        limits: lamina_core::PageLimits::default(),
        export: lamina_core::ExportPrefs::default(),
        style: Default::default(),
        files: Vec::new(),
        build: true,
    };
    let (built, _) = create_album(&new, &cache, &|_| {}).map_err(|e| e.to_string())?;
    let file = dir.join(EXAMPLE_FILE);
    Session::new_unsaved(built.project)
        .save_as(&file)
        .map_err(|e| e.to_string())?;
    let mut editor = Editor::new();
    editor.open(&file).map_err(|e| e.to_string())?;
    for spread in 0..SPREADS {
        quiet_layout(&mut editor, spread);
    }
    add_title(&mut editor, &dir, cache)?;
    let session = editor.session_mut().ok_or("nenhum álbum aberto")?;
    session.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Uma lâmina boa para um leigo estrear: sem aviso de pré-impressão e sem foto atravessando a
/// dobra do meio (no álbum impresso, a costura corta a foto).
fn calm(editor: &Editor, spread: usize) -> bool {
    let Some(session) = editor.session() else {
        return false;
    };
    let crosses = session.project().spreads.get(spread).is_some_and(|s| {
        s.frames.iter().any(|f| {
            matches!(f.content, FrameContent::Photo { .. })
                && f.rect.x() < 0.49
                && f.rect.x() + f.rect.width() > 0.51
        })
    });
    session.warnings_for(spread).is_empty() && !crosses
}

/// Se a lâmina não está calma, o primeiro layout da lista que a deixa calma.
fn quiet_layout(editor: &mut Editor, spread: usize) {
    if !editor.go_to_spread(spread) || calm(editor, spread) {
        return;
    }
    let count = editor.panel(Part::Spread).options.len();
    for option in 0..count {
        if editor.apply_option(Part::Spread, option) {
            if calm(editor, spread) {
                return;
            }
            editor.run(Command::Undo);
        }
    }
}

/// O título na faixa branca embaixo das fotos da página direita da primeira lâmina, centrado e
/// dentro da zona segura.
fn add_title(editor: &mut Editor, dir: &Path, cache: Cache) -> Result<(), String> {
    let options = AgentOptions {
        roots: vec![dir.to_path_buf()],
        images: false,
        cache,
        presets: Vec::new(),
    };
    let mut agent = Agent::new(options);
    let added = json!({
        "lamina": 1,
        "pagina": "direita",
        "texto": TITLE,
        "fonte": "Playfair Display",
        "tamanho": 20,
        "posicao_mm": { "x": 160.0, "y": 127.0, "largura": 130.0, "altura": 11.0 },
    });
    let reply = tool(&mut agent, editor, "adicionar_texto", added)?;
    let frame = reply["quadro"]
        .as_u64()
        .ok_or("o texto não voltou com o quadro")?;
    let styled = json!({
        "lamina": 1,
        "quadro": frame,
        "alinhamento": "centro",
        "vertical": "meio",
        "cor": "#3A3A3A",
    });
    tool(&mut agent, editor, "editar_texto", styled).map(|_| ())
}

fn tool(
    agent: &mut Agent,
    editor: &mut Editor,
    name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let call = ToolCall::parse(name, args).map_err(|e| e.to_string())?;
    agent
        .run(editor, call, &|_, _| {})
        .map(|reply| reply.data)
        .map_err(|e| e.to_string())
}
