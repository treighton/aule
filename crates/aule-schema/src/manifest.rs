use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::contract::{Contract, InputOutput, Determinism, ContractError, BehavioralMetadata};
use crate::validation::{ValidationResult, ValidationMessage, Severity};

// --- Core Manifest Types (v0.1.0) ---

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

// --- v0.2.0 Manifest Types ---

/// A skill definition within a v0.2.0 package. Each skill has its own entrypoint,
/// interface (inputs/outputs/permissions/determinism), and optional commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub description: String,
    pub entrypoint: String,
    pub version: String,

    #[serde(default)]
    pub inputs: Option<InputOutput>,
    #[serde(default)]
    pub outputs: Option<InputOutput>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default = "crate::contract::default_determinism")]
    pub determinism: Determinism,
    #[serde(default)]
    pub errors: Option<Vec<ContractError>>,
    #[serde(default)]
    pub behavior: Option<BehavioralMetadata>,
    #[serde(default)]
    pub commands: Option<HashMap<String, String>>,
}

/// An executable tool declared at the package level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub description: String,
    /// Runtime: "node", "python", "shell"
    pub using: String,
    #[serde(default)]
    pub version: Option<String>,
    pub entrypoint: String,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
}

/// Lifecycle hooks declared at the package level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hooks {
    #[serde(default)]
    pub on_install: Option<String>,
    #[serde(default)]
    pub on_activate: Option<String>,
    #[serde(default)]
    pub on_uninstall: Option<String>,
}

/// v0.2.0 manifest: multi-skill packages with tools, hooks, and file includes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestV2 {
    pub schema_version: String,
    pub name: String,
    pub description: String,
    pub version: String,

    /// Glob patterns for files bundled with the package.
    pub files: Vec<String>,

    /// Map of skill name → skill definition.
    pub skills: HashMap<String, SkillDefinition>,

    /// Map of tool name → tool definition. Optional.
    #[serde(default)]
    pub tools: Option<HashMap<String, Tool>>,

    /// Lifecycle hooks. Optional.
    #[serde(default)]
    pub hooks: Option<Hooks>,

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

/// Version-dispatched manifest: either v0.1.0 or v0.2.0.
#[derive(Debug, Clone)]
pub enum ManifestAny {
    V1(Manifest),
    V2(ManifestV2),
}

impl ManifestAny {
    pub fn name(&self) -> &str {
        match self {
            ManifestAny::V1(m) => &m.name,
            ManifestAny::V2(m) => &m.name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ManifestAny::V1(m) => &m.description,
            ManifestAny::V2(m) => &m.description,
        }
    }

    pub fn version(&self) -> &str {
        match self {
            ManifestAny::V1(m) => &m.version,
            ManifestAny::V2(m) => &m.version,
        }
    }

    pub fn schema_version(&self) -> &str {
        match self {
            ManifestAny::V1(m) => &m.schema_version,
            ManifestAny::V2(m) => &m.schema_version,
        }
    }

    pub fn adapters(&self) -> &HashMap<String, AdapterConfig> {
        match self {
            ManifestAny::V1(m) => &m.adapters,
            ManifestAny::V2(m) => &m.adapters,
        }
    }

    pub fn metadata(&self) -> Option<&ManifestMetadata> {
        match self {
            ManifestAny::V1(m) => m.metadata.as_ref(),
            ManifestAny::V2(m) => m.metadata.as_ref(),
        }
    }

    pub fn dependencies(&self) -> Option<&Dependencies> {
        match self {
            ManifestAny::V1(m) => m.dependencies.as_ref(),
            ManifestAny::V2(m) => m.dependencies.as_ref(),
        }
    }

    /// Get as v0.1.0 manifest (returns None for v0.2.0).
    pub fn as_v1(&self) -> Option<&Manifest> {
        match self {
            ManifestAny::V1(m) => Some(m),
            _ => None,
        }
    }

