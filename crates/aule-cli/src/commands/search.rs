use aule_cache::{CacheManager, UserConfig};

use super::CliError;
use crate::output;
use crate::registry::{resolve_registry_url, RegistryClient};

pub fn run(
    query: String,
    runtime: Option<String>,
    limit: Option<u32>,
    json: bool,
) -> Result<(), CliError> {
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;
    let config = UserConfig::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;
    let base_url = resolve_registry_url(None, config.registry_url.as_deref());

    let client = RegistryClient::new(base_url, config.auth_token.clone());

    let resp = client.search(&query, runtime.as_deref(), limit)?;

    if json {
        let value = serde_json::json!({
            "results": resp.results.iter().map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "description": r.description,
                    "version": r.version,
                    "verified": r.verified,
                })
            }).collect::<Vec<_>>(),
            "total": resp.total,
        });
        output::print_json(&value);
    } else {
        if resp.results.is_empty() {
            println!("No skills found for \"{}\".", query);
            return Ok(());
        }

        // Print table header
        println!(
            "{:<30} {:<10} {:<8} {}",
            "NAME", "VERSION", "VERIFIED", "DESCRIPTION"
        );
        println!("{}", "-".repeat(78));

        for r in &resp.results {
            let version = r.version.as_deref().unwrap_or("-");
            let verified = match r.verified {
                Some(true) => "yes",
                Some(false) => "no",
                None => "-",
            };
            let desc = r.description.as_deref().unwrap_or("");
            // Truncate description for display
            let desc_display = if desc.len() > 30 {
                format!("{}...", &desc[..27])
            } else {
                desc.to_string()
            };
            println!("{:<30} {:<10} {:<8} {}", r.name, version, verified, desc_display);
        }

        if let Some(total) = resp.total {
            println!();
            println!("{} result(s) found.", total);
        }
    }

    Ok(())
}
