use std::path::Path;

use serde::Deserialize;

use crate::error::ResolveError;
use crate::types::InstallPlan;

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    #[serde(default)]
    blocked_permissions: Vec<String>,
}

/// Evaluate the install plan against a policy config.
///
/// Reads `config.json` at `config_path` if it exists. If any permission in the
/// plan is on the blocklist, returns `ResolveError::PermissionBlocked`.
/// If no config file exists, everything is allowed.
pub fn evaluate_policy(plan: &InstallPlan, config_path: &Path) -> Result<(), ResolveError> {
    if !config_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(config_path).map_err(ResolveError::IoError)?;
    let config: PolicyConfig =
        serde_json::from_str(&content).map_err(|e| ResolveError::ManifestError(e.to_string()))?;

    for perm in &plan.permissions {
        if config.blocked_permissions.contains(perm) {
            return Err(ResolveError::PermissionBlocked {
                permission: perm.clone(),
            });
        }
    }

    Ok(())
}
