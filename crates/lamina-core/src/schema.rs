//! Esquema do projeto (spec EI, regra 9): o formato `.lamina` descrito em JSON Schema, gerado
//! dos próprios tipos. O arquivo publicado em `docs/formato/` é conferido contra este (`cargo
//! xtask docs`) e regravado por `cargo xtask esquema`.

use crate::{CURRENT_SCHEMA_VERSION, CoreError, Project};

/// `projeto-v8.schema.json`: o nome muda junto com a versão do formato.
pub fn schema_file_name() -> String {
    format!("projeto-v{CURRENT_SCHEMA_VERSION}.schema.json")
}

/// O JSON Schema do formato atual, formatado e terminado em quebra de linha.
pub fn project_schema() -> Result<String, CoreError> {
    let schema = schemars::schema_for!(Project);
    Ok(serde_json::to_string_pretty(&schema)? + "\n")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value;

    use super::*;
    use crate::{AlbumSpec, Dpi, Mm, RelRect};

    /// Um projeto com foto, lâmina e quadro, como o Lamina grava.
    fn saved() -> Value {
        let spec = AlbumSpec::new(Mm(300.0), Mm(300.0), Dpi::new(300).unwrap()).unwrap();
        let mut project = Project::new("teste", spec);
        let photo = project.add_photo("a.jpg", 300, 200);
        let spread = project.add_spread();
        let content = crate::FrameContent::Photo {
            photo,
            crop: crate::Crop::default(),
        };
        let rect = RelRect::new(0.1, 0.1, 0.3, 0.3).unwrap();
        project.add_frame(spread, rect, content).unwrap();
        serde_json::from_str(&project.to_json().unwrap()).unwrap()
    }

    fn required(schema: &Value) -> Vec<&str> {
        schema["required"]
            .as_array()
            .map(|r| r.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// Os obrigatórios estão no objeto, e o objeto não grava nada que o esquema não conheça
    /// (auditoria do Marco 2, M7).
    fn has_all(object: &Value, schema: &Value, what: &str) {
        for field in required(schema) {
            assert!(object.get(field).is_some(), "{what} sem o campo {field}");
        }
        let known = schema["properties"].as_object().unwrap();
        for key in object.as_object().unwrap().keys() {
            assert!(
                known.contains_key(key),
                "{what}: campo {key} fora do esquema"
            );
        }
    }

    #[test]
    fn ei_b10_projeto_salvo_tem_os_campos_obrigatorios_do_esquema() {
        let schema: Value = serde_json::from_str(&project_schema().unwrap()).unwrap();
        let project = saved();
        assert!(!required(&schema).is_empty());
        has_all(&project, &schema, "o projeto");
        let defs = &schema["$defs"];
        has_all(&project["spec"], &defs["AlbumSpec"], "a especificação");
        has_all(&project["photos"][0], &defs["Photo"], "a foto");
        has_all(&project["spreads"][0], &defs["Spread"], "a lâmina");
        has_all(
            &project["spreads"][0]["frames"][0],
            &defs["Frame"],
            "o quadro",
        );
    }

    #[test]
    fn ei_b10_esquema_publicado_bate_com_o_codigo() {
        // Só no repositório principal: o público não leva os documentos.
        let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/formato");
        let published = docs.join(schema_file_name());
        if !docs.join("..").join("adr").exists() {
            return;
        }
        let text = std::fs::read_to_string(&published).unwrap_or_default();
        assert!(
            text == project_schema().unwrap(),
            "{} desatualizado: rode cargo xtask esquema",
            published.display()
        );
        assert_eq!(schema_file_name(), "projeto-v8.schema.json");
    }
}
