//! A lista das fontes da aba Quadro (spec FS): as 6 do Lamina, as do computador, a busca, a trava
//! e as prévias. A leitura das fontes do computador mora no `lamina-render`, que só lê quando
//! precisa (regra 11).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use lamina_render::{
    FAMILIES, font_preview, font_styles, installed_names_ready, request_installed_names,
};

/// "Procurando…", enquanto os nomes não chegaram (regra 1).
pub const LOADING: &str = "Procurando as fontes do computador…";
/// O computador sem nenhuma fonte instalada (seção 6).
pub const NO_FONTS: &str = "Nenhuma fonte instalada encontrada";
/// O fim do nome de uma fonte que o álbum usa e o computador não tem (regra 5).
pub const MISSING: &str = " (não está neste computador)";
/// Os títulos das duas partes da lista.
pub const LAMINA: &str = "Lamina";
pub const COMPUTER: &str = "Neste computador";
/// O tamanho das prévias, em pixels: uma linha baixa da lista.
pub const PREVIEW_PX: (u32, u32) = (240, 28);

static SYSTEM_FONTS: AtomicBool = AtomicBool::new(true);

/// A trava das fontes do computador, num ponto só (regra 7). Até a versão de apoiador (spec VD),
/// aberta; depois, `License::allows(Feature::SystemFonts)`. Ela só pega a escolha: o desenho de
/// um texto que já usa uma fonte do computador nunca é travado.
pub fn system_fonts_allowed() -> bool {
    SYSTEM_FONTS.load(Ordering::Relaxed)
}

/// Abre ou fecha a trava (a VD e os testes).
pub fn set_system_fonts_allowed(open: bool) {
    SYSTEM_FONTS.store(open, Ordering::Relaxed);
}

/// Uma linha da lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontRow {
    /// O título de uma parte: [`LAMINA`] ou [`COMPUTER`].
    Header(&'static str),
    /// Uma fonte. `current` é a do texto; `missing`, a que o álbum usa e o computador não tem.
    Font {
        name: String,
        current: bool,
        missing: bool,
    },
    /// Um aviso no lugar das fontes: "Procurando…", "Nenhuma fonte com …".
    Note(String),
}

impl FontRow {
    /// O texto da linha, como aparece na lista e no campo Fonte.
    pub fn label(&self) -> String {
        match self {
            FontRow::Header(title) => (*title).to_owned(),
            FontRow::Font { name, missing, .. } if *missing => format!("{name}{MISSING}"),
            FontRow::Font { name, .. } => name.clone(),
            FontRow::Note(note) => note.clone(),
        }
    }
}

/// Minúsculas e sem acentos, para a busca (regra 4).
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

/// Os nomes de `names` com o trecho `query` (já passado por [`fold`]).
pub fn matching<'a>(names: &'a [String], query: &str) -> Vec<&'a String> {
    names.iter().filter(|n| fold(n).contains(query)).collect()
}

fn font(name: &str, current: &str) -> FontRow {
    FontRow::Font {
        name: name.to_owned(),
        current: name == current,
        missing: false,
    }
}

/// As linhas da lista para o texto na fonte `current`, com a busca `query`. Pedir a lista começa
/// a ler os nomes do computador em segundo plano, se ainda não leu (regras 1 e 11).
pub fn font_rows(current: &str, query: &str) -> Vec<FontRow> {
    let q = fold(query.trim());
    let mut rows = Vec::new();
    if q.is_empty() && !current.is_empty() && font_styles(current).is_none() {
        rows.push(FontRow::Font {
            name: current.to_owned(),
            current: true,
            missing: true,
        });
    }
    let lamina: Vec<FontRow> = FAMILIES
        .iter()
        .filter(|f| fold(f.name).contains(&q))
        .map(|f| font(f.name, current))
        .collect();
    if !lamina.is_empty() {
        rows.push(FontRow::Header(LAMINA));
        rows.extend(lamina);
    }
    if system_fonts_allowed() {
        computer_rows(current, &q, &mut rows);
    }
    if !q.is_empty() && !rows.iter().any(|r| matches!(r, FontRow::Font { .. })) {
        rows.retain(|r| matches!(r, FontRow::Note(n) if n == LOADING));
        rows.push(FontRow::Note(format!(
            "Nenhuma fonte com “{}”",
            query.trim()
        )));
    }
    rows
}

