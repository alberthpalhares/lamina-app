//! Os parâmetros das ferramentas do IA-M2 (spec IA, seção 3): recorte, aparência, texto, modelos,
//! especificação e reencontrar fotos. Mesmas regras de `calls.rs`: nomes em português, sem
//! acento, e lâmina e quadro contados a partir de 1.

use schemars::JsonSchema;
use serde::Deserialize;

use super::calls::PartArg;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CropCall {
    pub lamina: usize,
    /// Número do quadro de foto, como em ver_lamina.
    pub quadro: usize,
    /// 1 = a foto cobre o quadro inteiro (o encaixe); até 8. Sem ele, fica o de agora.
    pub zoom: Option<f64>,
    /// Ponto da foto no centro do quadro, de 0 (esquerda) a 1 (direita). Sem ele, fica o de agora.
    pub centro_x: Option<f64>,
    /// Ponto da foto no centro do quadro, de 0 (alto) a 1 (baixo). Sem ele, fica o de agora.
    pub centro_y: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LookCall {
    #[serde(default)]
    pub lamina: usize,
    /// Na capa em vez de numa lâmina.
    #[serde(default)]
    pub capa: bool,
    /// Depois das mudanças, levar a aparência desta lâmina para todas, como "Aplicar a todas".
    #[serde(default)]
    pub todas: bool,
    /// Espaço entre as fotos, de 0 a 20 mm; o desenho da lâmina se mantém.
    pub espaco_mm: Option<f64>,
    /// Cor do fundo, como "#F5F0E6" ou um nome da paleta ("Branco", "Marfim", "Creme"...).
    pub fundo: Option<String>,
    /// Onde vai o fundo: a lâmina inteira (padrão) ou só uma página.
    #[serde(default)]
    pub pagina: PartArg,
    /// Borda das fotos da lâmina, de 0 (sem borda) a 5 mm.
    pub borda_mm: Option<f64>,
    /// Cor da borda, como "#FFFFFF" ou um nome da paleta.
    pub cor_borda: Option<String>,
    /// Cantos arredondados das fotos, de 0 a 10 mm.
    pub cantos_mm: Option<f64>,
    /// Opacidade das fotos, de 0 (some) a 100 (opaca, o normal), em %.
    pub opacidade: Option<i64>,
    /// Só este quadro de foto (a partir de 1) recebe borda, cantos e opacidade; sem ele, todas as
    /// fotos da lâmina.
    pub quadro: Option<usize>,
}

/// A página de um texto novo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PageArg {
    Esquerda,
    Direita,
}

/// Onde fica o quadro, em mm a partir do canto do corte (como posicao_mm de ver_lamina).
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceMm {
    pub x: f64,
    pub y: f64,
    /// Sem ela, a largura de agora.
    pub largura: Option<f64>,
    /// Sem ela, a altura de agora.
    pub altura: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddText {
    #[serde(default)]
    pub lamina: usize,
    /// Na capa em vez de numa lâmina.
    #[serde(default)]
    pub capa: bool,
    pub pagina: PageArg,
    pub texto: String,
    /// Uma das fontes do Lamina (lista no erro, se não existir); padrão Inter.
    pub fonte: Option<String>,
    /// Tamanho em pontos, de 4 a 300 (padrão 24).
    pub tamanho: Option<f64>,
    /// Sem ela, o texto fica no centro da página.
    pub posicao_mm: Option<PlaceMm>,
}

/// Alinhamento das linhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlignArg {
    Esquerda,
    Centro,
    Direita,
}

/// Onde o bloco de linhas fica na altura do quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VAlignArg {
    Topo,
    Meio,
    Base,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditText {
    #[serde(default)]
    pub lamina: usize,
    /// Na capa em vez de numa lâmina.
    #[serde(default)]
    pub capa: bool,
    /// Número do quadro de texto, como em ver_lamina.
    pub quadro: usize,
    pub texto: Option<String>,
    pub fonte: Option<String>,
    /// Em pontos, de 4 a 300.
    pub tamanho: Option<f64>,
    pub negrito: Option<bool>,
    pub italico: Option<bool>,
    /// Como "#1A1A1A" ou um nome da paleta.
    pub cor: Option<String>,
    pub alinhamento: Option<AlignArg>,
    pub vertical: Option<VAlignArg>,
    /// Em vezes o tamanho, de 0,8 a 3.
    pub entrelinha: Option<f64>,
    /// Novo lugar do quadro, em mm a partir do canto do corte.
    pub posicao_mm: Option<PlaceMm>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseModel {
    pub lamina: usize,
    /// Nome do modelo, como o fotógrafo salvou.
    pub nome: String,
    /// A versão espelhada do modelo; sem ele, a normal (ou a espelhada, se só ela casa).
    pub espelhado: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveModel {
    pub lamina: usize,
    pub nome: String,
    /// Substituir um modelo que já tem esse nome.
    #[serde(default)]
    pub substituir: bool,
}

/// As medidas da janela Especificação; o que faltar fica como está (ou como no preset).
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RespecCall {
    /// Nome de um preset, como "25x25cm" ou "25x20cm (ProAlbuns)" (lista em resumo_do_album).
    pub preset: Option<String>,
    pub largura_cm: Option<f64>,
    pub altura_cm: Option<f64>,
    pub dpi: Option<u32>,
    pub sangria_mm: Option<f64>,
    pub zona_segura_mm: Option<f64>,
    /// Zona segura na lombada, se for diferente da zona segura.
    pub lombada_mm: Option<f64>,
    /// O álbum começa com uma página só (a primeira lâmina tem só a direita).
    pub inicia_com_pagina: Option<bool>,
    /// O álbum termina com uma página só.
    pub termina_com_pagina: Option<bool>,
    /// Faixa de páginas do laboratório (aviso quando o álbum sai dela).
    pub paginas_min: Option<u32>,
    pub paginas_max: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelinkCall {
    /// Pasta onde procurar as fotos faltando (as subpastas entram também).
    pub pasta: String,
}
