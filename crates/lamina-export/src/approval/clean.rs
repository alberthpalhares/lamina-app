//! Os arquivos da aprovação na pasta (spec AC, regras 3 e 14): `web-capa.jpg` e `web-lamina-NN.jpg`.
//! Nenhum outro arquivo é tocado, e a limpeza da impressão (`crate::clean`) não conhece estes nomes.

use std::path::{Path, PathBuf};

use super::ApprovalOptions;
use crate::ExportError;

/// O nome é de um arquivo que a aprovação grava?
fn is_approval_name(name: &str) -> bool {
    if name == super::plan::COVER_NAME {
        return true;
    }
    let number = name
        .strip_prefix("web-lamina-")
        .and_then(|rest| rest.strip_suffix(".jpg"));
    number.is_some_and(|n| n.len() >= 2 && n.bytes().all(|b| b.is_ascii_digit()))
}

/// O HTML de uma geração anterior, com qualquer nome que o cliente visse (spec AH, regra 19).
fn is_html_name(name: &str) -> bool {
    name.ends_with("-aprovacao.html")
}

/// Os arquivos da aprovação em `folder` (sem entrar nas subpastas), em ordem, só das saídas ligadas
/// em `options`: só com o HTML, os JPG ficam, e só com o JPG, o HTML fica. Pasta que não existe não
/// tem nenhum.
pub(super) fn approval_files(
    folder: &Path,
    options: &ApprovalOptions,
) -> Result<Vec<String>, ExportError> {
    if !folder.exists() {
        return Ok(Vec::new());
    }
    let mut found: Vec<String> = std::fs::read_dir(folder)
        .map_err(|e| remove_failed(folder, &e))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| {
            (options.jpg && is_approval_name(name))
                || (options.html.is_some() && is_html_name(name))
        })
        .collect();
    found.sort();
    Ok(found)
}

/// Apaga os arquivos da aprovação que não estão em `keep`. Devolve os apagados, em ordem.
pub(super) fn remove_leftovers(
    folder: &Path,
    keep: &[String],
    options: &ApprovalOptions,
) -> Result<Vec<PathBuf>, ExportError> {
    let mut removed = Vec::new();
    for name in approval_files(folder, options)? {
        if keep.contains(&name) {
            continue;
        }
        let path = folder.join(&name);
        std::fs::remove_file(&path).map_err(|e| remove_failed(&path, &e))?;
        removed.push(path);
    }
    Ok(removed)
}

fn remove_failed(path: &Path, e: &std::io::Error) -> ExportError {
    ExportError::Remove {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_os_nomes_da_aprovacao() {
        for name in ["web-capa.jpg", "web-lamina-01.jpg", "web-lamina-120.jpg"] {
            assert!(is_approval_name(name), "{name}");
        }
        for name in [
            "lamina-01.jpg",
            "capa.jpg",
            "pagina-01.jpg",
            "web-lamina-1.jpg",
            "web-lamina-0a.jpg",
            "web-lamina-01.png",
            "minha-web-lamina-01.jpg",
            "notas.txt",
        ] {
            assert!(!is_approval_name(name), "{name}");
        }
    }

    #[test]
    fn os_nomes_nao_se_cruzam_com_os_da_impressao() {
        for name in ["web-capa.jpg", "web-lamina-01.jpg"] {
            assert!(!crate::is_export_name(name), "{name}");
        }
    }
}