/// A parte "Neste computador": as que têm o trecho, ou o aviso de que ainda está lendo.
fn computer_rows(current: &str, q: &str, rows: &mut Vec<FontRow>) {
    let Some(names) = installed_names_ready() else {
        request_installed_names();
        rows.push(FontRow::Header(COMPUTER));
        rows.push(FontRow::Note(LOADING.to_owned()));
        return;
    };
    if names.is_empty() {
        rows.push(FontRow::Header(COMPUTER));
        rows.push(FontRow::Note(NO_FONTS.to_owned()));
        return;
    }
    let found = matching(&names, q);
    if !found.is_empty() {
        rows.push(FontRow::Header(COMPUTER));
        rows.extend(found.into_iter().map(|n| font(n, current)));
    }
}

/// Os nomes do computador já foram lidos (a janela refaz a lista quando chegam, FS-B11).
pub fn names_ready() -> bool {
    installed_names_ready().is_some()
}

/// Uma prévia: o nome da fonte escrito nela mesma, em RGBA com as cores pré-multiplicadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A prévia de uma fonte, ou nenhuma para a que não está aqui ou não desenha (regra 11). Leva
/// alguns milissegundos: a janela chama fora da linha dela.
pub fn preview(name: &str) -> Option<Preview> {
    font_preview(name, PREVIEW_PX).map(|img| Preview {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

/// As prévias da lista aberta (regra 11): só das linhas que aparecem, pedidas uma vez cada,
/// feitas fora da linha da janela e soltas ao fechar a lista.
#[derive(Debug, Default)]
pub struct FontPreviews {
    asked: HashSet<String>,
    images: HashMap<String, Preview>,
}

impl FontPreviews {
    /// Das fontes `visible` (as linhas na tela), as que ainda não foram pedidas. Quem chama faz
    /// as prévias em segundo plano e entrega com [`FontPreviews::put`].
    pub fn wanted(&mut self, visible: &[String]) -> Vec<String> {
        visible
            .iter()
            .filter(|n| self.asked.insert((*n).clone()))
            .cloned()
            .collect()
    }

    /// A prévia pronta (ou nenhuma, para uma fonte que não desenha).
    pub fn put(&mut self, name: String, image: Option<Preview>) {
        if let Some(image) = image.filter(|_| self.asked.contains(&name)) {
            self.images.insert(name, image);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Preview> {
        self.images.get(name)
    }

    /// Quantas prévias foram pedidas desde que a lista abriu.
    pub fn asked(&self) -> usize {
        self.asked.len()
    }

    /// A lista fechou: solta tudo.
    pub fn clear(&mut self) {
        self.asked.clear();
        self.images.clear();
    }
}

/// As fontes dos textos do álbum que não são do Lamina nem estão neste computador, na ordem em
/// que aparecem, sem repetir (regra 5). Um álbum só com as 6 do Lamina não lê nada (FS-B13).
pub fn missing_fonts(project: &lamina_core::Project) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    let texts = project.all_content().flat_map(|s| &s.frames);
    for frame in texts {
        if let lamina_core::FrameContent::Text(t) = &frame.content
            && !missing.contains(&t.font)
            && font_styles(&t.font).is_none()
        {
            missing.push(t.font.clone());
        }
    }
    missing
}

/// O aviso ao abrir um álbum com fontes que faltam (regra 5), ou nenhum.
pub fn missing_fonts_notice(missing: &[String]) -> Option<String> {
    (!missing.is_empty()).then(|| {
        format!(
            "Este álbum usa fontes que não estão neste computador: {}. Os textos dessas fontes \
             aparecem em {}. Instale as fontes e abra o álbum de novo.",
            missing.join(", "),
            lamina_core::DEFAULT_FONT
        )
    })
}
