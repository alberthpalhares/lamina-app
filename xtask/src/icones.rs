//! Trava dos ícones e da acessibilidade (spec DF, regras 7 e 8):
//! - nenhum caractere fazendo papel de ícone em texto de tela (todo ícone vem do global `Icons`);
//! - os SVGs do Lucide em `icons/lucide/` vão com a licença ISC ao lado;
//! - todo componente clicável de `ui/` tem `accessible-role` e `accessible-label`, menos as superfícies.

use std::fs;
use std::path::Path;

/// Caracteres que, sozinhos ou abrindo um rótulo ("+ Fotos"), fazem papel de ícone.
const ICON_CHARS: &[char] = &['▾', '▸', '★', '☆', '×', '✥', '▦', '✓', '−', '+'];

/// Superfícies que têm `TouchArea` para gestos, mas não são um controle com nome: a lâmina, as
/// grades e os fundos. Cada item daqui é uma decisão: um botão nunca entra nesta lista.
const SURFACES: &[&str] = &[
    "AppWindow",
    "ColorPicker",
    "ContextMenu",
    "Gallery",
    "GridDivider",
    "HistoryPanel",
    "Knob",
    "LayoutBand",
    "ModelsDialog",
    "NewsDialog",
    "OrganizeBar",
    "OrganizeTools",
    "PanelDivider",
    "PhotoGrid",
    "PresetsDialog",
    "ShortcutsDialog",
    "SpreadBar",
    "SpreadCanvas",
    "SpreadRoll",
    "SpreadStrip",
    "Stage",
    "TimelineStrip",
];

/// Os textos entre aspas de uma linha `text:` que usam um caractere como ícone. Títulos de menu
/// (`title:`) ficam de fora: o "✓ " de item ligado no menu do sistema é a marca dele.
pub fn icon_texts(line: &str) -> Vec<String> {
    let code = line.split("//").next().unwrap_or("");
    if !code.contains("text:") {
        return Vec::new();
    }
    literals(code)
        .into_iter()
        // Colado a um valor ("×" + usos) é sinal de multiplicação, não ícone.
        .filter(|s| !code.contains(&format!("\"{s}\" +")))
        .filter(|s| {
            let t = s.trim();
            let mut chars = t.chars();
            let first = chars.next();
            let rest: String = chars.collect();
            first.is_some_and(|c| ICON_CHARS.contains(&c))
                && (rest.is_empty()
                    || rest.starts_with(' ')
                    || t.chars().all(|c| ICON_CHARS.contains(&c)))
        })
        .collect()
}

/// Os textos entre aspas de `code`, respeitando as aspas escapadas (`\"`).
fn literals(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let mut escaped = false;
    for c in code.chars() {
        match (&mut current, c) {
            (Some(s), _) if escaped => {
                s.push('\\');
                s.push(c);
                escaped = false;
            }
            (Some(_), '\\') => escaped = true,
            (Some(s), '"') => {
                out.push(std::mem::take(s));
                current = None;
            }
            (Some(s), _) => s.push(c),
            (None, '"') => current = Some(String::new()),
            (None, _) => {}
        }
    }
    out
}

/// Os componentes de `text` que têm `TouchArea` e não declaram papel e nome acessíveis.
pub fn without_accessibility(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in text.split("component ").skip(1) {
        let name: String = block
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let clickable = block.contains("TouchArea");
        let named = block.contains("accessible-role") && block.contains("accessible-label");
        if clickable && !named {
            out.push(name);
        }
    }
    out
}

/// A pasta de ícones do Lucide tem SVG mas não tem a licença ISC.
pub fn lucide_without_license(dir: &Path) -> bool {
    let has_svg = fs::read_dir(dir).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "svg"))
    });
    let license = fs::read_to_string(dir.join("LICENSE")).unwrap_or_default();
    has_svg && !license.contains("ISC")
}

/// Todos os problemas de `ui` (telas) e `icons` (pasta de ícones do app).
pub fn problems(ui: &Path, icons: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in slint_files(ui) {
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (n, line) in text.lines().enumerate() {
            for t in icon_texts(line) {
                out.push(format!(
                    "{}:{}: \"{t}\" usa caractere como ícone",
                    path.display(),
                    n + 1
                ));
            }
        }
        for c in without_accessibility(&text) {
            if !SURFACES.contains(&c.as_str()) {
                out.push(format!("{}: {c} sem accessible-role/label", path.display()));
            }
        }
    }
    if lucide_without_license(&icons.join("lucide")) {
        out.push(format!(
            "{}: falta a licença ISC (LICENSE)",
            icons.join("lucide").display()
        ));
    }
    out
}

fn slint_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            out.extend(slint_files(&path));
        } else if path.extension().is_some_and(|e| e == "slint") {
            out.push(path);
        }
    }
    out.sort();
    out
}

pub fn run() -> bool {
    let app = Path::new("crates/lamina-app");
    let found = problems(&app.join("ui"), &app.join("icons"));
    for f in &found {
        eprintln!("  {f}");
    }
    if found.is_empty() {
        println!("  ícones e acessibilidade ok");
    }
    found.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df_b9_caractere_fazendo_papel_de_icone_reprova() {
        let dir = tempfile::tempdir().unwrap();
        let tela = dir.path().join("layouts.slint");
        fs::write(
            &tela,
            "Text { text: \"★\"; }\nText { text: \"← → trocam o layout\"; }\n",
        )
        .unwrap();
        assert_eq!(
            problems(dir.path(), dir.path()),
            [format!(
                "{}:1: \"★\" usa caractere como ícone",
                tela.display()
            )]
        );
        assert_eq!(icon_texts("text: \"+ Fotos\";"), ["+ Fotos"]);
        assert!(icon_texts("text: \"+5 mm\" + \"3 × 4\";").is_empty());
        assert!(icon_texts("text: \"Excluir \\\"\" + root.name + \"\\\"?\";").is_empty());
        assert!(icon_texts("title: (on ? \"✓ \" : \"\") + \"Guias\";").is_empty());
        assert!(icon_texts("text: \"×\" + root.data.uses;").is_empty());
    }

    #[test]
    fn df_b10_icone_lucide_sem_licenca_reprova() {
        let dir = tempfile::tempdir().unwrap();
        let lucide = dir.path().join("lucide");
        fs::create_dir_all(&lucide).unwrap();
        fs::write(lucide.join("house.svg"), "<svg/>").unwrap();
        assert!(lucide_without_license(&lucide));
        fs::write(
            lucide.join("LICENSE"),
            "ISC License\nCopyright (c) Lucide Contributors",
        )
        .unwrap();
        assert!(!lucide_without_license(&lucide));
    }

    #[test]
    fn df_b11_componente_clicavel_sem_nome_acessivel_reprova() {
        let sem = "export component BotaoIcone { TouchArea { } }\n";
        let com = "export component Botao { accessible-role: button; \
                   accessible-label: root.text; TouchArea { } }\n";
        assert_eq!(without_accessibility(sem), ["BotaoIcone"]);
        assert!(without_accessibility(com).is_empty());
        assert!(without_accessibility("component Titulo { Text { } }").is_empty());
    }
}