    /// Get as v0.2.0 manifest (returns None for v0.1.0).
    pub fn as_v2(&self) -> Option<&ManifestV2> {
        match self {
            ManifestAny::V2(m) => Some(m),
            _ => None,
        }
    }
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

/// Parse a manifest of any schema version (v0.1.0 or v0.2.0).
/// Uses two-phase parsing: first peeks at schemaVersion, then deserializes
/// the correct struct for precise error messages.
pub fn parse_manifest_any(yaml: &str) -> Result<ManifestAny, ManifestError> {
    // Phase 1: peek at schemaVersion
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0");

    match schema_version {
        "0.1.0" => {
            // Check for v0.2.0 fields that shouldn't be in v0.1.0
            // (no action needed — serde will just ignore unknown fields or fail on missing required)
            let manifest: Manifest = serde_yaml::from_str(yaml)?;
            Ok(ManifestAny::V1(manifest))
        }
        "0.2.0" => {
            // Check for v0.1.0-only fields that are invalid in v0.2.0
            if value.get("content").is_some() {
                return Err(ManifestError::Validation(
                    "v0.2.0 manifests must not contain 'content' — use 'files' and skill entrypoints instead".to_string(),
                ));
            }
            if value.get("contract").is_some() {
                return Err(ManifestError::Validation(
                    "v0.2.0 manifests must not contain 'contract' — use 'skills' instead".to_string(),
                ));
            }
            let manifest: ManifestV2 = serde_yaml::from_str(yaml)?;
            Ok(ManifestAny::V2(manifest))
        }
        other => {
            Err(ManifestError::Validation(format!(
                "unsupported schemaVersion \"{}\": supported versions are \"0.1.0\" and \"0.2.0\"",
                other
            )))
        }
    }
}

/// Load a manifest of any schema version from a file.
pub fn load_manifest_any(path: &Path) -> Result<ManifestAny, ManifestError> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| ManifestError::NotFound(path.display().to_string()))?;
    parse_manifest_any(&content)
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

