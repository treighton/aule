use std::path::PathBuf;

use aule_schema::manifest::{self, ManifestAny};
use super::CliError;
use crate::output;

pub fn run(path: Option<PathBuf>, json: bool) -> Result<(), CliError> {
    let base_path = path.unwrap_or_else(|| PathBuf::from("."));
    let manifest_path = base_path.join("skill.yaml");

    let manifest_any = manifest::load_manifest_any(&manifest_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    let m = match manifest_any {
        ManifestAny::V1(m) => m,
        ManifestAny::V2(_) => {
            if json {
                let value = serde_json::json!({
                    "status": "ok",
                    "message": "already v0.2.0, no migration needed",
                });
                output::print_json(&value);
            } else {
                println!("Manifest is already v0.2.0 — no migration needed.");
            }
            return Ok(());
        }
    };

    // Convert v0.1.0 → v0.2.0

    // Resolve the contract (inline or from file)
    let contract = match &m.contract {
        manifest::ContractRef::Inline(c) => c.clone(),
        manifest::ContractRef::File(file_path) => {
            let contract_path = base_path.join(file_path);
            aule_schema::contract::parse_contract(
                aule_schema::contract::ContractSource::File(&contract_path),
            )
            .map_err(|e| CliError::User(format!("failed to load contract: {}", e)))?
        }
    };

    // Build the skills map from the single contract
    let skill_name = m.name.clone();
    let mut skill_inputs = None;
    let mut skill_outputs = None;

    if !contract.inputs.is_prompt() {
        // Serialize the InputOutput to a serde_json::Value for the v0.2.0 format
        skill_inputs = Some(contract.inputs.clone());
    }
    if !contract.outputs.is_prompt() {
        skill_outputs = Some(contract.outputs.clone());
    }

    let skill_def = serde_json::json!({
        "description": m.description,
        "entrypoint": m.content.skill,
        "version": contract.version,
        "permissions": contract.permissions,
        "determinism": format!("{:?}", contract.determinism).to_lowercase(),
    });

    let mut skill_map = serde_json::Map::new();
    let mut skill_obj = skill_def.as_object().unwrap().clone();

    if let Some(inputs) = skill_inputs {
        skill_obj.insert("inputs".to_string(), serde_json::to_value(&inputs).unwrap());
    }
    if let Some(outputs) = skill_outputs {
        skill_obj.insert("outputs".to_string(), serde_json::to_value(&outputs).unwrap());
    }

    // Commands
    if let Some(ref commands) = m.content.commands {
        skill_obj.insert("commands".to_string(), serde_json::to_value(commands).unwrap());
    }

    // Errors
    if let Some(ref errors) = contract.errors {
        skill_obj.insert("errors".to_string(), serde_json::to_value(errors).unwrap());
    }

    // Behavior
    if let Some(ref behavior) = contract.behavior {
        skill_obj.insert("behavior".to_string(), serde_json::to_value(behavior).unwrap());
    }

    skill_map.insert(skill_name.clone(), serde_json::Value::Object(skill_obj));

    // Build the files list from content paths
    let files = vec!["content/**".to_string()];

    // Build the v0.2.0 YAML
    let mut v2 = serde_json::Map::new();
    v2.insert("schemaVersion".to_string(), serde_json::json!("0.2.0"));
    v2.insert("name".to_string(), serde_json::json!(m.name));
    v2.insert("description".to_string(), serde_json::json!(m.description));
    v2.insert("version".to_string(), serde_json::json!(m.version));
    v2.insert("files".to_string(), serde_json::to_value(&files).unwrap());
    v2.insert("skills".to_string(), serde_json::Value::Object(skill_map));

    if !m.adapters.is_empty() {
        v2.insert("adapters".to_string(), serde_json::to_value(&m.adapters).unwrap());
    }

    if let Some(ref metadata) = m.metadata {
        v2.insert("metadata".to_string(), serde_json::to_value(metadata).unwrap());
    }

    if let Some(ref deps) = m.dependencies {
        v2.insert("dependencies".to_string(), serde_json::to_value(deps).unwrap());
    }

    if let Some(ref identity) = m.identity {
        v2.insert("identity".to_string(), serde_json::json!(identity));
    }

    if let Some(ref extensions) = m.extensions {
        v2.insert("extensions".to_string(), serde_json::to_value(extensions).unwrap());
    }

    let v2_yaml = serde_yaml::to_string(&serde_json::Value::Object(v2))
        .map_err(|e| CliError::Internal(format!("YAML serialization failed: {}", e)))?;

    // Write the migrated manifest
    std::fs::write(&manifest_path, &v2_yaml)
        .map_err(|e| CliError::Internal(format!("failed to write manifest: {}", e)))?;

    if json {
        let value = serde_json::json!({
            "status": "ok",
            "message": "migrated to v0.2.0",
            "path": manifest_path.display().to_string(),
        });
        output::print_json(&value);
    } else {
        println!("Migrated {} to v0.2.0 format.", manifest_path.display());
        println!("  Skills: {}", skill_name);
        println!("  Files: {}", files.join(", "));
        println!("\nReview the generated manifest and adjust as needed.");
    }

    Ok(())
}
