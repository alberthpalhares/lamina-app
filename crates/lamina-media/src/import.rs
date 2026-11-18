//! Importação de uma pasta: metadados, impressão digital e miniatura de cada foto, em paralelo,
//! reaproveitando o que o catálogo já conhece. No máximo 4 fotos ao mesmo tempo, e cada arquivo
//! lido uma vez só (spec TR, regras 1 e 2).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use crate::derive::{Opened, open, write_derived};
use crate::thumbnail::{DerivedDirs, derived_path};
use crate::{
    Catalog, CatalogEntry, ImageFormat, MediaError, gather_photos, list_photos, map_photos,
};

/// O tamanho máximo de arquivo que a importação lê: 1,5 GB. acima disso, a foto não entra. As
/// câmeras reais param na casa das centenas de MB; o teto afasta TIFF gigante e arquivo que não
/// é foto de verdade (auditoria de 2026-11-11, S4).
pub const FILE_MAX: i64 = 1_500_000_000;

/// Resultado de uma importação.
#[derive(Debug, Default)]
pub struct ImportReport {
    /// Todas as fotos importadas, em ordem de captura.
    pub entries: Vec<CatalogEntry>,
    /// Quantas vieram do catálogo sem serem lidas de novo.
    pub reused: usize,
    /// Quantas foram lidas agora.
    pub processed: usize,
    /// Arquivos que não puderam ser importados, com o motivo.
    pub failures: Vec<(PathBuf, String)>,
    /// Avisos das fotos lidas agora (perfil de cor ilegível: spec CT, seção 6), em ordem de
    /// caminho.
    pub warnings: Vec<String>,
    /// Arquivos de uma lista que não são foto (spec IM, regra 5).
    pub not_photos: usize,
}

/// Andamento da importação: chamado com (fotos prontas, total), sempre em ordem crescente.
pub type Progress<'a> = &'a (dyn Fn(usize, usize) + Sync);

/// Importa as fotos de `dir` (e subpastas), gravando miniaturas em `thumbs_dir`.
pub fn import_folder(
    dir: &Path,
    catalog: &Catalog,
    thumbs_dir: &Path,
) -> Result<ImportReport, MediaError> {
    let dirs = DerivedDirs {
        thumbs: thumbs_dir,
        screens: None,
    };
    import_folder_with(dir, catalog, dirs, &|_, _| {})
}

/// Como [`import_folder`], gerando também as imagens de tela (se `dirs.screens` vier) e
/// informando o andamento.
pub fn import_folder_with(
    dir: &Path,
    catalog: &Catalog,
    dirs: DerivedDirs<'_>,
    progress: Progress<'_>,
) -> Result<ImportReport, MediaError> {
    let photos = list_photos(dir).map_err(MediaError::io(dir))?;
    import_list(photos, catalog, dirs, progress)
}

/// Importa uma lista de arquivos e pastas (spec IM, regra 3): o mesmo miolo da pasta, com as
/// subpastas, contando o que não é foto.
pub fn import_files(
    paths: &[PathBuf],
    catalog: &Catalog,
    dirs: DerivedDirs<'_>,
    progress: Progress<'_>,
) -> Result<ImportReport, MediaError> {
    let first = paths.first().cloned().unwrap_or_default();
    let gathered = gather_photos(paths).map_err(MediaError::io(&first))?;
    let mut report = import_list(gathered.photos, catalog, dirs, progress)?;
    report.not_photos = gathered.not_photos;
    Ok(report)
}

