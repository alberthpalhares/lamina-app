//! Migrações do formato de projeto (ADR 0004).
//!
//! Cada função leva o JSON de uma versão para a seguinte. [`migrate`] aplica a cadeia até a
//! versão atual. Mudou a estrutura? Suba `CURRENT_SCHEMA_VERSION`, escreva a função
//! `vN_para_vN1` e um teste com um arquivo da versão antiga.

use serde_json::{Map, Value, json};

use super::CURRENT_SCHEMA_VERSION;
use crate::CoreError;

/// Leva `value` da versão `from` até a atual.
pub(super) fn migrate(mut value: Value, from: u32) -> Result<Value, CoreError> {
    let mut version = from;
    while version < CURRENT_SCHEMA_VERSION {
        value = match version {
            1 => v1_para_v2(value)?,
            2 => v2_para_v3(value)?,
            3 => v3_para_v4(value)?,
            4 => v4_para_v5(value),
            5 => v5_para_v6(value),
            6 => v6_para_v7(value),
            7 => v7_para_v8(value),
            _ => {
                return Err(CoreError::Migration {
                    from: version,
                    reason: "versão desconhecida".into(),
                });
            }
        };
        version += 1;
    }
    Ok(value)
}

fn spec_mut(value: &mut Value, from: u32) -> Result<&mut Map<String, Value>, CoreError> {
    value
        .get_mut("spec")
        .and_then(Value::as_object_mut)
        .ok_or(CoreError::Migration {
            from,
            reason: "campo `spec` ausente".into(),
        })
}

/// v1 → v2: sangria e zona segura passam de um número para margens por borda
/// (`outer`, `top_bottom`, `inner`), e a especificação ganha `edges`.
fn v1_para_v2(mut value: Value) -> Result<Value, CoreError> {
    let spec = spec_mut(&mut value, 1)?;
    let uniform = |v: Option<Value>| {
        let mm = v.unwrap_or(json!(0.0));
        json!({ "outer": mm, "top_bottom": mm, "inner": mm })
    };
    let bleed = uniform(spec.remove("bleed"));
    let safe = uniform(spec.remove("safe_margin"));
    spec.insert("bleed".into(), bleed);
    spec.insert("safe_margins".into(), safe);
    spec.insert(
        "edges".into(),
        json!({ "start": "full_spread", "end": "full_spread" }),
    );
    value["schema_version"] = json!(2);
    Ok(value)
}

/// v2 → v3 (spec AP): espaço entre fotos do álbum (8 pt); por lâmina, fundo da página direita e
/// espaço próprios (nenhum); por quadro, borda (nenhuma) e cantos (retos).
fn v2_para_v3(mut value: Value) -> Result<Value, CoreError> {
    let missing = |what: &str| CoreError::Migration {
        from: 2,
        reason: format!("campo `{what}` ausente"),
    };
    let object = value.as_object_mut().ok_or_else(|| missing("projeto"))?;
    object.insert("gap".into(), json!(super::project::DEFAULT_GAP.get()));
    let spreads = object
        .get_mut("spreads")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| missing("spreads"))?;
    for spread in spreads {
        let spread = spread.as_object_mut().ok_or_else(|| missing("lâmina"))?;
        spread.insert("background_right".into(), Value::Null);
        spread.insert("gap".into(), Value::Null);
        let frames = spread
            .get_mut("frames")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| missing("frames"))?;
        for frame in frames {
            let frame = frame.as_object_mut().ok_or_else(|| missing("quadro"))?;
            frame.insert(
                "border".into(),
                json!({ "width": 0.0, "color": { "r": 255, "g": 255, "b": 255 } }),
            );
            frame.insert("radius".into(), json!(0.0));
        }
    }
    value["schema_version"] = json!(3);
    Ok(value)
}

