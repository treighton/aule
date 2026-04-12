use aule_cache::{CacheManager, UserConfig};

use super::CliError;
use crate::output;

pub fn run(json: bool) -> Result<(), CliError> {
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;
    mgr.ensure_dirs()
        .map_err(|e| CliError::Internal(e.to_string()))?;

    let mut config = UserConfig::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;

    if config.auth_token.is_none() {
        if json {
            let value = serde_json::json!({
                "status": "ok",
                "message": "not logged in",
            });
            output::print_json(&value);
        } else {
            println!("Not logged in.");
        }
        return Ok(());
    }

    config.auth_token = None;
    config.publisher = None;
    config
        .save(&mgr)
        .map_err(|e| CliError::Internal(e.to_string()))?;

    if json {
        let value = serde_json::json!({
            "status": "ok",
            "message": "logged out",
        });
        output::print_json(&value);
    } else {
        println!("Logged out.");
    }

    Ok(())
}
