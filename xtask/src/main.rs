//! Tarefas do projeto. Uso: `cargo xtask <tarefa>`.
//!
//! - `verify`: a porta de verificação única (a mesma que o CI roda).
//! - `layers`: confere que nenhum crate depende de uma camada igual ou acima dele.
//! - `size`: alerta de tamanho de arquivo (nunca reprova).
//! - `specs`: toda spec implementada tem um teste por cenário BDD.
//! - `asserts`: nenhuma assertiva frouxa (`is_err()` sem conferir o erro) nos testes.
//! - `erros`: nenhum erro chega à janela sem a frase de fotógrafo (spec EH).
//! - `sala-limpa`: nada da marca ou dos arquivos do software de referência no produto.
//! - `cores`, `contraste`, `icones`: o sistema visual da interface (spec DF).
//! - `espelho [--trazer] <pasta>`: copia só o código para o clone do público (`lamina-app`).
//! - `esquema`: grava o JSON Schema do formato do projeto em `docs/formato/` (spec EI).
//! - `portao <termos>`: lista o que foi alocado para a fase que está abrindo.
//! - `fotos <pasta> [quantas] [--lado N]`: fotos sintéticas para os testes de fumaça (da release e
//!   da importação de 1.000 fotos, spec TR).
//! - `exemplo <pasta>`: prepara as fotos do álbum de exemplo (spec PC).
//! - `versao <tag> [--notas <arquivo>]`: a tag da release bate com o `Cargo.toml` e as novidades.
//! - `mcpb <windows|macos> <binário> <pasta>`: a pasta do pacote do Claude Desktop (spec IA).
//! - `limpar [--dias N] [--poupar <pasta>] [--simular]`: apaga o cache incremental (ADR 0015).

mod asserts;
mod contraste;
mod cores;
mod docs;
mod erros;
mod espelho;
mod esquema;
mod exemplo;
mod exemplo_album;
mod fases;
mod fontes;
mod fotos;
mod icones;
#[cfg(test)]
mod interface;
mod layers;
mod limpar;
mod mcpb;
mod portao;
mod sala_limpa;
mod size;
mod specs;
mod speed;
mod versao;
mod vitrine;

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let ok = match task.as_str() {
        "verify" => verify(),
        "layers" => layers::run(),
        "size" => {
            size::run();
            true
        }
        "specs" => specs::run(),
        "sala-limpa" => sala_limpa::run(),
        "asserts" => asserts::run(),
        "erros" => erros::run(),
        "docs" => docs::run(),
        "cores" => cores::run(),
        "contraste" => contraste::run(),
        "icones" => icones::run(),
        "esquema" => esquema::run(),
        "espelho" => espelho::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "portao" => portao::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "fotos" => fotos::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "exemplo" => exemplo::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "versao" => versao::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "mcpb" => mcpb::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        "limpar" => limpar::run(&std::env::args().skip(2).collect::<Vec<_>>()),
        _ => {
            eprintln!(
                "uso: cargo xtask <verify | layers | size | specs | asserts | erros | sala-limpa | cores | contraste | icones | esquema | portao | fotos | exemplo | versao | mcpb | limpar>"
            );
            false
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Uma checagem do repositório: nome e função que diz se passou.
type Check = (&'static str, fn() -> bool);

/// Roda todos os passos em ordem e para no primeiro que reprovar.
fn verify() -> bool {
    // Os testes de velocidade (todos com "rapido" no nome) rodam num passo próprio, um de cada
    // vez: junto dos outros, disputam a máquina e reprovam sem motivo (auditoria do Marco 2, M1).
    let cargo_steps: [(&str, &[&str]); 3] = [
        ("formatação", &["fmt", "--all", "--check"]),
        (
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "testes",
            &["test", "--workspace", "--quiet", "--", "--skip", "rapido"],
        ),
    ];
    for (name, args) in cargo_steps {
        if !step(name, || cargo(args)) {
            return false;
        }
    }
    if !step("testes de velocidade", speed::run) {
        return false;
    }
    let checks: [Check; 9] = [
        ("camadas", layers::run),
        ("specs", specs::run),
        ("assertivas", asserts::run),
        ("erros", erros::run),
        ("sala limpa", sala_limpa::run),
        ("cores", cores::run),
        ("contraste", contraste::run),
        ("ícones e acessibilidade", icones::run),
        ("documentos", docs::run),
    ];
    for (name, check) in checks {
        if !step(name, check) {
            return false;
        }
    }
    size::run();
    if cargo_deny_available() {
        let deny = || cargo(&["deny", "check", "licenses", "bans", "sources"]);
        if !step("licenças (cargo deny)", deny) {
            return false;
        }
    } else {
        println!("\n· licenças: cargo-deny não instalado aqui; o CI confere.");
    }
    println!("\n✔ porta de verificação: tudo verde");
    true
}

fn step(name: &str, run: impl FnOnce() -> bool) -> bool {
    println!("\n▶ {name}");
    let ok = run();
    if !ok {
        eprintln!("\n✘ porta de verificação reprovou em: {name}");
    }
    ok
}

fn cargo(args: &[&str]) -> bool {
    Command::new(env!("CARGO"))
        .args(args)
        .status()
        .is_ok_and(|status| status.success())
}

fn cargo_deny_available() -> bool {
    Command::new(env!("CARGO"))
        .args(["deny", "--version"])
        .output()
        .is_ok_and(|out| out.status.success())
}
