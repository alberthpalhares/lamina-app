//! A lista das ferramentas, na ordem da spec IA (seção 3): as do IA-M1 e depois as do IA-M2.

use super::super::calls::{
    ApplyLayout, BlankPageCall, BuildAlbumCall, BuildSpread, ExportCall, FrameArg, LeaveAlbum,
    ListPhotos, MovePhoto, MoveSpread, NewAlbumCall, NewSpreadCall, NoArgs, OpenAlbum, PlacePhoto,
    RenameAlbum, RunCommand, SaveAlbum, SpreadArg, Steps, SuggestLayouts, SwapPhotos, ViewPhotos,
    ViewSpread,
};
use super::super::params::{
    AddText, CropCall, EditText, LookCall, RelinkCall, RespecCall, SaveModel, UseModel,
};
use super::{ToolInfo, info};

/// Todas as ferramentas, na ordem da spec.
pub fn tools() -> Vec<ToolInfo> {
    let mut all = first_part();
    all.extend(second_part());
    all
}

/// As ferramentas do IA-M1.
fn first_part() -> Vec<ToolInfo> {
    vec![
        info::<OpenAlbum>(
            "abrir_album",
            "Abre um álbum (.lamina) do Lamina, mesmo com fotos faltando.",
        ),
        info::<NewAlbumCall>(
            "novo_album",
            "Cria um álbum a partir de uma pasta de fotos e grava o .lamina, como o Novo álbum da \
             janela: o álbum começa em branco, com todas as fotos e nenhuma lâmina. Com montar: \
             true, já monta as lâminas pelo Auto Build (laminas só vale com montar).",
        ),
        info::<SaveAlbum>(
            "salvar_album",
            "Grava o álbum no arquivo dele ou, com caminho, salva como outro arquivo. O Lamina não \
             salva sozinho.",
        ),
        info::<RenameAlbum>(
            "renomear_album",
            "Troca o nome do álbum (o do topo da janela), como Arquivo → Renomear álbum…. O              arquivo .lamina não muda de nome; o nome novo vai para ele ao salvar.",
        ),
        info::<NoArgs>(
            "resumo_do_album",
            "Resumo do álbum aberto: especificação, lâminas, fotos usadas e sem uso, avisos, \
             histórico, alterações não salvas e os presets disponíveis.",
        ),
        info::<ViewSpread>(
            "ver_lamina",
            "Uma lâmina: quadros numerados com página, posição em mm a partir do corte, foto, DPI \
             efetivo, recorte e texto, os avisos e a prévia em imagem.",
        ),
        info::<ListPhotos>(
            "listar_fotos",
            "As fotos do álbum em ordem de captura, com número, nome, data, câmera, tamanho e as \
             lâminas onde aparecem.",
        ),
        info::<ViewPhotos>(
            "ver_fotos",
            "As miniaturas de até 24 fotos, para escolher destaques e protagonistas.",
        ),
        info::<NoArgs>(
            "pre_impressao",
            "Todos os avisos de pré-impressão (foto ampliada, quadro vazio, perto do corte, texto \
             fora da zona segura), com lâmina e quadro.",
        ),
        info::<SuggestLayouts>(
            "sugerir_layouts",
            "As opções de layout de uma lâmina (ou de uma página), com a nota de 0 a 100 e as \
             partes dela, favorita e em uso.",
        ),
        info::<ApplyLayout>(
            "aplicar_layout",
            "Aplica uma opção da última lista de sugerir_layouts da mesma lâmina e parte.",
        ),
        info::<SpreadArg>(
            "espelhar",
            "Espelha a lâmina: a esquerda troca com a direita.",
        ),
        info::<PlacePhoto>("colocar_foto", "Põe uma foto num quadro de foto."),
        info::<SwapPhotos>(
            "trocar_fotos",
            "Troca as fotos de dois quadros, da mesma lâmina ou de duas (com lamina_b).",
        ),
        info::<MovePhoto>(
            "mover_foto",
            "Leva a foto de um quadro para outra lâmina: para um quadro (enche ou troca) ou, sem              para_quadro, como uma foto a mais (o layout do destino é refeito). O quadro de origem              fica vazio.",
        ),
        info::<FrameArg>(
            "tirar_foto",
            "Tira a foto com o quadro e refaz o layout da página ou da lâmina para as fotos que \
             restaram (sem fotos, a lâmina fica em branco).",
        ),
        info::<LeaveAlbum>(
            "tirar_do_album",
            "Tira as fotos do álbum: saem da lista de fotos e de todas as lâminas, que são \
             refeitas. O arquivo continua no computador. Confirme com o fotógrafo antes, se \
             alguma foto estiver em lâmina.",
        ),
        info::<BuildSpread>(
            "montar_lamina",
            "Cria uma lâmina nova com as fotos escolhidas, no primeiro layout da lista.",
        ),
        info::<BuildAlbumCall>(
            "montar_album",
            "Monta de uma vez as fotos ainda não usadas em lâminas novas, depois da última, pelo \
             Auto Build (como Montar o álbum… da janela). Num álbum em branco, monta todas.",
        ),
        info::<NewSpreadCall>(
            "nova_lamina",
            "Cria uma lâmina vazia; com em_branco: true, as páginas dela ficam em branco.",
        ),
        info::<BlankPageCall>(
            "pagina_em_branco",
            "Marca ou desmarca uma página em branco: ela é impressa, mas não recebe fotos. As fotos \
             da página passam para a outra; sem lugar, voltam ao painel.",
        ),
        info::<SpreadArg>(
            "apagar_lamina",
            "Apaga uma lâmina; as fotos dela voltam a ficar sem uso.",
        ),
        info::<MoveSpread>("mover_lamina", "Muda uma lâmina de lugar."),
        info::<Steps>("desfazer", "Desfaz passos do histórico, como Ctrl+Z."),
        info::<Steps>("refazer", "Refaz passos desfeitos, como Ctrl+Y."),
        info::<NoArgs>(
            "historico",
            "Os passos do histórico, com rótulo, o atual e o que está salvo.",
        ),
        info::<ExportCall>(
            "exportar",
            "Exporta para a gráfica em JPEG, PDF/X-4 ou TIFF, conferindo antes. Nunca substitui \
             arquivos sem substituir: true, e nunca apaga arquivos.",
        ),
        info::<RunCommand>(
            "executar_comando",
            "Roda um comando sem diálogo pelo nome dos menus (setas de layout, favoritar, \
             rejeitar, frente e trás...) numa lâmina e num quadro.",
        ),
    ]
}

