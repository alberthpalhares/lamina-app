//! Os parâmetros de cada ferramenta (spec IA, seção 3). Os nomes dos campos são os que a IA vê,
//! em português e sem acento, como as opções da linha de comando. Lâmina, quadro, foto e opção
//! contam a partir de 1, como na tela.

use schemars::JsonSchema;
use serde::Deserialize;

/// Sem parâmetros.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoArgs {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenAlbum {
    /// Caminho do arquivo .lamina.
    pub caminho: String,
    /// Abrir mesmo que o álbum atual tenha alterações não salvas (elas se perdem).
    #[serde(default)]
    pub descartar: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewAlbumCall {
    /// Pasta com as fotos (as subpastas entram também).
    pub pasta_fotos: String,
    /// Onde gravar o álbum novo (arquivo .lamina).
    pub salvar_em: String,
    /// Nome do álbum (o que aparece no topo da janela). Sem ele, o nome do arquivo salvar_em.
    pub nome: Option<String>,
    /// Nome de um preset, como "30x30cm" ou "25x20cm (ProAlbuns)" (lista em resumo_do_album). Sem preset, a
    /// especificação padrão do Novo álbum.
    pub preset: Option<String>,
    /// Número de lâminas; só vale com montar: true. Sem ele, a sugestão do Lamina (cerca de 4
    /// fotos por lâmina).
    pub laminas: Option<usize>,
    /// Montar as lâminas já, como a caixa "Montar as lâminas agora" do Novo álbum. Sem ele, o
    /// álbum começa em branco, só com as fotos; use montar_album depois.
    #[serde(default)]
    pub montar: bool,
    /// Criar mesmo que o álbum atual tenha alterações não salvas (elas se perdem).
    #[serde(default)]
    pub descartar: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameAlbum {
    /// O nome novo do álbum (até 120 letras). O arquivo .lamina não muda de nome.
    pub nome: String,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveAlbum {
    /// Salvar como este arquivo .lamina; sem ele, o arquivo do álbum.
    pub caminho: Option<String>,
    /// Gravar por cima de outro arquivo que já existe nesse caminho.
    #[serde(default)]
    pub substituir: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewSpread {
    /// Número da lâmina, a partir de 1 (use capa: true para ver a capa).
    #[serde(default)]
    pub lamina: usize,
    /// Ver a capa em vez de uma lâmina.
    #[serde(default)]
    pub capa: bool,
    /// Mandar a prévia da lâmina em PNG (padrão: sim, se as imagens estiverem ligadas).
    #[serde(default = "yes")]
    pub imagem: bool,
}

fn yes() -> bool {
    true
}

/// Quais fotos listar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhotoFilter {
    #[default]
    Todas,
    Usadas,
    SemUso,
    Faltando,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListPhotos {
    #[serde(default)]
    pub filtro: PhotoFilter,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewPhotos {
    /// Números das fotos (até 24), como em listar_fotos.
    pub fotos: Vec<u64>,
}

/// Que parte da lâmina troca de layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PartArg {
    #[default]
    Lamina,
    Esquerda,
    Direita,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuggestLayouts {
    pub lamina: usize,
    /// A lâmina inteira (padrão) ou só uma página.
    #[serde(default)]
    pub parte: PartArg,
    /// Quantas opções mandar (padrão 6, até 24).
    pub quantas: Option<usize>,
    /// Mandar a prévia das 6 primeiras opções.
    #[serde(default)]
    pub imagens: bool,
    /// nota (padrão: a ordem da janela) ou preenchimento (da maior área ocupada para a menor).
    #[serde(default)]
    pub ordem: OrderArg,
    /// Tolerância de corte, em %: só as opções em que nenhuma foto perde mais do que isso.
    pub corte_maximo: Option<f64>,
}

/// A ordem das opções em sugerir_layouts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderArg {
    #[default]
    Nota,
    Preenchimento,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyLayout {
    pub lamina: usize,
    #[serde(default)]
    pub parte: PartArg,
    /// Número da opção na última lista de sugerir_layouts desta lâmina e parte.
    pub opcao: usize,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpreadArg {
    pub lamina: usize,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacePhoto {
    #[serde(default)]
    pub lamina: usize,
    /// Na capa em vez de numa lâmina.
    #[serde(default)]
    pub capa: bool,
    /// Número do quadro na lâmina, como em ver_lamina.
    pub quadro: usize,
    /// Número da foto, como em listar_fotos.
    pub foto: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SwapPhotos {
    pub lamina: usize,
    pub quadro_a: usize,
    pub quadro_b: usize,
    /// A lâmina do `quadro_b`, quando é outra (padrão: a mesma `lamina`).
    pub lamina_b: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MovePhoto {
    /// A lâmina e o quadro de onde a foto sai (o quadro fica vazio).
    pub lamina: usize,
    pub quadro: usize,
    /// A lâmina de destino.
    pub para_lamina: usize,
    /// O quadro de destino: vazio, a foto vai para ele; com foto, as duas trocam. Sem ele, a foto
    /// entra como uma a mais e o layout do destino é refeito.
    pub para_quadro: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameArg {
    pub lamina: usize,
    pub quadro: usize,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildSpread {
    /// Arranjar as fotos na capa, em vez de criar uma lâmina.
    #[serde(default)]
    pub capa: bool,
    /// Números das fotos da lâmina nova, na ordem.
    pub fotos: Vec<u64>,
    /// Posição da lâmina nova (padrão: no fim).
    pub posicao: Option<usize>,
}

/// Fotos que saem do álbum (spec TF, regra 8).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeaveAlbum {
    /// Números das fotos (como em listar_fotos) que saem do álbum e das lâminas.
    pub fotos: Vec<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewSpreadCall {
    /// Posição da lâmina nova (padrão: no fim).
    pub posicao: Option<usize>,
    /// A lâmina nasce com as páginas em branco (padrão: não).
    #[serde(default)]
    pub em_branco: bool,
}

/// Medidas da capa, em centímetros; o que faltar fica como está ou com o padrão (spec CA,
/// regra 15).
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoverCall {
    /// Largura de um lado da capa fechada, em cm.
    pub face_largura_cm: Option<f64>,
    /// Altura de um lado da capa fechada, em cm.
    pub face_altura_cm: Option<f64>,
    /// Lombada fixa, em cm. Sem ela, a lombada é calculada pelas páginas do álbum.
    pub lombada_cm: Option<f64>,
    /// Espessura de cada folha, em mm, para a lombada calculada (padrão 1).
    pub folha_mm: Option<f64>,
    /// A parte que vira para dentro da capa dura, em cm (padrão 1,5).
    pub dobra_cm: Option<f64>,
    /// Zona segura, em cm (padrão 1).
    pub zona_segura_cm: Option<f64>,
}

/// Lado de uma lâmina, para a página em branco.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlankSide {
    Esquerda,
    Direita,
    Ambas,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BlankPageCall {
    pub lamina: usize,
    pub lado: BlankSide,
    pub em_branco: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveSpread {
    pub lamina: usize,
    /// Nova posição da lâmina.
    pub para: usize,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    /// Quantos passos (padrão 1).
    pub passos: Option<usize>,
}

pub use super::export::{ColorArg, CoverArg, ExportCall, FormatArg};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunCommand {
    /// Nome estável do comando, como "layout.proximo" (lista em lamina://comandos).
    pub comando: String,
    /// Lâmina onde rodar (padrão: a atual).
    pub lamina: Option<usize>,
    /// Quadro selecionado antes de rodar (para "editar.tirar-foto", por exemplo).
    pub quadro: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildAlbumCall {
    /// Número de lâminas novas; sem ele, a sugestão do Lamina (cerca de 4 fotos por lâmina).
    pub laminas: Option<usize>,
}
