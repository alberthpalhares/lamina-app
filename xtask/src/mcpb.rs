//! Pacote do Claude Desktop (spec IA, decisão 4 e regra 14): `cargo xtask mcpb <windows|macos>
//! <binário lamina> <pasta>` monta a pasta do `.mcpb` com o manifesto, o ícone e o binário. O
//! manifesto sai do catálogo das ferramentas (IA-B24: não diverge dele); o `release.yml`
//! confere e empacota a pasta com o CLI oficial (`mcpb pack`).
//!
//! Formato: MANIFEST.md do MCPB, versão 0.3 (spike de 2026-10-19). O Claude Desktop troca
//! `${user_config.pastas}` por uma pasta em cada argumento, e a chave de ligar e desligar por
//! `true` ou `false`.

use std::fs;
use std::path::{Path, PathBuf};

use lamina_editor::agent::tools;
use serde_json::{Value, json};

/// Versão do formato do manifesto.
pub const MANIFEST_VERSION: &str = "0.3";
/// O ícone do pacote: o mesmo do programa, a partir da raiz do repositório.
const ICON: &str = "crates/lamina-app/icons/lamina-512.png";
/// O repositório público, do `Cargo.toml` (acompanha um renome).
const REPO: &str = env!("CARGO_PKG_REPOSITORY");

/// As duas configurações que o fotógrafo vê ao instalar (spec IA, seção 3).
pub const FOLDERS_TITLE: &str = "Pastas com álbuns e fotos";
pub const IMAGES_TITLE: &str = "Permitir que a IA veja as fotos";

const DESCRIPTION: &str = "Diagramador de álbuns de fotografia: a IA abre, revisa, edita e \
                           exporta os seus álbuns pelas mesmas operações do Lamina, e cada edição \
                           dela pode ser desfeita.";
const LONG_DESCRIPTION: &str = "O Lamina monta e diagrama álbuns de fotografia. Com este pacote, \
     o Claude abre os álbuns (.lamina) das pastas que você escolher, mostra os problemas de \
     impressão, troca layouts, fotos, recortes, textos e a aparência, e exporta para a gráfica. \
     Cada edição entra no histórico do Lamina como \"IA: …\" e pode ser desfeita; nada é salvo \
     sem você pedir, e nenhum arquivo é apagado.\n\n**Fotos de clientes (LGPD):** com \"Permitir \
     que a IA veja as fotos\" ligada, as prévias das lâminas e as miniaturas das fotos vão para o \
     provedor da IA (a Anthropic, no Claude). Desligue se o contrato com o cliente ou a LGPD \
     pedirem: a IA continua trabalhando só com os dados das fotos e das lâminas.";

/// Os sistemas com Claude Desktop (não há para Linux; spec IA, seção 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
}

impl Platform {
    pub fn parse(name: &str) -> Option<Platform> {
        match name {
            "windows" => Some(Platform::Windows),
            "macos" => Some(Platform::MacOs),
            _ => None,
        }
    }

    /// O nome do sistema no manifesto (o `process.platform` do Node).
    pub fn id(self) -> &'static str {
        match self {
            Platform::Windows => "win32",
            Platform::MacOs => "darwin",
        }
    }

    /// O binário `lamina` dentro do pacote.
    pub fn binary(self) -> &'static str {
        match self {
            Platform::Windows => "server/lamina.exe",
            Platform::MacOs => "server/lamina",
        }
    }

    /// O nome do pacote na release.
    pub fn file_name(self) -> &'static str {
        match self {
            Platform::Windows => "Lamina-Windows.mcpb",
            Platform::MacOs => "Lamina-macOS.mcpb",
        }
    }
}

/// O manifesto do pacote de `platform`: as ferramentas do catálogo, com nome e descrição, e as
/// duas configurações, que viram `--permitir <pastas…>` e `--imagens <true|false>`.
pub fn manifest(platform: Platform, version: &str) -> Value {
    let tools: Vec<Value> = tools()
        .iter()
        .map(|t| json!({ "name": t.name, "description": t.description }))
        .collect();
    json!({
        "manifest_version": MANIFEST_VERSION,
        "name": "lamina",
        "display_name": "Lamina",
        "version": version,
        "description": DESCRIPTION,
        "long_description": LONG_DESCRIPTION,
        "author": { "name": "Alberth Klinsmann", "url": REPO },
        "repository": { "type": "git", "url": REPO },
        "homepage": REPO,
        "support": format!("{REPO}/issues"),
        "icon": "icon.png",
        "license": "PolyForm-Strict-1.0.0",
        "keywords": ["álbum", "fotografia", "diagramação", "impressão"],
        "server": {
            "type": "binary",
            "entry_point": platform.binary(),
            "mcp_config": {
                "command": format!("${{__dirname}}/{}", platform.binary()),
                "args": [
                    "mcp",
                    "--imagens", "${user_config.imagens}",
                    "--permitir", "${user_config.pastas}",
                ],
            },
        },
        "tools": tools,
        // Os prompts prontos vêm do servidor (com os argumentos do fotógrafo).
        "prompts_generated": true,
        "compatibility": { "platforms": [platform.id()] },
        "user_config": {
            "pastas": {
                "type": "directory",
                "title": FOLDERS_TITLE,
                "description": "Onde ficam os álbuns (.lamina) e as fotos. A IA só abre, cria, \
                                grava e exporta dentro destas pastas.",
                "multiple": true,
                "required": true,
            },
            "imagens": {
                "type": "boolean",
                "title": IMAGES_TITLE,
                "description": "Ligada, as prévias das lâminas e as miniaturas das fotos vão para \
                                o provedor da IA (fotos de clientes; LGPD). Desligada, a IA só \
                                recebe os dados das fotos e das lâminas.",
                "default": true,
            },
        },
    })
}

