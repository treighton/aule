use aule_cache::{CacheManager, PublisherInfo, UserConfig};

use super::CliError;
use crate::output;
use crate::registry::{resolve_registry_url, RegistryClient};

pub fn run(registry: Option<String>, json: bool) -> Result<(), CliError> {
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;
    mgr.ensure_dirs()
        .map_err(|e| CliError::Internal(e.to_string()))?;

    let config = UserConfig::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;
    let base_url = resolve_registry_url(registry.as_deref(), config.registry_url.as_deref());

    let client = RegistryClient::new(base_url.clone(), None);

    // Start device auth flow
    let start = client.device_start()?;

    // Try to open browser
    let browser_opened = open::that(&start.verification_url).is_ok();

    if !json {
        if browser_opened {
            println!("Opened browser to authenticate.");
        } else {
            println!("Open this URL to authenticate:");
        }
        println!();
        println!("  {}", start.verification_url);
        println!();
        println!("Your code: {}", start.user_code);
        println!();
        println!("Waiting for authentication...");
    }

    // Poll for completion
    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));

        let poll = client.device_poll(&start.device_code)?;

        match poll.status.as_str() {
            "complete" | "authorized" => {
                let token = poll.api_token.ok_or_else(|| {
                    CliError::Internal("auth completed but no token returned".to_string())
                })?;

                // Save to config
                let mut config =
                    UserConfig::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;
                config.auth_token = Some(token);
                config.registry_url = Some(base_url.clone());
                if let Some(pub_info) = poll.publisher {
                    config.publisher = Some(PublisherInfo {
                        github_username: pub_info.github_username,
                        display_name: pub_info.display_name,
                    });
                }
                config
                    .save(&mgr)
                    .map_err(|e| CliError::Internal(e.to_string()))?;

                if json {
                    let value = serde_json::json!({
                        "status": "ok",
                        "registry": base_url,
                        "publisher": config.publisher.as_ref().and_then(|p| p.github_username.as_deref()),
                    });
                    output::print_json(&value);
                } else {
                    let who = config
                        .publisher
                        .as_ref()
                        .and_then(|p| {
                            p.display_name
                                .as_deref()
                                .or(p.github_username.as_deref())
                        })
                        .unwrap_or("unknown");
                    println!("Logged in as {} (registry: {})", who, base_url);
                }
                return Ok(());
            }
            "pending" | "authorization_pending" => {
                // keep polling
            }
            "expired" => {
                return Err(CliError::User(
                    "authentication timed out — please try again".to_string(),
                ));
            }
            other => {
                return Err(CliError::User(format!(
                    "unexpected auth status: {}",
                    other
                )));
            }
        }
    }
}
