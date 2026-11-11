//! Entrega para conferência do dono (ADR 0018): `cargo xtask conferir <pasta>` roda a porta de
//! verificação completa, compila o `lamina-app` em release e põe o executável na pasta de
//! conferência com um link físico — sem copiar os ~130 MB de novo.
//!
//! O link físico (mesmo volume, NTFS) não copia bytes: o arquivo aponta para o mesmo conteúdo
//! que o cargo publicou. Como o cargo troca o executável por renomeação, um link antigo continua
//! abrindo a versão anterior, mesmo com o dono rodando o programa no momento da entrega.
//!
//! Uso: `cargo xtask conferir D:\Lamina\conferencia\beta-10-2`.

use std::fs;
use std::path::{Path, PathBuf};

const EXE: &str = "target/release/lamina-app.exe";

pub fn run(args: &[String]) -> bool {
    if args.len() != 1 || args[0].is_empty() {
        eprintln!("uso: cargo xtask conferir <pasta da conferência>");
        return false;
    }
    let Some(folder) = validate(&args[0]) else {
        eprintln!(
            "  ✘ conferir: a pasta {} não existe; crie-a antes (com o CONFERIR.md)",
            args[0]
        );
        return false;
    };
    if !crate::verify() {
        eprintln!("  ✘ conferir: a porta de verificação reprova; nada chega ao dono");
        return false;
    }
    let build = crate::cargo(&["build", "--release", "-p", "lamina-app"]);
    if !build {
        eprintln!("  ✘ conferir: o build de release falhou");
        return false;
    }
    let source = PathBuf::from(EXE);
    if !source.is_file() {
        eprintln!(
            "  ✘ conferir: {} não existe depois do build",
            source.display()
        );
        return false;
    }
    let dest = folder.join("lamina-app.exe");
    match publish(&source, &dest) {
        Ok(()) => {
            println!("  ✔ conferir: {} pronto para o dono", dest.display());
            true
        }
        Err(e) => {
            eprintln!("  ✘ conferir: {e}");
            false
        }
    }
}

/// A pasta existe e é um diretório.
fn validate(arg: &str) -> Option<PathBuf> {
    let path = PathBuf::from(arg);
    path.is_dir().then_some(path)
}

/// Põe o executável na pasta: link físico quando dá (mesmo volume), cópia como reserva.
pub fn publish(source: &Path, dest: &Path) -> Result<(), String> {
    if let Err(hard_error) = fs::hard_link(source, dest) {
        // O link falha quando o destino já existe, está em uso ou o volume é outro.
        let _ = fs::remove_file(dest);
        fs::copy(source, dest).map_err(|copy_error| {
            format!(
                "não consegui publicar o executável: link ({hard_error}) nem cópia ({copy_error})"
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publicar_por_link_fisico_no_mesmo_volume() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("app.exe");
        let dest = dir.path().join("conferencia").join("app.exe");
        fs::write(&source, [7; 128]).unwrap();
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        publish(&source, &dest).unwrap();
        let bytes = fs::read(&dest).unwrap();
        assert_eq!(bytes, [7; 128]);
    }

    #[test]
    fn publicar_de_novo_substitui_a_versao_anterior() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("app.exe");
        let dest = dir.path().join("conferencia").join("app.exe");
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&source, [1; 64]).unwrap();
        publish(&source, &dest).unwrap();
        fs::write(&source, [2; 64]).unwrap();
        publish(&source, &dest).unwrap();
        let bytes = fs::read(&dest).unwrap();
        assert_eq!(bytes, [2; 64]);
    }

    #[test]
    fn reprova_quando_a_pasta_nao_existe() {
        let args = ["nao-existe".to_owned()];
        assert!(!run(&args));
    }

    #[test]
    fn reprova_sem_argumento() {
        assert!(!run(&[]));
    }
}