pub fn run(args: &[String]) -> bool {
    let usage = "uso: cargo xtask mcpb <windows|macos> <binário lamina> <pasta>";
    let [platform, binary, folder] = args else {
        eprintln!("{usage}");
        return false;
    };
    let Some(platform) = Platform::parse(platform) else {
        eprintln!("{usage}");
        return false;
    };
    match assemble(platform, Path::new(binary), Path::new(folder)) {
        Ok(()) => {
            println!(
                "  pasta do {} pronta em {folder} (versão {})",
                platform.file_name(),
                env!("CARGO_PKG_VERSION")
            );
            true
        }
        Err(e) => {
            eprintln!("  ✘ {e}");
            false
        }
    }
}

/// A pasta do pacote: `manifest.json`, `icon.png` e `server/lamina(.exe)`.
fn assemble(platform: Platform, binary: &Path, folder: &Path) -> Result<(), String> {
    let fail = |what: &str, e: &dyn std::fmt::Display| format!("{what}: {e}");
    fs::create_dir_all(folder.join("server")).map_err(|e| fail("criar a pasta", &e))?;
    fs::copy(binary, folder.join(platform.binary())).map_err(|e| fail("copiar o binário", &e))?;
    fs::copy(icon(), folder.join("icon.png")).map_err(|e| fail("copiar o ícone", &e))?;
    let manifest = manifest(platform, env!("CARGO_PKG_VERSION"));
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| fail("manifesto", &e))?;
    fs::write(folder.join("manifest.json"), text + "\n").map_err(|e| fail("gravar", &e))
}

/// O ícone, achado a partir da pasta do `xtask` (vale de qualquer pasta atual).
fn icon() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(ICON)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IA-B24: o manifesto lista as ferramentas do catálogo, com os mesmos nomes e descrições,
    /// e as duas configurações da spec, ligadas aos argumentos do `lamina mcp`.
    #[test]
    fn ia_b24_manifesto_igual_ao_catalogo() {
        for platform in [Platform::Windows, Platform::MacOs] {
            let m = manifest(platform, "1.2.3");
            let listed: Vec<(&str, &str)> = m["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    (
                        t["name"].as_str().unwrap(),
                        t["description"].as_str().unwrap(),
                    )
                })
                .collect();
            let catalog: Vec<(&str, &str)> =
                tools().iter().map(|t| (t.name, t.description)).collect();
            assert_eq!(listed, catalog);
            let config = &m["user_config"];
            assert_eq!(config["pastas"]["title"], FOLDERS_TITLE);
            assert_eq!(config["pastas"]["type"], "directory");
            assert_eq!(config["pastas"]["multiple"], true);
            assert_eq!(config["imagens"]["title"], IMAGES_TITLE);
            assert_eq!(config["imagens"]["type"], "boolean");
            assert_eq!(config["imagens"]["default"], true);
            // A lista de pastas só se expande num argumento que é só a variável.
            let args = &m["server"]["mcp_config"]["args"];
            assert_eq!(
                *args,
                json!([
                    "mcp",
                    "--imagens",
                    "${user_config.imagens}",
                    "--permitir",
                    "${user_config.pastas}"
                ])
            );
            assert_eq!(m["compatibility"]["platforms"], json!([platform.id()]));
            assert_eq!(m["server"]["entry_point"], platform.binary());
            assert_eq!(m["version"], "1.2.3");
            assert_eq!(m["manifest_version"], MANIFEST_VERSION);
        }
    }

    #[test]
    fn a_pasta_do_pacote_tem_manifesto_icone_e_binario() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("lamina.exe");
        fs::write(&binary, b"binario").unwrap();
        let folder = dir.path().join("pacote");
        assemble(Platform::Windows, &binary, &folder).unwrap();
        assert_eq!(
            fs::read(folder.join("server/lamina.exe")).unwrap(),
            b"binario"
        );
        assert!(folder.join("icon.png").is_file());
        let text = fs::read_to_string(folder.join("manifest.json")).unwrap();
        let m: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(m, manifest(Platform::Windows, env!("CARGO_PKG_VERSION")));
        assert_eq!(Platform::parse("linux"), None);
    }
}
