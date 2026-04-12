use std::path::PathBuf;
use std::process::Command;

use aule_cache::{CacheManager, UserConfig};

use super::validate;
use super::CliError;
use crate::output;
use crate::registry::{resolve_registry_url, RegisterSkillRequest, RegistryClient};

pub fn run(
    path: Option<PathBuf>,
    git_ref_override: Option<String>,
    json: bool,
) -> Result<(), CliError> {
    let base_path = path.unwrap_or_else(|| PathBuf::from("."));
    let base_path = std::fs::canonicalize(&base_path)
        .map_err(|e| CliError::User(format!("invalid path: {}", e)))?;

    // Validate the skill locally
    let _manifest = validate::validate_and_load(&base_path)?;

    if !json {
        println!("Skill validated successfully.");
    }

    // Detect git remote URL
    let repo_url = git_output(&base_path, &["remote", "get-url", "origin"])
        .map_err(|_| {
            CliError::User(
                "could not detect git remote — is this a git repository with an 'origin' remote?"
                    .to_string(),
            )
        })?;

    // Detect current ref
    let git_ref = if let Some(r) = git_ref_override {
        r
    } else {
        git_output(&base_path, &["rev-parse", "--abbrev-ref", "HEAD"]).map_err(|_| {
            CliError::User("could not detect current git branch".to_string())
        })?
    };

    // Detect skill_path relative to repo root
    let repo_root = git_output(&base_path, &["rev-parse", "--show-toplevel"])
        .map_err(|_| CliError::User("could not detect git repo root".to_string()))?;

    let skill_path = base_path
        .strip_prefix(&repo_root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let skill_path = if skill_path.is_empty() {
        ".".to_string()
    } else {
        skill_path
    };

    // Load config for auth
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;
    let config = UserConfig::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;
    let base_url = resolve_registry_url(None, config.registry_url.as_deref());

    let client = RegistryClient::new(base_url, config.auth_token.clone());

    let req = RegisterSkillRequest {
        repo_url: repo_url.clone(),
        skill_path: skill_path.clone(),
        git_ref: git_ref.clone(),
    };

    if !json {
        println!("Publishing from {} (ref: {}, path: {})...", repo_url, git_ref, skill_path);
    }

    let resp = client.register_skill(&req)?;

    if json {
        let value = serde_json::json!({
            "status": resp.status,
            "skill_id": resp.skill_id,
            "repo_url": repo_url,
            "ref": git_ref,
            "skill_path": skill_path,
            "message": resp.message,
        });
        output::print_json(&value);
    } else {
        if let Some(msg) = &resp.message {
            println!("{}", msg);
        } else {
            println!("Published successfully.");
        }
        if let Some(id) = &resp.skill_id {
            println!("Skill ID: {}", id);
        }
    }

    Ok(())
}

/// Run a git command and return trimmed stdout.
fn git_output(cwd: &std::path::Path, args: &[&str]) -> Result<String, CliError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| CliError::Internal(format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(CliError::Internal(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
