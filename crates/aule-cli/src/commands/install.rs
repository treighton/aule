use std::path::PathBuf;

use aule_cache::{CacheManager, MetadataIndex, IndexEntry, UserConfig, install_artifact};
use aule_resolver::{ResolveRequest, resolve_from_path, resolve_from_git, is_git_url, ArtifactSource};

use super::CliError;
use crate::output;
use crate::registry::{resolve_registry_url, RegistryClient, ResolveSkillRequest};

pub fn run(
    source: String,
    git_ref: Option<String>,
    version: Option<String>,
    target: Option<String>,
    json: bool,
) -> Result<(), CliError> {
    if source.starts_with('@') {
        run_registry(&source, version, target, json)
    } else if is_git_url(&source) {
        run_git(&source, git_ref.as_deref(), json)
    } else {
        let path = PathBuf::from(&source);
        run_local(path, json)
    }
}

fn run_registry(
    identifier: &str,
    version: Option<String>,
    target: Option<String>,
    json: bool,
) -> Result<(), CliError> {
    let mgr = CacheManager::new().map_err(|e| CliError::Internal(e.to_string()))?;
    let config = UserConfig::load(&mgr).map_err(|e| CliError::Internal(e.to_string()))?;
    let base_url = resolve_registry_url(None, config.registry_url.as_deref());
    let client = RegistryClient::new(base_url, config.auth_token.clone());

    if !json {
        println!("Resolving {} from registry...", identifier);
    }

    let req = ResolveSkillRequest {
        skill: identifier.to_string(),
        version,
        target: target.clone(),
    };

    let resolved = client.resolve_skill(&req)?;

    if !json {
        println!("Cloning from {}...", resolved.repo_url);
    }

    // Clone into temp dir
    let temp_dir = std::env::temp_dir().join(format!("skill-install-{}", std::process::id()));
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    let clone_status = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            &resolved.git_ref,
            &resolved.repo_url,
            &temp_dir.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| CliError::Internal(format!("failed to run git clone: {}", e)))?;

    if !clone_status.success() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(CliError::User(format!(
            "failed to clone {} (ref: {})",
            resolved.repo_url, resolved.git_ref
        )));
    }

    // Locate skill within the repo
    let skill_dir = if resolved.skill_path == "." || resolved.skill_path.is_empty() {
        temp_dir.clone()
    } else {
        temp_dir.join(&resolved.skill_path)
    };

    // Validate skill.yaml
    let manifest_path = skill_dir.join("skill.yaml");
    if !manifest_path.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(CliError::User(format!(
            "no skill.yaml found at {} in the cloned repository",
            resolved.skill_path
        )));
    }

    // Use the existing local-path resolution + install flow
    let skill_name = identifier
        .trim_start_matches('@')
        .replace('/', "__");

    let request = ResolveRequest {
        skill_name: skill_name.clone(),
        version_constraint: None,
        runtime_target: target.clone(),
        local_path: Some(skill_dir.clone()),
    };

    let plan = resolve_from_path(&skill_dir, &request)
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&temp_dir);
            CliError::User(format!("failed to resolve skill: {}", e))
        })?;

    let result = install_plan(&plan, &skill_dir, "registry", json);

    // Activate if a target was specified
    if result.is_ok() {
        if let Some(ref tgt) = target {
            if !json {
                println!("Activating for {}...", tgt);
            }
            // Delegate to the activate command
            let activate_result =
                super::activate::run(plan.skill_name.clone(), Some(tgt.clone()), json);
            if let Err(e) = activate_result {
                eprintln!("warning: installed but activation failed: {}", e);
            }
        }
    }

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    result
}

fn run_local(path: PathBuf, json: bool) -> Result<(), CliError> {
    let path = std::fs::canonicalize(&path)
        .map_err(|e| CliError::User(format!("invalid path: {}", e)))?;

    let request = ResolveRequest {
        skill_name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        version_constraint: None,
        runtime_target: None,
        local_path: Some(path.clone()),
    };

    let plan = resolve_from_path(&path, &request)
        .map_err(|e| CliError::User(e.to_string()))?;

    install_plan(&plan, &path, "local", json)
}

fn run_git(url: &str, git_ref: Option<&str>, json: bool) -> Result<(), CliError> {
    if !json {
        println!("Cloning from {}...", url);
    }

    let request = ResolveRequest {
        skill_name: skill_name_from_url(url),
        version_constraint: None,
        runtime_target: None,
        local_path: None,
    };

    let plan = resolve_from_git(url, git_ref, &request)
        .map_err(|e| CliError::User(e.to_string()))?;

    // The temp dir is inside the artifact source; we need the path for install_artifact.
    let temp_dir = match &plan.artifact_source {
        ArtifactSource::Git { temp_dir, .. } => temp_dir.clone(),
        _ => unreachable!(),
    };

    let result = install_plan(&plan, &temp_dir, "git", json);

    // Clean up temp directory regardless of success/failure
    let _ = std::fs::remove_dir_all(&temp_dir);

    result
}

/// Extract a skill name from a git URL (last path component minus `.git`).
fn skill_name_from_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let last = url.rsplit('/').next().unwrap_or(url);
    // Also handle SSH-style git@host:user/repo.git
    let last = last.rsplit(':').next().unwrap_or(last);
    last.trim_end_matches(".git").to_string()
}

fn install_plan(
    plan: &aule_resolver::InstallPlan,
    artifact_path: &std::path::Path,
    source_label: &str,
    json: bool,
) -> Result<(), CliError> {
    let mgr = CacheManager::new()
        .map_err(|e| CliError::Internal(e.to_string()))?;
    mgr.ensure_dirs()
        .map_err(|e| CliError::Internal(e.to_string()))?;

    let identity_hash = install_artifact(&mgr, artifact_path, &plan.skill_name, &plan.resolved_version)
        .map_err(|e| CliError::Internal(e.to_string()))?;

    let mut index = MetadataIndex::load(&mgr)
        .map_err(|e| CliError::Internal(e.to_string()))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    index.add_entry(IndexEntry {
        name: plan.skill_name.clone(),
        version: plan.resolved_version.clone(),
        identity_hash: identity_hash.clone(),
        installed_at: format!("{}", now),
        manifest_path: artifact_path.join("skill.yaml").to_string_lossy().to_string(),
        source: source_label.to_string(),
    });

    index.save(&mgr)
        .map_err(|e| CliError::Internal(e.to_string()))?;

    if json {
        let value = serde_json::json!({
            "status": "ok",
            "skill": plan.skill_name,
            "version": plan.resolved_version,
            "identity_hash": identity_hash,
            "source": source_label,
        });
        output::print_json(&value);
    } else {
        println!(
            "Installed {} v{} (hash: {})",
            plan.skill_name, plan.resolved_version, &identity_hash[..12]
        );
    }

    Ok(())
}