/// Validate a v0.2.0 manifest.
pub fn validate_manifest_v2(manifest: &ManifestV2, base_path: Option<&Path>) -> ValidationResult {
    let mut result = ValidationResult::new();

    // schemaVersion check
    if manifest.schema_version != "0.2.0" {
        result.add_error(format!(
            "schemaVersion must be \"0.2.0\", got \"{}\"",
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

    // files: warn if empty
    if manifest.files.is_empty() {
        result.add(ValidationMessage {
            severity: Severity::Warning,
            message: "files list is empty — the package bundles no files".to_string(),
        });
    }

    // skills: must have at least one
    if manifest.skills.is_empty() {
        result.add_error("skills map must contain at least one skill".to_string());
    }

    // Validate each skill definition
    for (skill_name, skill_def) in &manifest.skills {
        // Skill name must be kebab-case
        if !is_kebab_case(skill_name) {
            result.add_error(format!(
                "skill name must be kebab-case, got \"{}\"",
                skill_name
            ));
        }

        // Skill version must be semver
        if semver_parse(&skill_def.version).is_none() {
            result.add_error(format!(
                "skills.{}.version must be valid semver, got \"{}\"",
                skill_name, skill_def.version
            ));
        }

        // Validate permissions
        for perm in &skill_def.permissions {
            let check = crate::permissions::validate_permission(perm);
            if !check.valid_format {
                result.add_error(format!(
                    "skills.{}: permission \"{}\" has invalid format",
                    skill_name, perm
                ));
            } else if !check.known {
                result.add(ValidationMessage {
                    severity: Severity::Warning,
                    message: format!(
                        "skills.{}: permission \"{}\" is not in the v0 vocabulary",
                        skill_name, perm
                    ),
                });
            }
        }

        // Validate entrypoint exists on disk
        if let Some(base) = base_path {
            let ep_path = base.join(&skill_def.entrypoint);
            if !ep_path.exists() {
                result.add_error(format!(
                    "skills.{}.entrypoint file not found: {}",
                    skill_name, skill_def.entrypoint
                ));
            }

            // Validate command files exist
            if let Some(ref commands) = skill_def.commands {
                for (cmd_name, cmd_path) in commands {
                    let full = base.join(cmd_path);
                    if !full.exists() {
                        result.add_error(format!(
                            "skills.{}.commands.{} file not found: {}",
                            skill_name, cmd_name, cmd_path
                        ));
                    }
                }
            }
        }
    }

    // Validate tools
    let known_runtimes = ["node", "python", "shell"];
    if let Some(ref tools) = manifest.tools {
        for (tool_name, tool_def) in tools {
            // Tool name must be kebab-case
            if !is_kebab_case(tool_name) {
                result.add_error(format!(
                    "tool name must be kebab-case, got \"{}\"",
                    tool_name
                ));
            }

            // using must be a known runtime (warn on unknown)
            if !known_runtimes.contains(&tool_def.using.as_str()) {
                result.add(ValidationMessage {
                    severity: Severity::Warning,
                    message: format!(
                        "tools.{}: unknown runtime \"{}\", supported: node, python, shell",
                        tool_name, tool_def.using
                    ),
                });
            }

            // Validate entrypoint exists on disk
            if let Some(base) = base_path {
                let ep_path = base.join(&tool_def.entrypoint);
                if !ep_path.exists() {
                    result.add_error(format!(
                        "tools.{}.entrypoint file not found: {}",
                        tool_name, tool_def.entrypoint
                    ));
                }
            }

            // Validate input/output are valid JSON Schema (basic check: must be objects)
            if let Some(ref input) = tool_def.input {
                if !input.is_object() {
                    result.add_error(format!(
                        "tools.{}.input must be a JSON Schema object",
                        tool_name
                    ));
                }
            }
            if let Some(ref output) = tool_def.output {
                if !output.is_object() {
                    result.add_error(format!(
                        "tools.{}.output must be a JSON Schema object",
                        tool_name
                    ));
                }
            }
        }
    }

    // Validate hooks
    if let Some(ref hooks) = manifest.hooks {
        if let Some(base) = base_path {
            if let Some(ref path) = hooks.on_install {
                if !base.join(path).exists() {
                    result.add_error(format!("hooks.onInstall file not found: {}", path));
                }
            }
            if let Some(ref path) = hooks.on_activate {
                if !base.join(path).exists() {
                    result.add_error(format!("hooks.onActivate file not found: {}", path));
                }
            }
            if let Some(ref path) = hooks.on_uninstall {
                if !base.join(path).exists() {
                    result.add_error(format!("hooks.onUninstall file not found: {}", path));
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

/// Validate any manifest (dispatches to v0.1.0 or v0.2.0 validation).
pub fn validate_manifest_any(manifest: &ManifestAny, base_path: Option<&Path>) -> ValidationResult {
    match manifest {
        ManifestAny::V1(m) => validate_manifest(m, base_path),
        ManifestAny::V2(m) => validate_manifest_v2(m, base_path),
    }
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

    // --- v0.2.0 tests ---

    const VALID_V2_MANIFEST: &str = r#"
schemaVersion: "0.2.0"
name: "api-testing-suite"
description: "API contract testing with executable tools"
version: "2.0.0"
files:
  - "content/**"
  - "logic/**"
skills:
  contract-tester:
    description: "Generate and run contract tests"
    entrypoint: "content/contract-tester.md"
    version: "1.0.0"
    permissions:
      - "filesystem.read"
      - "filesystem.write"
      - "process.spawn"
      - "network.external"
    determinism: "bounded"
  spec-linter:
    description: "Validate an OpenAPI spec"
    entrypoint: "content/spec-linter.md"
    version: "1.0.0"
    permissions:
      - "filesystem.read"
    determinism: "deterministic"
tools:
  generate:
    description: "Generate test harness from an OpenAPI spec"
    using: "node"
    version: ">= 18"
    entrypoint: "logic/tools/generate.ts"
    input:
      type: "object"
      properties:
        spec:
          type: "string"
      required: ["spec"]
    output:
      type: "object"
      properties:
        status:
          type: "string"
        testCount:
          type: "integer"
  run-tests:
    description: "Execute generated tests"
    using: "node"
    entrypoint: "logic/tools/run-tests.ts"
    input:
      type: "object"
      properties:
        baseUrl:
          type: "string"
      required: ["baseUrl"]
    output:
      type: "object"
      properties:
        status:
          type: "string"
hooks:
  onInstall: "logic/hooks/setup.sh"
  onActivate: "logic/hooks/verify-runtime.sh"
adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true
metadata:
  author: "aule"
  license: "MIT"
"#;

    #[test]
    fn parse_v2_manifest() {
        let result = parse_manifest_any(VALID_V2_MANIFEST).unwrap();
        let m = result.as_v2().expect("should be V2");
        assert_eq!(m.name, "api-testing-suite");
        assert_eq!(m.schema_version, "0.2.0");
        assert_eq!(m.skills.len(), 2);
        assert_eq!(m.tools.as_ref().unwrap().len(), 2);
        assert!(m.hooks.is_some());
        assert_eq!(m.files.len(), 2);
    }

    #[test]
    fn parse_v1_via_any() {
        let result = parse_manifest_any(VALID_MANIFEST).unwrap();
        assert!(result.as_v1().is_some());
        assert_eq!(result.name(), "openspec-explore");
    }

    #[test]
    fn v2_rejects_content_field() {
        let yaml = r#"
schemaVersion: "0.2.0"
name: "test"
description: "test"
version: "1.0.0"
content:
  skill: "content/skill.md"
files:
  - "content/**"
skills:
  main:
    description: "main skill"
    entrypoint: "content/skill.md"
    version: "1.0.0"
"#;
        let result = parse_manifest_any(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("content"), "error should mention 'content': {}", err);
    }

    #[test]
    fn v2_rejects_contract_field() {
        let yaml = r#"
schemaVersion: "0.2.0"
name: "test"
description: "test"
version: "1.0.0"
contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
files:
  - "content/**"
skills:
  main:
    description: "main skill"
    entrypoint: "content/skill.md"
    version: "1.0.0"
"#;
        let result = parse_manifest_any(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("contract"), "error should mention 'contract': {}", err);
    }

    #[test]
    fn unknown_schema_version_rejected() {
        let yaml = VALID_MANIFEST.replace("\"0.1.0\"", "\"0.3.0\"");
        let result = parse_manifest_any(&yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unsupported schemaVersion"));
    }

    #[test]
    fn validate_v2_tool_name_not_kebab() {
        let yaml = VALID_V2_MANIFEST.replace("generate:", "generateTests:");
        let m: ManifestV2 = serde_yaml::from_str(&yaml).unwrap();
        let result = validate_manifest_v2(&m, None);
        assert!(!result.is_ok());
        assert!(result.errors().iter().any(|e| e.contains("kebab-case")));
    }

    #[test]
    fn validate_v2_unknown_runtime_warns() {
        let yaml = VALID_V2_MANIFEST.replace("using: \"node\"", "using: \"ruby\"");
        let m: ManifestV2 = serde_yaml::from_str(&yaml).unwrap();
        let result = validate_manifest_v2(&m, None);
        assert!(result.is_ok()); // warnings only
        assert!(result.warnings().iter().any(|w| w.contains("unknown runtime")));
    }

    #[test]
    fn validate_v2_empty_skills_error() {
        let yaml = r#"
schemaVersion: "0.2.0"
name: "test"
description: "test"
version: "1.0.0"
files:
  - "content/**"
skills: {}
"#;
        let m: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        let result = validate_manifest_v2(&m, None);
        assert!(!result.is_ok());
        assert!(result.errors().iter().any(|e| e.contains("at least one skill")));
    }

    #[test]
    fn validate_v2_empty_files_warns() {
        let yaml = r#"
schemaVersion: "0.2.0"
name: "test"
description: "test"
version: "1.0.0"
files: []
skills:
  main:
    description: "main"
    entrypoint: "content/main.md"
    version: "1.0.0"
"#;
        let m: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        let result = validate_manifest_v2(&m, None);
        assert!(result.is_ok()); // warning only
        assert!(result.warnings().iter().any(|w| w.contains("files list is empty")));
    }

    #[test]
    fn v1_manifests_still_parse_via_any() {
        // Ensure all existing v0.1.0 manifests parse without changes
        let result = parse_manifest_any(VALID_MANIFEST).unwrap();
        match result {
            ManifestAny::V1(m) => {
                assert_eq!(m.name, "openspec-explore");
                assert_eq!(m.schema_version, "0.1.0");
                let vr = validate_manifest(&m, None);
                assert!(vr.is_ok(), "v0.1.0 validation should still pass: {:?}", vr.errors());
            }
            _ => panic!("should parse as V1"),
        }
    }

    #[test]
    fn validate_v2_tool_input_not_object() {
        let yaml = r#"
schemaVersion: "0.2.0"
name: "test"
description: "test"
version: "1.0.0"
files:
  - "logic/**"
skills:
  main:
    description: "main"
    entrypoint: "content/main.md"
    version: "1.0.0"
tools:
  bad-tool:
    description: "bad"
    using: "node"
    entrypoint: "logic/bad.ts"
    input: "not an object"
"#;
        let m: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        let result = validate_manifest_v2(&m, None);
        assert!(!result.is_ok());
        assert!(result.errors().iter().any(|e| e.contains("JSON Schema object")));
    }

    #[test]
    fn manifest_any_accessors() {
        let v1 = parse_manifest_any(VALID_MANIFEST).unwrap();
        assert_eq!(v1.name(), "openspec-explore");
        assert_eq!(v1.schema_version(), "0.1.0");
        assert!(v1.as_v1().is_some());
        assert!(v1.as_v2().is_none());

        let v2 = parse_manifest_any(VALID_V2_MANIFEST).unwrap();
        assert_eq!(v2.name(), "api-testing-suite");
        assert_eq!(v2.schema_version(), "0.2.0");
        assert!(v2.as_v1().is_none());
        assert!(v2.as_v2().is_some());
    }
}
