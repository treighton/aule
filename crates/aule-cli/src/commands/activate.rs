use aule_adapter::{generate_any, GenerateOptions, RuntimeTarget};
use aule_cache::{ActivationRecord, ActivationState, CacheManager, MetadataIndex, execute_hook};
use aule_schema::manifest::{self, ManifestAny};

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

    // Load manifest from the stored manifest path (supports both v0.1.0 and v0.2.0)
    let manifest_path = std::path::Path::new(&entry.manifest_path);
    let manifest_any = manifest::load_manifest_any(manifest_path)
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
        manifest_any.adapters()
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

        let generated = generate_any(&manifest_any, base_path, &options)
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

    // Run onActivate hook if present (v0.2.0 manifests)
    if let ManifestAny::V2(ref m) = manifest_any {
        if let Some(ref hooks) = m.hooks {
            if let Some(ref on_activate) = hooks.on_activate {
                let hook_path = base_path.join(on_activate);
                if !json {
                    println!("Running onActivate hook...");
                }
                match execute_hook(&hook_path, base_path) {
                    Ok(result) => {
                        if result.success {
                            if !json {
                                println!("  onActivate hook completed successfully.");
                            }
                        } else {
                            eprintln!(
                                "warning: onActivate hook failed (exit {})",
                                result.exit_code.map_or("unknown".to_string(), |c| c.to_string())
                            );
                            if !result.stderr.is_empty() {
                                eprintln!("  {}", result.stderr.trim());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("warning: could not run onActivate hook: {}", e);
                    }
                }
            }
        }
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