/// As ferramentas do IA-M2.
fn second_part() -> Vec<ToolInfo> {
    vec![
        info::<CropCall>(
            "recortar",
            "Recorta a foto de um quadro como o modo recorte: zoom (1 = a foto cobre o quadro, até \
             8) e o ponto da foto no centro do quadro (0 a 1).",
        ),
        info::<LookCall>(
            "aparencia",
            "Muda a aparência de uma lâmina como a aba Aparência: espaço entre fotos, fundo (da \
             lâmina ou de uma página), borda e cantos; com todas: true, leva a aparência a todas \
             as lâminas.",
        ),
        info::<AddText>(
            "adicionar_texto",
            "Cria um quadro de texto numa página da lâmina, com o texto, a fonte, o tamanho e o \
             lugar (opcionais).",
        ),
        info::<EditText>(
            "editar_texto",
            "Muda um quadro de texto como a aba Quadro: texto, fonte, tamanho, negrito, itálico, \
             cor, alinhamentos, entrelinha e lugar.",
        ),
        info::<UseModel>(
            "usar_modelo",
            "Aplica numa lâmina um modelo salvo pelo fotógrafo, se as fotos dela casam com os \
             quadros dele.",
        ),
        info::<SaveModel>(
            "salvar_modelo",
            "Salva uma lâmina como modelo do fotógrafo, para usar em outras lâminas e álbuns.",
        ),
        info::<RespecCall>(
            "trocar_especificacao",
            "Troca a especificação do álbum (um preset ou as medidas: tamanho, DPI, sangria, zona \
             segura, pontas e faixa de páginas) numa edição só; as lâminas são refeitas no \
             tamanho novo.",
        ),
        info::<RelinkCall>(
            "reencontrar_fotos",
            "Procura as fotos faltando numa pasta pela impressão digital (mesmo com outro nome) e \
             religa as que achar.",
        ),
    ]
}