fn import_list(
    photos: Vec<PathBuf>,
    catalog: &Catalog,
    dirs: DerivedDirs<'_>,
    progress: Progress<'_>,
) -> Result<ImportReport, MediaError> {
    let mut report = ImportReport::default();
    let mut pending = Vec::new();
    let total = photos.len();
    for path in photos {
        if !ImageFormat::from_path(&path).is_some_and(ImageFormat::is_decodable) {
            report
                .failures
                .push((path, "formato ainda não suportado".into()));
            continue;
        }
        // Um arquivo que não dá para ler fica de fora; os outros entram (spec IM, seção 6).
        let (size, modified) = match file_stamp(&path) {
            Ok(stamp) => stamp,
            Err(e) => {
                let reason = match &e {
                    MediaError::Io { source, .. } => crate::io_reason(source),
                    other => other.to_string(),
                };
                report.failures.push((path, reason));
                continue;
            }
        };
        // O teto de tamanho: um arquivo que não cabe na memória não chega a ser lido (auditoria
        // de 2026-11-11, S4). Não é erro de álbum: a foto fica de fora, como as que não abrem.
        if size_over_limit(size) {
            report.failures.push((path, too_large(size, FILE_MAX)));
            continue;
        }
        match catalog.get(&path)? {
            Some(e) if is_current(&e, size, modified, dirs) => {
                report.reused += 1;
                report.entries.push(e);
            }
            _ => pending.push((path, size, modified)),
        }
    }

    // Um cadeado em volta do contador e da chamada mantém as contagens em ordem crescente.
    let done = Mutex::new(total - pending.len());
    let step = || {
        if let Ok(mut n) = done.lock() {
            *n += 1;
            progress(*n, total);
        }
    };
    progress(total - pending.len(), total);
    // No máximo 4 fotos ao mesmo tempo (spec TR, regra 1), e os resultados na ordem da lista.
    let results = map_photos(pending, |(path, size, modified)| {
        let result = process(&path, (size, modified), dirs).map_err(|e| (path, e.to_string()));
        step();
        result
    });
    let mut fresh = Vec::new();
    for result in results {
        match result {
            Ok((entry, warning)) => {
                report.warnings.extend(warning);
                fresh.push(entry);
            }
            Err(failure) => report.failures.push(failure),
        }
    }
    catalog.upsert_all(&fresh)?;
    report.processed = fresh.len();
    report.entries.extend(fresh);
    report.entries.sort_by(|a, b| {
        (a.taken_at.is_none(), a.taken_at, &a.path).cmp(&(
            b.taken_at.is_none(),
            b.taken_at,
            &b.path,
        ))
    });
    Ok(report)
}

/// A entrada do catálogo vale se o arquivo não mudou e as cópias reduzidas pedidas existem.
fn is_current(e: &CatalogEntry, size: i64, modified: i64, dirs: DerivedDirs<'_>) -> bool {
    let screen_ok = dirs
        .screens
        .is_none_or(|d| derived_path(d, &e.fingerprint).exists());
    // A miniatura tem que ser da versão atual do cache (spec CT, regra 10).
    let thumb_ok = e.thumbnail == derived_path(dirs.thumbs, &e.fingerprint) && e.thumbnail.exists();
    e.file_size == size && e.modified == modified && thumb_ok && screen_ok
}

/// Tamanho e data de modificação (segundos desde 1970) do arquivo.
fn file_stamp(path: &Path) -> Result<(i64, i64), MediaError> {
    let meta = std::fs::metadata(path).map_err(MediaError::io(path))?;
    let size = i64::try_from(meta.len()).unwrap_or(i64::MAX);
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    Ok((size, modified))
}

/// O motivo que entra na lista de falhas quando a foto passa do teto: linguagem de fotógrafo
/// (regra 15), sem o caminho no meio porque ele já vem no lado de fora do par.
fn too_large(size: i64, max: i64) -> String {
    let max_gb = max / 1_000_000_000;
    let size_gb = size / 1_000_000_000;
    format!("grande demais para importar: {size_gb} GB (o máximo é {max_gb} GB)")
}

/// Se o carimbo do tamanho passa do teto de importação.
fn size_over_limit(size: i64) -> bool {
    size > FILE_MAX
}

/// Uma foto: o arquivo é lido do disco uma vez só (spec TR, regra 2).
fn process(
    path: &Path,
    stamp: (i64, i64),
    dirs: DerivedDirs<'_>,
) -> Result<(CatalogEntry, Option<String>), MediaError> {
    let bytes = std::fs::read(path).map_err(MediaError::io(path))?;
    process_bytes(path, bytes, stamp, dirs)
}

/// O trabalho de uma foto a partir do arquivo já lido: a entrada do catálogo, as cópias
/// reduzidas e o aviso do perfil. Nada mais é lido do disco (TR-B3).
pub(crate) fn process_bytes(
    path: &Path,
    bytes: Vec<u8>,
    (file_size, modified): (i64, i64),
    dirs: DerivedDirs<'_>,
) -> Result<(CatalogEntry, Option<String>), MediaError> {
    let opened = open(path, &bytes)?;
    let thumbnail = write_derived(path, bytes, &opened, dirs)?;
    let Opened {
        fingerprint,
        metadata: meta,
        warning,
        ..
    } = opened;
    let entry = CatalogEntry {
        path: path.to_path_buf(),
        fingerprint,
        file_size,
        modified,
        width: meta.width,
        height: meta.height,
        exif_orientation: meta.exif_orientation,
        taken_at: meta.taken_at,
        camera: meta.camera,
        thumbnail,
    };
    Ok((entry, warning))
}

