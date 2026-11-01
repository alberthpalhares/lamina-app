//! Fotos ligadas ao projeto por caminho relativo à pasta dele (ADR 0004; spec CB, regra 4), e o
//! nome do arquivo do projeto. Só contas com texto: quem chama decide as pastas e olha o disco.

use std::path::{Component, Path, PathBuf};

use super::{Photo, Project};
use crate::CoreError;

/// Caminho de `target` visto de `base`, com `/` entre as partes (o mesmo texto abre no Windows,
/// no macOS e no Linux). `None` se não houver caminho relativo seguro: outro disco no Windows,
/// um relativo e o outro absoluto, ou `..` no meio de algum dos dois (normalize antes, por
/// exemplo com `std::path::absolute`). Um `.` no começo é ignorado.
pub fn relative_path(base: &Path, target: &Path) -> Option<String> {
    if base.is_absolute() != target.is_absolute() {
        return None;
    }
    let (b, t) = (parts(base)?, parts(target)?);
    if b[..root(&b)] != t[..root(&t)] {
        return None;
    }
    let common = b.iter().zip(&t).take_while(|(x, y)| x == y).count();
    let mut out: Vec<String> = vec!["..".to_owned(); b.len() - common];
    out.extend(
        t[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    Some(if out.is_empty() {
        ".".to_owned()
    } else {
        out.join("/")
    })
}

/// O caminho com `.` e `..` resolvidos pelo texto (`/a/b/../c` → `/a/c`), sem tocar o disco. O
/// `std::path::absolute` faz isso no Windows, mas não no Linux nem no macOS: lá, abrir e salvar
/// de novo um álbum com as fotos numa pasta vizinha gravava caminhos absolutos (achado no IA-B16).
/// Um `..` que passaria da raiz fica de fora; num caminho relativo, os `..` do começo ficam.
pub fn lexical_path(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(part),
            },
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return PathBuf::from(".");
    }
    out.iter().collect()
}

/// Partes do caminho sem os `.`; `None` se houver `..`.
fn parts(path: &Path) -> Option<Vec<Component<'_>>> {
    let comps: Vec<Component> = path
        .components()
        .filter(|c| *c != Component::CurDir)
        .collect();
    (!comps.contains(&Component::ParentDir)).then_some(comps)
}

/// Quantas partes iniciais são o disco e a raiz.
fn root(parts: &[Component]) -> usize {
    parts
        .iter()
        .take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
        .count()
}

/// Nomes que o Windows reserva para dispositivos.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Caracteres proibidos em nomes de arquivo no Windows (e a barra, em todos).
const FORBIDDEN: &str = r#"<>:"/\|?*"#;

/// Confere se `name` serve de nome do arquivo do projeto nos três sistemas: sem `<>:"/\|?*`,
/// sem caracteres de controle, sem terminar em `.` ou espaço e sem nome reservado do Windows.
pub fn validate_project_name(name: &str) -> Result<(), CoreError> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim()
        .to_uppercase();
    let invalid = name.trim().is_empty()
        || name
            .chars()
            .any(|c| c.is_control() || FORBIDDEN.contains(c))
        || name.ends_with(['.', ' '])
        || RESERVED.contains(&stem.as_str());
    if invalid {
        return Err(CoreError::InvalidName(name.to_owned()));
    }
    Ok(())
}

impl Project {
    /// Troca o caminho de cada foto pelo relativo a `base` (a pasta onde o projeto será
    /// gravado); fotos sem caminho relativo seguro ficam com o absoluto.
    pub fn relativize(&mut self, base: &Path) {
        for photo in &mut self.photos {
            if let Some(rel) = relative_path(base, &photo.path) {
                photo.path = PathBuf::from(rel);
            }
        }
        // O guia da capa acompanha o álbum, como as fotos (spec CA, regra 18).
        if let Some(guide) = self.cover.as_mut().and_then(|c| c.guide.as_mut())
            && let Some(rel) = relative_path(base, &guide.path)
        {
            guide.path = PathBuf::from(rel);
        }
    }

    /// Resolve os caminhos relativos a partir de `base` (a pasta do arquivo do projeto); os
    /// absolutos ficam como estão.
    pub fn resolve_paths(&mut self, base: &Path) {
        for photo in &mut self.photos {
            if photo.path.is_relative() {
                photo.path = lexical_path(&base.join(&photo.path));
            }
        }
        if let Some(guide) = self.cover.as_mut().and_then(|c| c.guide.as_mut())
            && guide.path.is_relative()
        {
            guide.path = lexical_path(&base.join(&guide.path));
        }
    }

    /// A primeira foto cujo arquivo não existe, segundo `exists` (quem chama olha o disco).
    pub fn missing_photo(&self, exists: impl Fn(&Path) -> bool) -> Option<&Photo> {
        self.photos.iter().find(|p| !exists(&p.path))
    }
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