/// v3 → v4 (spec NP): a montagem do álbum, com o comportamento de sempre (livre, com página
/// sangrada e mosaicos).
fn v3_para_v4(mut value: Value) -> Result<Value, CoreError> {
    let object = value.as_object_mut().ok_or(CoreError::Migration {
        from: 3,
        reason: "campo `projeto` ausente".into(),
    })?;
    object.insert(
        "montagem".into(),
        json!({ "recorte": "livre", "pagina_sangrada": true, "mosaicos": true }),
    );
    object.insert(
        "aparencia".into(),
        json!({
            "fundo": { "r": 255, "g": 255, "b": 255 },
            "borda": { "width": 0.0, "color": { "r": 255, "g": 255, "b": 255 } },
            "cantos": 0.0
        }),
    );
    value["schema_version"] = json!(4);
    Ok(value)
}

/// v4 → v5 (spec AF): as fotos ganham os ajustes, que começam neutros e não são gravados; nada
/// muda no arquivo além da versão.
fn v4_para_v5(mut value: Value) -> Value {
    value["schema_version"] = json!(5);
    value
}

/// v5 → v6 (spec OP): os quadros ganham a opacidade, que começa em 100 e não é gravada; nada muda
/// no arquivo além da versão.
fn v5_para_v6(mut value: Value) -> Value {
    value["schema_version"] = json!(6);
    value
}

/// v6 → v7 (specs TM e CD): as fotos ganham temperatura e matiz, que começam em zero e não são
/// gravados, e o giro passa a ir até ±180°; nada muda no arquivo além da versão. Um Lamina antigo
/// recusa o arquivo 7 em vez de perder a cor e o giro ao salvar.
fn v6_para_v7(mut value: Value) -> Value {
    value["schema_version"] = json!(7);
    value
}

/// v7 → v8 (specs PB e CA): as lâminas ganham as páginas em branco e o projeto, a capa, que só vão
/// para o arquivo quando existem; nada muda além da versão. Um Lamina antigo recusa o arquivo 8 em
/// vez de perder as duas coisas ao salvar.
fn v7_para_v8(mut value: Value) -> Value {
    value["schema_version"] = json!(8);
    value
}

#[cfg(test)]
mod tests {
    use crate::{AlbumEdge, Margins, Mm, Project};

    /// Projeto salvo pela versão 1 do formato (fundação, 2026-10-14).
    const PROJETO_V1: &str = r#"{
        "schema_version": 1,
        "name": "Álbum antigo",
        "spec": {
            "page_width": 300.0, "page_height": 300.0, "dpi": 300,
            "bleed": 3.0, "safe_margin": 10.0, "mode": "spread"
        },
        "photos": [],
        "spreads": [],
        "next_id": 1
    }"#;

    #[test]
    fn projeto_v1_abre_com_margens_uniformes() {
        let project = Project::from_json(PROJETO_V1).unwrap();
        assert_eq!(project.schema_version, super::CURRENT_SCHEMA_VERSION);
        assert_eq!(project.spec.bleed(), Margins::uniform(Mm(3.0)));
        assert_eq!(project.spec.safe_margins(), Margins::uniform(Mm(10.0)));
        assert_eq!(project.spec.edges().start, AlbumEdge::FullSpread);
    }

    #[test]
    fn projeto_v1_sem_spec_da_erro_de_migracao() {
        let json = r#"{ "schema_version": 1, "name": "x" }"#;
        assert!(matches!(
            Project::from_json(json),
            Err(crate::CoreError::Migration { from: 1, .. })
        ));
    }

    #[test]
    fn np_b14_projeto_v3_abre_livre_e_salva_na_versao_4() {
        let project = Project::from_json(PROJETO_V1).unwrap();
        // O mesmo projeto como a versão 3 gravava: sem a montagem.
        let mut v3: serde_json::Value = serde_json::from_str(&project.to_json().unwrap()).unwrap();
        v3["schema_version"] = serde_json::json!(3);
        v3.as_object_mut().unwrap().remove("montagem");
        v3.as_object_mut().unwrap().remove("aparencia");
        let opened = Project::from_json(&v3.to_string()).unwrap();
        assert_eq!(opened.mounting, crate::Mounting::default());
        let saved = opened.to_json().unwrap();
        assert!(
            saved.contains(&format!(
                "\"schema_version\": {}",
                super::CURRENT_SCHEMA_VERSION
            )),
            "{saved}"
        );
        assert!(saved.contains("\"montagem\""), "{saved}");
        assert_eq!(Project::from_json(&saved).unwrap(), opened);
    }
}