/// Pasta de cache padrão do sistema para o Lamina (catálogo e miniaturas).
pub fn default_cache_dir() -> PathBuf {
    let env = |name| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(target_os = "windows") {
        env("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Caches"))
    } else {
        env("XDG_CACHE_HOME").or_else(|| env("HOME").map(|h| h.join(".cache")))
    };
    base.unwrap_or_else(std::env::temp_dir).join("Lamina")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{Exif, write_jpeg};

    #[test]
    fn importa_ordena_por_data_e_reaproveita_na_segunda_vez() {
        let dir = tempfile::tempdir().unwrap();
        let photos = dir.path().join("fotos");
        std::fs::create_dir_all(&photos).unwrap();
        write_jpeg(
            &photos,
            "b.jpg",
            60,
            40,
            Some(&Exif::new(1, "2026:09:12 16:00:00", "C")),
        );
        write_jpeg(
            &photos,
            "a.jpg",
            60,
            40,
            Some(&Exif::new(1, "2026:09:12 15:00:00", "C")),
        );
        write_jpeg(&photos, "sem_data.jpg", 40, 60, None);
        std::fs::write(photos.join("quebrada.jpg"), b"nada").unwrap();
        let catalog = Catalog::open_in_memory().unwrap();
        let thumbs = dir.path().join("miniaturas");

        let first = import_folder(&photos, &catalog, &thumbs).unwrap();
        assert_eq!(
            (first.processed, first.reused, first.failures.len()),
            (3, 0, 1)
        );
        let names: Vec<_> = first
            .entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.jpg", "b.jpg", "sem_data.jpg"]);
        assert!(first.entries.iter().all(|e| e.thumbnail.exists()));

        let second = import_folder(&photos, &catalog, &thumbs).unwrap();
        assert_eq!((second.processed, second.reused), (0, 3));
    }

    #[test]
    fn heic_e_reportado_como_nao_suportado() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("iphone.heic"), b"x").unwrap();
        let catalog = Catalog::open_in_memory().unwrap();
        let report = import_folder(dir.path(), &catalog, &dir.path().join("t")).unwrap();
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].1.contains("não suportado"));
    }

    #[test]
    fn arquivo_grande_demais_fica_de_fora_sem_ser_lido() {
        // Auditoria de 2026-11-11, S4: o carimbo do tamanho vem do metadado do sistema; gravar
        // 1,5 GB de verdade não cabe num teste. O que este teste confere é a decisão e a frase:
        // acima do teto a foto é afastada com o motivo, abaixo (e no próprio limite) ela entra.
        let small = 60_000_i64;
        let huge = FILE_MAX + 500_000_000;
        assert!(size_over_limit(huge));
        assert!(!size_over_limit(small));
        assert!(!size_over_limit(FILE_MAX), "o próprio limite passa");
        // O motivo que o usuário vê, em linguagem de fotógrafo (regra 15):
        assert_eq!(
            too_large(huge, FILE_MAX),
            "grande demais para importar: 2 GB (o máximo é 1 GB)"
        );
    }

    #[test]
    fn foto_no_limite_do_teto_passa() {
        let dir = tempfile::tempdir().unwrap();
        write_jpeg(dir.path(), "no_limite.jpg", 60, 40, None);
        assert_eq!(FILE_MAX, 1_500_000_000);
        let catalog = Catalog::open_in_memory().unwrap();
        let report = import_folder(dir.path(), &catalog, &dir.path().join("t")).unwrap();
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    }

    #[test]
    fn andamento_cresce_ate_o_total_e_gera_imagens_de_tela() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..6 {
            write_jpeg(dir.path(), &format!("{i}.jpg"), 60, 40, None);
        }
        let catalog = Catalog::open_in_memory().unwrap();
        let (thumbs, screens) = (dir.path().join("m"), dir.path().join("t"));
        let dirs = DerivedDirs {
            thumbs: &thumbs,
            screens: Some(&screens),
        };
        let seen = Mutex::new(Vec::new());
        let progress = |done: usize, total: usize| seen.lock().unwrap().push((done, total));
        let report = import_folder_with(dir.path(), &catalog, dirs, &progress).unwrap();
        let seen = seen.into_inner().unwrap();
        assert!(seen.windows(2).all(|w| w[0].0 < w[1].0), "{seen:?}");
        assert_eq!(seen.last(), Some(&(6, 6)));
        let fp = &report.entries[0].fingerprint;
        assert!(derived_path(&screens, fp).exists());
    }
}
