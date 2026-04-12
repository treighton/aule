use aule_adapter::{generate, GenerateOptions, RuntimeTarget};
use aule_cache::{ActivationRecord, ActivationState, CacheManager, MetadataIndex};
use aule_schema::manifest;

use super::CliError;
use crate::output;

pub fn run(name: String, target: Option<String>, json: bool) -> Result<(), CliError> {
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;

    let index =
        MetadataIndex::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;

    let entry = index
        .entries
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| CliError::User(format!("skill \"{}\" is not installed", name)))?;

    // Load manifest from the stored manifest path
    let manifest_path = std::path::Path::new(&entry.manifest_path);
    let m = manifest::load_manifest(manifest_path)
        .map_err(|e| CliError::User(format!("failed to load manifest: {}", e)))?;

    let base_path = manifest_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    // Determine which targets to activate
    let targets: Vec<RuntimeTarget> = if let Some(ref t) = target {
        RuntimeTarget::by_id(t)
            .map(|rt| vec![rt])
            .ok_or_else(|| CliError::User(format!("unknown target: {}", t)))?
    } else {
        m.adapters
            .iter()
            .filter(|(_, cfg)| cfg.enabled)
            .filter_map(|(id, _)| RuntimeTarget::by_id(id))
            .collect()
    };

    if targets.is_empty() {
        return Err(CliError::User("no adapter targets to activate".to_string()));
    }

    let mut activated = Vec::new();

    for rt in &targets {
        let options = GenerateOptions {
            targets: vec![rt.id.clone()],
            output_dir: None, // generates into current directory
        };

        let generated = generate(&m, base_path, &options)
            .map_err(|e| CliError::User(format!("generate failed for {}: {}", rt.id, e)))?;

        let output_paths: Vec<String> = generated.iter().map(|f| f.relative_path.clone()).collect();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let record = ActivationRecord {
            skill_name: name.clone(),
            version: entry.version.clone(),
            identity_hash: entry.identity_hash.clone(),
            activated_at: format!("{}", now),
            output_paths,
        };

        ActivationState::activate(&mgr, &rt.id, record)
            .map_err(|e| CliError::Internal(e.to_string()))?;

        activated.push(rt.id.clone());
    }

    if json {
        let value = serde_json::json!({
            "status": "ok",
            "skill": name,
            "activated_targets": activated,
        });
        output::print_json(&value);
    } else {
        println!("Activated \"{}\" for: {}", name, activated.join(", "));
    }

    Ok(())
}
