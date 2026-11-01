//! Os recursos (spec IA, seção 3): o que o cliente lê sem chamar ferramenta. O resumo, a lâmina e
//! a prévia passam pelas mesmas ferramentas; o esquema e os comandos vêm do núcleo e do registro.

use lamina_editor::agent::{NoArgs, ToolCall, ViewSpread, commands_data};
use rmcp::ErrorData;
use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult, Resource,
    ResourceContents, ResourceTemplate,
};

use crate::{Server, convert};

const ALBUM: &str = "lamina://album";
const SCHEMA: &str = "lamina://esquema";
const COMMANDS: &str = "lamina://comandos";
const SPREAD: &str = "lamina://lamina/";
const PREVIEW: &str = "/previa.png";
const JSON: &str = "application/json";

pub(crate) fn list() -> ListResourcesResult {
    let resource = |uri: &str, name: &str, description: &str| {
        Resource::new(uri, name)
            .with_description(description)
            .with_mime_type(JSON)
    };
    ListResourcesResult::with_all_items(vec![
        resource(ALBUM, "album", "O resumo do álbum aberto."),
        resource(SCHEMA, "esquema", "O esquema JSON do arquivo .lamina."),
        resource(
            COMMANDS,
            "comandos",
            "Os comandos do Lamina, com nome estável, rótulo, menu e atalho.",
        ),
    ])
}

/// A lâmina sempre; a prévia só com as imagens ligadas (regra 8).
pub(crate) fn templates(images: bool) -> ListResourceTemplatesResult {
    let mut all = vec![
        ResourceTemplate::new(format!("{SPREAD}{{n}}"), "lamina")
            .with_description("Os dados da lâmina n (a partir de 1).")
            .with_mime_type(JSON),
    ];
    if images {
        all.push(
            ResourceTemplate::new(format!("{SPREAD}{{n}}{PREVIEW}"), "previa")
                .with_description("A prévia da lâmina n em PNG.")
                .with_mime_type("image/png"),
        );
    }
    ListResourceTemplatesResult::with_all_items(all)
}

pub(crate) async fn read(server: &Server, uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let text = |body: String| {
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(body, uri).with_mime_type(JSON),
        ]))
    };
    match uri {
        ALBUM => text(tool_data(server, ToolCall::Summary(NoArgs {})).await?),
        SCHEMA => text(lamina_core::project_schema().map_err(|e| not_found(&e.to_string()))?),
        COMMANDS => text(commands_data().to_string()),
        _ => read_spread(server, uri).await,
    }
}

async fn read_spread(server: &Server, uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let rest = uri.strip_prefix(SPREAD).ok_or_else(|| unknown(uri))?;
    let (number, preview) = match rest.strip_suffix(PREVIEW) {
        Some(n) => (n, true),
        None => (rest, false),
    };
    let lamina: usize = number.parse().map_err(|_| unknown(uri))?;
    if !preview {
        let data = tool_data(server, view(lamina, false)).await?;
        return Ok(ReadResourceResult::new(vec![
            ResourceContents::text(data, uri).with_mime_type(JSON),
        ]));
    }
    if !server.images() {
        return Err(not_found(
            "As imagens estão desligadas nas configurações do Lamina.",
        ));
    }
    let reply = server
        .run(view(lamina, true), |_, _| {})
        .await
        .map_err(|e| not_found(&e.to_string()))?;
    let image = reply
        .images
        .into_iter()
        .next()
        .ok_or_else(|| not_found(&reply.text))?;
    let blob =
        ResourceContents::blob(convert::encode(&image.bytes), uri).with_mime_type(image.mime);
    Ok(ReadResourceResult::new(vec![blob]))
}

fn view(lamina: usize, imagem: bool) -> ToolCall {
    ToolCall::ViewSpread(ViewSpread {
        capa: false,
        lamina,
        imagem,
    })
}

async fn tool_data(server: &Server, call: ToolCall) -> Result<String, ErrorData> {
    let reply = server
        .run(call, |_, _| {})
        .await
        .map_err(|e| not_found(&e.to_string()))?;
    Ok(reply.data.to_string())
}

fn unknown(uri: &str) -> ErrorData {
    not_found(&format!("Não existe o recurso {uri}."))
}

fn not_found(message: &str) -> ErrorData {
    ErrorData::resource_not_found(message.to_owned(), None)
}
