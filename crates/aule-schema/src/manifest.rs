use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::contract::Contract;
use crate::validation::{ValidationResult, ValidationMessage, Severity};

// --- Core Manifest Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub content: ContentPaths,
    pub contract: ContractRef,

    #[serde(default)]
    pub identity: Option<String>,

    #[serde(default)]
    pub adapters: HashMap<String, AdapterConfig>,

    #[serde(default)]
    pub dependencies: Option<Dependencies>,

    #[serde(default)]
    pub metadata: Option<ManifestMetadata>,

    #[serde(default)]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPaths {
    pub skill: String,
    #[serde(default)]
    pub commands: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContractRef {
    Inline(Contract),
    File(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub enabled: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub skills: Vec<SkillDependency>,
    #[serde(default)]
    pub tools: Vec<ToolDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependency {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Extra metadata fields passed through to adapter frontmatter.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// --- Errors ---

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest file not found: {0}")]
    NotFound(String),
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("validation failed: {0}")]
    Validation(String),
}

// --- Parsing ---

pub fn parse_manifest(yaml: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest = serde_yaml::from_str(yaml)?;
    Ok(manifest)
}

pub fn load_manifest(path: &Path) -> Result<Manifest, ManifestError> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| ManifestError::NotFound(path.display().to_string()))?;
    parse_manifest(&content)
}

// --- Validation ---

pub fn validate_manifest(manifest: &Manifest, base_path: Option<&Path>) -> ValidationResult {
    let mut result = ValidationResult::new();

    // schemaVersion check
    if manifest.schema_version != "0.1.0" {
        result.add_error(format!(
            "schemaVersion must be \"0.1.0\", got \"{}\"",
            manifest.schema_version
        ));
    }

    // name format: kebab-case, 1-100 chars
    if manifest.name.is_empty() || manifest.name.len() > 100 {
        result.add_error("name must be 1-100 characters".to_string());
    } else if !is_kebab_case(&manifest.name) {
        result.add_error(format!(
            "name must be kebab-case (lowercase alphanumeric and hyphens), got \"{}\"",
            manifest.name
        ));
    }

    // description: 1-500 chars
    if manifest.description.is_empty() || manifest.description.len() > 500 {
        result.add_error("description must be 1-500 characters".to_string());
    }

    // version: semver
    if semver_parse(&manifest.version).is_none() {
        result.add_error(format!(
            "version must be valid semver, got \"{}\"",
            manifest.version
        ));
    }

    // identity format (optional)
    if let Some(ref identity) = manifest.identity {
        if !is_valid_identity(identity) {
            result.add_error(format!(
                "identity must be a valid domain/path string, got \"{}\"",
                identity
            ));
        }
    }

    // tags limit
    if let Some(ref metadata) = manifest.metadata {
        if let Some(ref tags) = metadata.tags {
            if tags.len() > 10 {
                result.add_error(format!(
                    "tags must have at most 10 entries, got {}",
                    tags.len()
                ));
            }
        }
    }

    // content path validation (if base_path provided)
    if let Some(base) = base_path {
        let skill_path = base.join(&manifest.content.skill);
        if !skill_path.exists() {
            result.add_error(format!(
                "content.skill file not found: {}",
                manifest.content.skill
            ));
        }

        if let Some(ref commands) = manifest.content.commands {
            for (name, path) in commands {
                let cmd_path = base.join(path);
                if !cmd_path.exists() {
                    result.add_error(format!(
                        "content.commands.{} file not found: {}",
                        name, path
                    ));
                }
            }
        }
    }

    // unknown adapter targets (warning, not error)
    let known_targets = ["claude-code", "codex"];
    for target in manifest.adapters.keys() {
        if !known_targets.contains(&target.as_str()) {
            result.add(ValidationMessage {
                severity: Severity::Warning,
                message: format!("unknown adapter target \"{}\", will be skipped", target),
            });
        }
    }

    result
}

// --- Helpers ---

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

