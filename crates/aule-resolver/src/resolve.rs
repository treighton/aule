use std::path::Path;

use semver::{Version, VersionReq};
use tempfile::TempDir;

use aule_schema::contract::Contract;
use aule_schema::manifest::{ContractRef, Manifest};
use aule_schema::permissions::{max_risk_tier, RiskTier};

use crate::error::ResolveError;
use crate::types::{ArtifactSource, CacheIndexEntry, InstallPlan, ResolveRequest, ResolvedAdapter};

/// Build an install plan from a local skill directory.
pub fn resolve_from_path(
    path: &Path,
    request: &ResolveRequest,
) -> Result<InstallPlan, ResolveError> {
    let manifest_path = path.join("skill.yaml");
    let content = std::fs::read_to_string(&manifest_path).map_err(|_| {
        ResolveError::ManifestError(format!(
            "could not read skill.yaml at {}",
            manifest_path.display()
        ))
    })?;

    let manifest: Manifest = serde_yaml::from_str(&content)
        .map_err(|e| ResolveError::ManifestError(format!("YAML parse error: {e}")))?;

    // Check version constraint if provided
    if let Some(ref constraint_str) = request.version_constraint {
        let req = VersionReq::parse(constraint_str).map_err(|e| {
            ResolveError::ManifestError(format!("invalid version constraint \"{constraint_str}\": {e}"))
        })?;
        let ver = Version::parse(&manifest.version).map_err(|e| {
            ResolveError::ManifestError(format!(
                "invalid manifest version \"{}\": {e}",
                manifest.version
            ))
        })?;
        if !req.matches(&ver) {
            return Err(ResolveError::NoMatchingVersion {
                name: manifest.name.clone(),
                constraint: constraint_str.clone(),
            });
        }
    }

    let (contract_version, permissions) = extract_contract_info(&manifest, path)?;
    let risk_tier = if permissions.is_empty() {
        RiskTier::None
    } else {
        max_risk_tier(&permissions)
    };

    let adapters: Vec<ResolvedAdapter> = manifest
        .adapters
        .iter()
        .map(|(id, cfg)| ResolvedAdapter {
            runtime_id: id.clone(),
            enabled: cfg.enabled,
        })
        .collect();

    Ok(InstallPlan {
        skill_name: manifest.name,
        resolved_version: manifest.version,
        contract_version,
        adapters,
        artifact_source: ArtifactSource::LocalPath(path.to_path_buf()),
        permissions,
        risk_tier,
    })
}

