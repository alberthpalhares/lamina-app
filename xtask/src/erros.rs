//! Trava dos erros humanos (spec EH, regra 9; AGENTS.md, regra 15): nenhum erro chega à janela
//! sem a frase de fotógrafo. Reprova:
//! - `_ =>` no módulo `human/` (o `match` tem que nomear toda variante);
//! - um tipo de erro (`enum` com `derive(... Error)`) sem `impl Human for` no `human/`;
//! - um erro virando texto (`e.to_string()`, `"{e}"`…) no código da janela, fora de uma linha
//!   com a marca `// texto já humano`.

use std::fs;
use std::path::{Path, PathBuf};

const HUMAN_DIR: &str = "crates/lamina-editor/src/human";
const WINDOW_DIRS: [&str; 2] = ["crates/lamina-app/src", "crates/lamina-editor/src/editor"];
/// Erros que nunca chegam à janela, com o motivo.
const EXEMPT: [(&str, &str); 3] = [
    ("ToolError", "só o controle por IA vê (spec IA, regra 13)"),
    ("McpError", "só o servidor de IA vê, na linha de comando"),
    (
        "FilmError",
        "só a filmagem vê, na linha de comando (spec FI)",
    ),
];
const ALLOW: &str = "// texto já humano";
const TO_TEXT: [&str; 7] = [
    "e.to_string()",
    "err.to_string()",
    "error.to_string()",
    "\"{e}\"",
    "\"{err}\"",
    "{e:#}",
    "{error}",
];

/// Linhas (1-based) com `_ =>` fora de comentários.
pub fn wildcards(text: &str) -> Vec<usize> {
    lines_with(text, |l| {
        !l.trim_start().starts_with("//") && l.contains("_ =>")
    })
}

/// Linhas (1-based) em que um erro vira texto, sem a marca de exceção.
pub fn raw_texts(text: &str) -> Vec<usize> {
    lines_with(text, |l| {
        !l.trim_start().starts_with("//")
            && TO_TEXT.iter().any(|p| l.contains(p))
            && !l.contains(ALLOW)
    })
}

fn lines_with(text: &str, hit: impl Fn(&str) -> bool) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| hit(l))
        .map(|(i, _)| i + 1)
        .collect()
}

/// Os nomes dos `enum` que derivam `Error` (o `derive` pode quebrar a linha).
pub fn error_enums(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut derive = String::new();
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("#[derive(") {
            inside = true;
            derive.clear();
        }
        if inside {
            derive.push_str(line);
            if line.contains(")]") {
                inside = false;
            }
            continue;
        }
        if let Some(rest) = line
            .split("enum ")
            .nth(1)
            .filter(|_| line.contains("enum "))
            && is_error(&derive)
        {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            names.push(name);
        }
        if !line.starts_with("#[") {
            derive.clear();
        }
    }
    names
}

fn is_error(derive: &str) -> bool {
    derive
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| word == "Error")
}

/// Os tipos de erro sem `impl Human for <Nome>` e fora das exceções.
pub fn untranslated(enums: &[String], human: &str) -> Vec<String> {
    enums
        .iter()
        .filter(|name| !EXEMPT.iter().any(|(exempt, _)| exempt == name))
        .filter(|name| !human.contains(&format!("impl Human for {name} ")))
        .cloned()
        .collect()
}

fn rust_files(dir: &Path, skip_tests: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if skip_tests && (name == "tests" || name == "tests.rs" || name == "testutil.rs") {
            continue;
        }
        if path.is_dir() {
            rust_files(&path, skip_tests, out);
        } else if name.ends_with(".rs") {
            out.push(path);
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Os problemas do repositório em `root`.
pub fn problems(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut human_files = Vec::new();
    rust_files(&root.join(HUMAN_DIR), false, &mut human_files);
    let human: String = human_files.iter().map(|p| read(p)).collect();
    for path in &human_files {
        for line in wildcards(&read(path)) {
            found.push(format!(
                "{}:{line}: `_ =>` no módulo human; nomeie cada variante",
                path.display()
            ));
        }
    }
    let mut sources = Vec::new();
    rust_files(&root.join("crates"), true, &mut sources);
    for path in &sources {
        for name in untranslated(&error_enums(&read(path)), &human) {
            found.push(format!(
                "{}: o erro `{name}` não tem frase (`impl Human for {name}`)",
                path.display()
            ));
        }
    }
    for dir in WINDOW_DIRS {
        let mut window = Vec::new();
        rust_files(&root.join(dir), true, &mut window);
        for path in &window {
            for line in raw_texts(&read(path)) {
                found.push(format!(
                    "{}:{line}: erro virando texto na janela; use a mensagem humana ou marque `{ALLOW}`",
                    path.display()
                ));
            }
        }
    }
    found
}

pub fn run() -> bool {
    let found = problems(Path::new("."));
    for p in &found {
        eprintln!("  ✘ {p}");
    }
    if found.is_empty() {
        println!("  erros ok");
    }
    found.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EH-B10, proveta: cada um dos três problemas reprova, e o repositório passa.
    #[test]
    fn eh_b10_a_trava_dos_erros() {
        let wildcard = "//! sem `_ =>` aqui\nmatch e {\n    A => 1,\n    _ => 2,\n}\n";
        assert_eq!(wildcards(wildcard), vec![4]);
        let enums = "#[derive(Debug, Clone,\n    thiserror::Error)]\npub enum NovoErro {\n    A,\n}\n\
                     #[derive(Debug)]\npub enum NaoErro {}\n";
        let names = error_enums(enums);
        assert_eq!(names, vec!["NovoErro".to_owned()]);
        let human = "impl Human for CoreError {\n";
        let all = vec![
            "NovoErro".to_owned(),
            "CoreError".to_owned(),
            "ToolError".to_owned(),
        ];
        assert_eq!(untranslated(&all, human), vec!["NovoErro".to_owned()]);
        let window = "show(e.to_string());\nshow(e.to_string()); // texto já humano\n\
                      // e.to_string() num comentário\nlet t = format!(\"{e:#}\");\n";
        assert_eq!(raw_texts(window), vec![1, 4]);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let found = problems(root);
        assert!(found.is_empty(), "{found:#?}");
    }
}