fn is_valid_identity(s: &str) -> bool {
    // domain/path format: at least one dot in domain, then a slash, then path
    if let Some(slash_pos) = s.find('/') {
        let domain = &s[..slash_pos];
        let path = &s[slash_pos + 1..];
        domain.contains('.') && !domain.is_empty() && !path.is_empty()
    } else {
        false
    }
}

fn semver_parse(s: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse::<u64>().ok()?;
    let minor = parts[1].parse::<u64>().ok()?;
    let patch = parts[2].parse::<u64>().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"
schemaVersion: "0.1.0"
name: "openspec-explore"
description: "Enter explore mode for thinking through ideas"
version: "1.0.0"
content:
  skill: "content/skill.md"
contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions: []
  determinism: "probabilistic"
adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true
"#;

    #[test]
    fn parse_valid_manifest() {
        let manifest = parse_manifest(VALID_MANIFEST).unwrap();
        assert_eq!(manifest.name, "openspec-explore");
        assert_eq!(manifest.schema_version, "0.1.0");
        assert_eq!(manifest.adapters.len(), 2);
        assert!(manifest.adapters["claude-code"].enabled);
    }

    #[test]
    fn validate_valid_manifest() {
        let manifest = parse_manifest(VALID_MANIFEST).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(result.is_ok(), "errors: {:?}", result.errors());
    }

    #[test]
    fn validate_bad_name() {
        let yaml = VALID_MANIFEST.replace("openspec-explore", "Invalid Name!");
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(!result.is_ok());
        assert!(result.errors().iter().any(|e| e.contains("kebab-case")));
    }

    #[test]
    fn validate_bad_schema_version() {
        let yaml = VALID_MANIFEST.replace("\"0.1.0\"", "\"2.0.0\"");
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(!result.is_ok());
    }

    #[test]
    fn validate_tags_limit() {
        let yaml = format!(
            "{}\nmetadata:\n  tags: [\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\",\"i\",\"j\",\"k\"]",
            VALID_MANIFEST
        );
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(!result.is_ok());
        assert!(result.errors().iter().any(|e| e.contains("tags")));
    }

    #[test]
    fn validate_identity_format() {
        let yaml = VALID_MANIFEST.replace(
            "version: \"1.0.0\"\ncontent:",
            "version: \"1.0.0\"\nidentity: \"skills.acme.dev/workflow/explore\"\ncontent:",
        );
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_bad_identity() {
        let yaml = VALID_MANIFEST.replace(
            "version: \"1.0.0\"\ncontent:",
            "version: \"1.0.0\"\nidentity: \"no spaces allowed\"\ncontent:",
        );
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(!result.is_ok());
    }

    #[test]
    fn validate_unknown_adapter_warns() {
        let yaml = VALID_MANIFEST.replace(
            "codex:\n    enabled: true",
            "codex:\n    enabled: true\n  unknown-runtime:\n    enabled: true",
        );
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(result.is_ok()); // warnings don't fail
        assert!(result.warnings().iter().any(|w| w.contains("unknown adapter")));
    }

    #[test]
    fn parse_missing_required_field() {
        let yaml = r#"
schemaVersion: "0.1.0"
name: "test"
description: "test"
"#;
        let result = parse_manifest(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn validate_content_path_missing() {
        let manifest = parse_manifest(VALID_MANIFEST).unwrap();
        let tmp = std::env::temp_dir().join("aule-test-empty");
        let _ = std::fs::create_dir_all(&tmp);
        let result = validate_manifest(&manifest, Some(&tmp));
        assert!(!result.is_ok());
        assert!(result.errors().iter().any(|e| e.contains("content.skill")));
    }

    #[test]
    fn validate_extension_namespaces() {
        let yaml = format!(
            "{}\nextensions:\n  vendor:\n    claude:\n      maxTokens: 4096",
            VALID_MANIFEST
        );
        let manifest = parse_manifest(&yaml).unwrap();
        let result = validate_manifest(&manifest, None);
        assert!(result.is_ok());
        assert!(manifest.extensions.is_some());
    }
}
