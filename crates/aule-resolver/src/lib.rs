pub mod types;
pub mod error;
pub mod resolve;
pub mod policy;
pub mod compat;

pub use types::{ResolveRequest, InstallPlan, ResolvedAdapter, ArtifactSource};
pub use error::ResolveError;
pub use resolve::{resolve, resolve_from_path, resolve_from_cache, resolve_from_git, is_git_url};
pub use policy::evaluate_policy;
pub use compat::check_adapter_compatibility;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a minimal skill package directory with a skill.yaml.
    fn create_skill_dir(dir: &std::path::Path, name: &str, version: &str, permissions: &[&str], adapters: &[(&str, bool)]) {
        let perms_yaml: String = if permissions.is_empty() {
            "[]".to_string()
        } else {
            let items: Vec<String> = permissions.iter().map(|p| format!("\"{}\"", p)).collect();
            format!("[{}]", items.join(", "))
        };

        let adapters_yaml: String = adapters
            .iter()
            .map(|(id, enabled)| format!("  {}:\n    enabled: {}", id, enabled))
            .collect::<Vec<_>>()
            .join("\n");

        let yaml = format!(
            r#"schemaVersion: "0.1.0"
name: "{name}"
description: "A test skill"
version: "{version}"
content:
  skill: "content/skill.md"
contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions: {perms_yaml}
  determinism: "probabilistic"
adapters:
{adapters_yaml}
"#
        );

        fs::write(dir.join("skill.yaml"), yaml).unwrap();
        fs::create_dir_all(dir.join("content")).unwrap();
        fs::write(dir.join("content/skill.md"), "# Test skill").unwrap();
    }

    /// Helper: create a cache metadata index.json.
    fn create_cache_index(cache_root: &std::path::Path, entries: &[types::CacheIndexEntry]) {
        let metadata_dir = cache_root.join("metadata");
        fs::create_dir_all(&metadata_dir).unwrap();
        let json = serde_json::to_string_pretty(entries).unwrap();
        fs::write(metadata_dir.join("index.json"), json).unwrap();
    }

    #[test]
    fn resolve_from_local_path_success() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        create_skill_dir(&skill_dir, "my-skill", "1.2.3", &["filesystem.read"], &[("claude-code", true)]);

        let request = ResolveRequest {
            skill_name: "my-skill".to_string(),
            version_constraint: None,
            runtime_target: None,
            local_path: Some(skill_dir.clone()),
        };

        let plan = resolve_from_path(&skill_dir, &request).unwrap();
        assert_eq!(plan.skill_name, "my-skill");
        assert_eq!(plan.resolved_version, "1.2.3");
        assert_eq!(plan.contract_version, "1.0.0");
        assert_eq!(plan.permissions, vec!["filesystem.read".to_string()]);
        assert_eq!(plan.adapters.len(), 1);
        assert_eq!(plan.adapters[0].runtime_id, "claude-code");
        assert!(plan.adapters[0].enabled);
        assert!(matches!(plan.artifact_source, ArtifactSource::LocalPath(_)));
    }

    #[test]
    fn resolve_from_cache_success() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![types::CacheIndexEntry {
            name: "cached-skill".to_string(),
            version: "2.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            permissions: vec!["network.external".to_string()],
            adapters: vec![
                types::CacheAdapterEntry { runtime_id: "claude-code".to_string(), enabled: true },
                types::CacheAdapterEntry { runtime_id: "codex".to_string(), enabled: true },
            ],
            identity_hash: "abc123".to_string(),
        }];
        create_cache_index(tmp.path(), &entries);

        let request = ResolveRequest {
            skill_name: "cached-skill".to_string(),
            version_constraint: None,
            runtime_target: None,
            local_path: None,
        };

        let plan = resolve_from_cache(&request, tmp.path()).unwrap();
        assert_eq!(plan.skill_name, "cached-skill");
        assert_eq!(plan.resolved_version, "2.0.0");
        assert!(matches!(plan.artifact_source, ArtifactSource::Cache(ref h) if h == "abc123"));
    }

    #[test]
    fn resolve_from_cache_no_matching_version() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![types::CacheIndexEntry {
            name: "my-skill".to_string(),
            version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            permissions: vec![],
            adapters: vec![],
            identity_hash: "hash1".to_string(),
        }];
        create_cache_index(tmp.path(), &entries);

        let request = ResolveRequest {
            skill_name: "my-skill".to_string(),
            version_constraint: Some("^2.0.0".to_string()),
            runtime_target: None,
            local_path: None,
        };

        let err = resolve_from_cache(&request, tmp.path()).unwrap_err();
        assert!(matches!(err, ResolveError::NoMatchingVersion { .. }));
    }

    #[test]
    fn resolve_from_cache_version_constraint_match() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![
            types::CacheIndexEntry {
                name: "my-skill".to_string(),
                version: "1.0.0".to_string(),
                contract_version: "1.0.0".to_string(),
                permissions: vec![],
                adapters: vec![],
                identity_hash: "hash1".to_string(),
            },
            types::CacheIndexEntry {
                name: "my-skill".to_string(),
                version: "1.5.0".to_string(),
                contract_version: "1.0.0".to_string(),
                permissions: vec![],
                adapters: vec![],
                identity_hash: "hash2".to_string(),
            },
            types::CacheIndexEntry {
                name: "my-skill".to_string(),
                version: "2.0.0".to_string(),
                contract_version: "2.0.0".to_string(),
                permissions: vec![],
                adapters: vec![],
                identity_hash: "hash3".to_string(),
            },
        ];
        create_cache_index(tmp.path(), &entries);

        let request = ResolveRequest {
            skill_name: "my-skill".to_string(),
            version_constraint: Some("^1.0.0".to_string()),
            runtime_target: None,
            local_path: None,
        };

        let plan = resolve_from_cache(&request, tmp.path()).unwrap();
        assert_eq!(plan.resolved_version, "1.5.0");
        assert!(matches!(plan.artifact_source, ArtifactSource::Cache(ref h) if h == "hash2"));
    }

    #[test]
    fn permission_blocked_by_policy() {
        let tmp = TempDir::new().unwrap();
        let config = serde_json::json!({
            "blocked_permissions": ["process.spawn"]
        });
        let config_path = tmp.path().join("config.json");
        fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

        let plan = InstallPlan {
            skill_name: "dangerous-skill".to_string(),
            resolved_version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            adapters: vec![],
            artifact_source: ArtifactSource::LocalPath(tmp.path().to_path_buf()),
            permissions: vec!["filesystem.read".to_string(), "process.spawn".to_string()],
            risk_tier: aule_schema::permissions::RiskTier::High,
        };

        let err = evaluate_policy(&plan, &config_path).unwrap_err();
        assert!(matches!(err, ResolveError::PermissionBlocked { ref permission } if permission == "process.spawn"));
    }

    #[test]
    fn policy_allows_when_no_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("nonexistent-config.json");

        let plan = InstallPlan {
            skill_name: "safe-skill".to_string(),
            resolved_version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            adapters: vec![],
            artifact_source: ArtifactSource::LocalPath(tmp.path().to_path_buf()),
            permissions: vec!["process.spawn".to_string()],
            risk_tier: aule_schema::permissions::RiskTier::High,
        };

        assert!(evaluate_policy(&plan, &config_path).is_ok());
    }

    #[test]
    fn no_compatible_adapter() {
        let plan = InstallPlan {
            skill_name: "my-skill".to_string(),
            resolved_version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            adapters: vec![
                ResolvedAdapter { runtime_id: "codex".to_string(), enabled: true },
            ],
            artifact_source: ArtifactSource::Cache("hash".to_string()),
            permissions: vec![],
            risk_tier: aule_schema::permissions::RiskTier::None,
        };

        let err = check_adapter_compatibility(&plan, Some("claude-code")).unwrap_err();
        assert!(matches!(err, ResolveError::NoCompatibleAdapter { ref target, .. } if target == "claude-code"));
    }

    #[test]
    fn adapter_compatibility_passes_when_present() {
        let plan = InstallPlan {
            skill_name: "my-skill".to_string(),
            resolved_version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            adapters: vec![
                ResolvedAdapter { runtime_id: "claude-code".to_string(), enabled: true },
                ResolvedAdapter { runtime_id: "codex".to_string(), enabled: false },
            ],
            artifact_source: ArtifactSource::Cache("hash".to_string()),
            permissions: vec![],
            risk_tier: aule_schema::permissions::RiskTier::None,
        };

        assert!(check_adapter_compatibility(&plan, Some("claude-code")).is_ok());
        // codex is disabled, so it should fail
        assert!(check_adapter_compatibility(&plan, Some("codex")).is_err());
    }

    #[test]
    fn adapter_compatibility_no_target_always_passes() {
        let plan = InstallPlan {
            skill_name: "my-skill".to_string(),
            resolved_version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            adapters: vec![],
            artifact_source: ArtifactSource::Cache("hash".to_string()),
            permissions: vec![],
            risk_tier: aule_schema::permissions::RiskTier::None,
        };

        assert!(check_adapter_compatibility(&plan, None).is_ok());
    }

    #[test]
    fn resolve_falls_through_to_cache() {
        let tmp = TempDir::new().unwrap();
        let cache_root = tmp.path().join("cache");
        fs::create_dir_all(&cache_root).unwrap();

        let entries = vec![types::CacheIndexEntry {
            name: "fallback-skill".to_string(),
            version: "1.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            permissions: vec![],
            adapters: vec![types::CacheAdapterEntry {
                runtime_id: "claude-code".to_string(),
                enabled: true,
            }],
            identity_hash: "fallback-hash".to_string(),
        }];
        create_cache_index(&cache_root, &entries);

        let request = ResolveRequest {
            skill_name: "fallback-skill".to_string(),
            version_constraint: None,
            runtime_target: None,
            local_path: None,
        };

        let plan = resolve(&request, &cache_root).unwrap();
        assert_eq!(plan.skill_name, "fallback-skill");
        assert!(matches!(plan.artifact_source, ArtifactSource::Cache(_)));
    }

    #[test]
    fn resolve_prefers_local_path() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("local-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        create_skill_dir(&skill_dir, "my-skill", "1.0.0", &[], &[("claude-code", true)]);

        let cache_root = tmp.path().join("cache");
        fs::create_dir_all(&cache_root).unwrap();
        let entries = vec![types::CacheIndexEntry {
            name: "my-skill".to_string(),
            version: "2.0.0".to_string(),
            contract_version: "1.0.0".to_string(),
            permissions: vec![],
            adapters: vec![],
            identity_hash: "cache-hash".to_string(),
        }];
        create_cache_index(&cache_root, &entries);

        let request = ResolveRequest {
            skill_name: "my-skill".to_string(),
            version_constraint: None,
            runtime_target: None,
            local_path: Some(skill_dir),
        };

        let plan = resolve(&request, &cache_root).unwrap();
        // Should use local path, version 1.0.0, not cache version 2.0.0
        assert_eq!(plan.resolved_version, "1.0.0");
        assert!(matches!(plan.artifact_source, ArtifactSource::LocalPath(_)));
    }

    #[test]
    fn resolve_returns_skill_not_found() {
        let tmp = TempDir::new().unwrap();
        let cache_root = tmp.path().join("empty-cache");
        // No cache dir at all

        let request = ResolveRequest {
            skill_name: "nonexistent".to_string(),
            version_constraint: None,
            runtime_target: None,
            local_path: None,
        };

        let err = resolve(&request, &cache_root).unwrap_err();
        assert!(matches!(err, ResolveError::SkillNotFound(_)));
    }

    #[test]
    fn is_git_url_detects_https() {
        assert!(resolve::is_git_url("https://github.com/user/repo"));
        assert!(resolve::is_git_url("https://github.com/user/repo.git"));
    }

    #[test]
    fn is_git_url_detects_git_protocol() {
        assert!(resolve::is_git_url("git://github.com/user/repo.git"));
    }

    #[test]
    fn is_git_url_detects_ssh() {
        assert!(resolve::is_git_url("git@github.com:user/repo.git"));
    }

    #[test]
    fn is_git_url_detects_dot_git_suffix() {
        assert!(resolve::is_git_url("something.git"));
    }

    #[test]
    fn is_git_url_rejects_local_paths() {
        assert!(!resolve::is_git_url("./my-skill"));
        assert!(!resolve::is_git_url("/home/user/my-skill"));
        assert!(!resolve::is_git_url("my-skill"));
        assert!(!resolve::is_git_url("../relative/path"));
    }

    #[test]
    #[ignore] // requires network access
    fn resolve_from_git_invalid_url_returns_error() {
        let request = ResolveRequest {
            skill_name: "nonexistent".to_string(),
            version_constraint: None,
            runtime_target: None,
            local_path: None,
        };

        let err = resolve::resolve_from_git(
            "https://github.com/not-a-real-org-xyz/not-a-real-repo-xyz.git",
            None,
            &request,
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::GitCloneFailed { .. }));
    }
}