/// Build an install plan from the cache metadata index.
pub fn resolve_from_cache(
    request: &ResolveRequest,
    cache_root: &Path,
) -> Result<InstallPlan, ResolveError> {
    let index_path = cache_root.join("metadata").join("index.json");
    let content = std::fs::read_to_string(&index_path).map_err(ResolveError::IoError)?;
    let entries: Vec<CacheIndexEntry> =
        serde_json::from_str(&content).map_err(|e| ResolveError::ManifestError(e.to_string()))?;

    // Filter by name
    let matching: Vec<&CacheIndexEntry> = entries
        .iter()
        .filter(|e| e.name == request.skill_name)
        .collect();

    if matching.is_empty() {
        return Err(ResolveError::SkillNotFound(request.skill_name.clone()));
    }

    // Filter by version constraint if provided
    let entry = if let Some(ref constraint_str) = request.version_constraint {
        let req = VersionReq::parse(constraint_str).map_err(|e| {
            ResolveError::ManifestError(format!("invalid version constraint \"{constraint_str}\": {e}"))
        })?;
        matching
            .into_iter()
            .filter(|e| {
                Version::parse(&e.version)
                    .map(|v| req.matches(&v))
                    .unwrap_or(false)
            })
            .max_by(|a, b| {
                let va = Version::parse(&a.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                let vb = Version::parse(&b.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                va.cmp(&vb)
            })
            .ok_or_else(|| ResolveError::NoMatchingVersion {
                name: request.skill_name.clone(),
                constraint: constraint_str.clone(),
            })?
    } else {
        // Pick the latest version
        matching
            .into_iter()
            .max_by(|a, b| {
                let va = Version::parse(&a.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                let vb = Version::parse(&b.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                va.cmp(&vb)
            })
            .unwrap() // safe: we already checked non-empty
    };

    let risk_tier = if entry.permissions.is_empty() {
        RiskTier::None
    } else {
        max_risk_tier(&entry.permissions)
    };

    let adapters: Vec<ResolvedAdapter> = entry
        .adapters
        .iter()
        .map(|a| ResolvedAdapter {
            runtime_id: a.runtime_id.clone(),
            enabled: a.enabled,
        })
        .collect();

    Ok(InstallPlan {
        skill_name: entry.name.clone(),
        resolved_version: entry.version.clone(),
        contract_version: entry.contract_version.clone(),
        adapters,
        artifact_source: ArtifactSource::Cache(entry.identity_hash.clone()),
        permissions: entry.permissions.clone(),
        risk_tier,
    })
}

/// Returns `true` if the given string looks like a git URL.
pub fn is_git_url(s: &str) -> bool {
    s.starts_with("https://")
        || s.starts_with("git://")
        || s.starts_with("git@")
        || s.ends_with(".git")
}

/// Build an install plan by cloning a git repository.
///
/// Clones the repo to a temporary directory, reads `skill.yaml`, and returns an
/// [`InstallPlan`] whose `artifact_source` is [`ArtifactSource::Git`].  The
/// caller is responsible for cleaning up the temp directory after installation.
pub fn resolve_from_git(
    url: &str,
    git_ref: Option<&str>,
    request: &ResolveRequest,
) -> Result<InstallPlan, ResolveError> {
    let tmp_dir = TempDir::new().map_err(ResolveError::IoError)?.keep();

    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(r) = git_ref {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg(url).arg(&tmp_dir);

    let output = cmd.output().map_err(|e| ResolveError::GitCloneFailed {
        url: url.to_string(),
        reason: format!("failed to run git: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Clean up temp dir on failure
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(ResolveError::GitCloneFailed {
            url: url.to_string(),
            reason: stderr.trim().to_string(),
        });
    }

    let plan_result = resolve_from_path(&tmp_dir, request);

    match plan_result {
        Ok(mut plan) => {
            plan.artifact_source = ArtifactSource::Git {
                url: url.to_string(),
                temp_dir: tmp_dir,
            };
            Ok(plan)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            Err(e)
        }
    }
}

/// Resolve a skill: tries local path first, then cache, then returns SkillNotFound.
pub fn resolve(
    request: &ResolveRequest,
    cache_root: &Path,
) -> Result<InstallPlan, ResolveError> {
    if let Some(ref local_path) = request.local_path {
        return resolve_from_path(local_path, request);
    }

    match resolve_from_cache(request, cache_root) {
        Ok(plan) => Ok(plan),
        Err(ResolveError::IoError(_)) | Err(ResolveError::SkillNotFound(_)) => {
            Err(ResolveError::SkillNotFound(request.skill_name.clone()))
        }
        Err(e) => Err(e),
    }
}

/// Extract contract version and permissions from a manifest.
fn extract_contract_info(
    manifest: &Manifest,
    base_path: &Path,
) -> Result<(String, Vec<String>), ResolveError> {
    match &manifest.contract {
        ContractRef::Inline(contract) => {
            Ok((contract.version.clone(), contract.permissions.clone()))
        }
        ContractRef::File(file_path) => {
            let full_path = base_path.join(file_path);
            let content = std::fs::read_to_string(&full_path).map_err(|_| {
                ResolveError::ManifestError(format!(
                    "could not read contract file: {}",
                    full_path.display()
                ))
            })?;
            let contract: Contract = serde_yaml::from_str(&content)
                .map_err(|e| ResolveError::ManifestError(format!("contract parse error: {e}")))?;
            Ok((contract.version, contract.permissions))
        }
    }
}
