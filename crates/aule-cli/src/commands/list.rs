use aule_adapter::RuntimeTarget;
use aule_cache::{ActivationState, CacheManager, MetadataIndex};

use super::CliError;
use crate::output;

pub fn run(_installed: bool, active: bool, json: bool) -> Result<(), CliError> {
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;

    let index =
        MetadataIndex::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;

    // Gather activation state across all known runtimes
    let known_targets = RuntimeTarget::all_known();
    let mut activation_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for rt in &known_targets {
        let records = ActivationState::list_active(&mgr, &rt.id).unwrap_or_default();
        for r in records {
            activation_map
                .entry(r.skill_name.clone())
                .or_default()
                .push(rt.id.clone());
        }
    }

    let entries = index.list_installed();

    if json {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                let active_for = activation_map
                    .get(&e.name)
                    .cloned()
                    .unwrap_or_default();
                serde_json::json!({
                    "name": e.name,
                    "version": e.version,
                    "active_for": active_for,
                })
            })
            .collect();
        output::print_json(&serde_json::json!({ "skills": items }));
    } else {
        if entries.is_empty() {
            println!("No skills installed.");
            return Ok(());
        }

        // Simple table output
        println!("{:<30} {:<12} {}", "NAME", "VERSION", "ACTIVE FOR");
        println!("{}", "-".repeat(60));
        for e in &entries {
            let active_for = activation_map
                .get(&e.name)
                .map(|v| v.join(", "))
                .unwrap_or_else(|| "-".to_string());

            // Filter based on flags
            if active && !activation_map.contains_key(&e.name) {
                continue;
            }
            // --installed is the default (show all), so no filtering needed

            println!("{:<30} {:<12} {}", e.name, e.version, active_for);
        }
    }

    Ok(())
}
