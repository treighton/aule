//! Adapter definition types and parsing.
//!
//! An adapter defines how a skill package is transformed into runtime-specific
//! output. Two types are supported:
//!
//! - **Config-based**: Declarative path templates and frontmatter config,
//!   processed by the built-in generation pipeline.
//! - **Script-based**: External scripts that receive manifest+content as JSON
//!   on stdin and return generated files as JSON on stdout.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Maximum protocol version supported by this version of the CLI.
pub const MAX_SUPPORTED_PROTOCOL: u32 = 1;

#[derive(Debug, Error)]
pub enum AdapterDefError {
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("unknown adapter type: {0} (expected 'config' or 'script')")]
    UnknownType(String),
    #[error("invalid protocol version: {0}")]
    InvalidProtocol(String),
    #[error("adapter requires protocol v{required} but this version of Aulë only supports up to v{max}. Please upgrade.")]
    UnsupportedProtocol { required: u32, max: u32 },
    #[error("config adapter paths.skill must contain {{name}} placeholder")]
    MissingSkillPlaceholder,
    #[error("config adapter paths.commands.path must contain {{namespace}} and {{command_name}} placeholders")]
    MissingCommandPlaceholders,
    #[error("script adapter missing 'generate' field")]
    MissingGenerateScript,
    #[error("failed to parse adapter.yaml: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Where an adapter was discovered from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterSource {
    BuiltIn,
    UserInstalled,
    SkillBundled,
}

impl std::fmt::Display for AdapterSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterSource::BuiltIn => write!(f, "built-in"),
            AdapterSource::UserInstalled => write!(f, "user-installed"),
            AdapterSource::SkillBundled => write!(f, "skill-bundled"),
        }
    }
}

/// An adapter definition — either config-based or script-based.
#[derive(Debug, Clone)]
pub enum AdapterDef {
    Config(ConfigAdapter),
    Script(ScriptAdapter),
}

impl AdapterDef {
    pub fn id(&self) -> &str {
        match self {
            AdapterDef::Config(c) => &c.id,
            AdapterDef::Script(s) => &s.id,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            AdapterDef::Config(c) => &c.description,
            AdapterDef::Script(s) => &s.description,
        }
    }

    pub fn protocol(&self) -> u32 {
        match self {
            AdapterDef::Config(c) => c.protocol,
            AdapterDef::Script(s) => s.protocol,
        }
    }

    pub fn adapter_type_name(&self) -> &str {
        match self {
            AdapterDef::Config(_) => "config",
            AdapterDef::Script(_) => "script",
        }
    }

    /// Resolve the skill output path for a given skill name.
    pub fn skill_path(&self, name: &str) -> String {
        match self {
            AdapterDef::Config(c) => c.paths.skill.replace("{name}", name),
            AdapterDef::Script(_) => {
                // Script adapters return paths in their output; this shouldn't be called.
                panic!("skill_path() not applicable to script adapters")
            }
        }
    }

    /// Resolve the command output path for a given namespace and command name.
    /// Returns None if commands are not supported.
    pub fn command_path(&self, namespace: &str, command_name: &str) -> Option<String> {
        match self {
            AdapterDef::Config(c) => c.paths.commands.as_ref().map(|cmd| {
                cmd.path
                    .replace("{namespace}", namespace)
                    .replace("{command_name}", command_name)
            }),
            AdapterDef::Script(_) => None,
        }
    }

    /// Whether this adapter supports command generation.
    pub fn supports_commands(&self) -> bool {
        match self {
            AdapterDef::Config(c) => c.paths.commands.is_some(),
            AdapterDef::Script(_) => false, // script adapters handle commands themselves
        }
    }

    /// Get the validate script path, if any.
    pub fn validate_script(&self) -> Option<&str> {
        match self {
            AdapterDef::Config(c) => c.validate.as_deref(),
            AdapterDef::Script(s) => s.validate.as_deref(),
        }
    }

    /// Get the list of extra frontmatter fields this adapter consumes.
    pub fn extra_fields(&self) -> &[String] {
        match self {
            AdapterDef::Config(c) => &c.frontmatter.extra_fields,
            AdapterDef::Script(_) => &[],
        }
    }
}

/// A config-based adapter that uses the built-in generation pipeline.
#[derive(Debug, Clone)]
pub struct ConfigAdapter {
    pub id: String,
    pub description: String,
    pub author: Option<String>,
    pub protocol: u32,
    pub paths: AdapterPaths,
    pub frontmatter: AdapterFrontmatter,
    pub validate: Option<String>,
}

/// Path configuration for a config-based adapter.
#[derive(Debug, Clone)]
pub struct AdapterPaths {
    /// Template path for skill files. Must contain `{name}`.
    pub skill: String,
    /// Command configuration, if supported.
    pub commands: Option<CommandConfig>,
}

/// Command output configuration.
#[derive(Debug, Clone)]
pub struct CommandConfig {
    /// Template path for command files. Must contain `{namespace}` and `{command_name}`.
    pub path: String,
    /// Display name template. Supports `{skill}` and `{command}`.
    pub display_name: String,
    /// Category for command files.
    pub category: String,
    /// Tags template. Supports `{skill}` and `{command}`.
    pub tags: Vec<String>,
}

/// Frontmatter configuration for a config-based adapter.
#[derive(Debug, Clone)]
pub struct AdapterFrontmatter {
    /// Extra fields from AdapterConfig.extra to include in frontmatter.
    pub extra_fields: Vec<String>,
}

/// A script-based adapter that owns the entire generation pipeline.
#[derive(Debug, Clone)]
pub struct ScriptAdapter {
    pub id: String,
    pub description: String,
    pub author: Option<String>,
    pub protocol: u32,
    pub generate: String,
    pub validate: Option<String>,
    /// The directory where the adapter lives (scripts are relative to this).
    pub adapter_dir: Option<PathBuf>,
}

// --- Built-in adapter definitions ---

impl AdapterDef {
    pub fn claude_code() -> Self {
        AdapterDef::Config(ConfigAdapter {
            id: "claude-code".to_string(),
            description: "Adapter for Claude Code".to_string(),
            author: None,
            protocol: 1,
            paths: AdapterPaths {
                skill: ".claude/skills/{name}/SKILL.md".to_string(),
                commands: Some(CommandConfig {
                    path: ".claude/commands/{namespace}/{command_name}.md".to_string(),
                    display_name: "OPSX: {command}".to_string(),
                    category: "Workflow".to_string(),
                    tags: vec!["workflow".to_string(), "{command}".to_string(), "experimental".to_string()],
                }),
            },
            frontmatter: AdapterFrontmatter {
                extra_fields: vec![],
            },
            validate: None,
        })
    }

    pub fn codex() -> Self {
        AdapterDef::Config(ConfigAdapter {
            id: "codex".to_string(),
            description: "Adapter for Codex".to_string(),
            author: None,
            protocol: 1,
            paths: AdapterPaths {
                skill: ".codex/skills/{name}/SKILL.md".to_string(),
                commands: None,
            },
            frontmatter: AdapterFrontmatter {
                extra_fields: vec![],
            },
            validate: None,
        })
    }

    pub fn pi() -> Self {
        AdapterDef::Config(ConfigAdapter {
            id: "pi".to_string(),
            description: "Adapter for Pi".to_string(),
            author: None,
            protocol: 1,
            paths: AdapterPaths {
                skill: "~/.pi/agent/skills/{name}/SKILL.md".to_string(),
                commands: None,
            },
            frontmatter: AdapterFrontmatter {
                extra_fields: vec![
                    "allowed-tools".to_string(),
                    "disable-model-invocation".to_string(),
                ],
            },
            validate: None,
        })
    }

    /// Look up a built-in adapter by ID.
    pub fn built_in_by_id(id: &str) -> Option<Self> {
        match id {
            "claude-code" => Some(Self::claude_code()),
            "codex" => Some(Self::codex()),
            "pi" => Some(Self::pi()),
            _ => None,
        }
    }

    /// Return all built-in adapters.
    pub fn all_built_in() -> Vec<Self> {
        vec![Self::claude_code(), Self::codex(), Self::pi()]
    }
}

// --- adapter.yaml parsing ---

/// Raw deserialized form of adapter.yaml before validation.
#[derive(Debug, Deserialize)]
struct RawAdapterDef {
    id: String,
    #[serde(rename = "type")]
    adapter_type: String,
    #[serde(default = "default_protocol")]
    protocol: u32,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    paths: Option<RawAdapterPaths>,
    #[serde(default)]
    frontmatter: Option<RawAdapterFrontmatter>,
    #[serde(default)]
    generate: Option<String>,
    #[serde(default)]
    validate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAdapterPaths {
    skill: String,
    #[serde(default)]
    commands: Option<RawCommandConfig>,
}

#[derive(Debug, Deserialize)]
struct RawCommandConfig {
    path: String,
    #[serde(default = "default_display_name")]
    display_name: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawAdapterFrontmatter {
    #[serde(default)]
    extra_fields: Vec<String>,
}

fn default_protocol() -> u32 {
    1
}

fn default_display_name() -> String {
    "{skill}: {command}".to_string()
}

fn default_category() -> String {
    "Workflow".to_string()
}

/// Parse an adapter definition from YAML string.
pub fn parse_adapter_def(yaml: &str) -> Result<AdapterDef, AdapterDefError> {
    let raw: RawAdapterDef = serde_yaml::from_str(yaml)
        .map_err(|e| AdapterDefError::ParseError(e.to_string()))?;
    validate_and_build(raw, None)
}

/// Parse an adapter definition from an adapter.yaml file path.
pub fn parse_adapter_def_from_path(path: &Path) -> Result<AdapterDef, AdapterDefError> {
    let yaml = std::fs::read_to_string(path)?;
    let raw: RawAdapterDef = serde_yaml::from_str(&yaml)
        .map_err(|e| AdapterDefError::ParseError(e.to_string()))?;
    let adapter_dir = path.parent().map(|p| p.to_path_buf());
    validate_and_build(raw, adapter_dir)
}

fn validate_and_build(raw: RawAdapterDef, adapter_dir: Option<PathBuf>) -> Result<AdapterDef, AdapterDefError> {
    // Check protocol version
    if raw.protocol > MAX_SUPPORTED_PROTOCOL {
        return Err(AdapterDefError::UnsupportedProtocol {
            required: raw.protocol,
            max: MAX_SUPPORTED_PROTOCOL,
        });
    }

    match raw.adapter_type.as_str() {
        "config" => build_config_adapter(raw),
        "script" => build_script_adapter(raw, adapter_dir),
        other => Err(AdapterDefError::UnknownType(other.to_string())),
    }
}

fn build_config_adapter(raw: RawAdapterDef) -> Result<AdapterDef, AdapterDefError> {
    let paths_raw = raw.paths.ok_or_else(|| AdapterDefError::MissingField("paths".to_string()))?;

    // Validate skill path template
    if !paths_raw.skill.contains("{name}") {
        return Err(AdapterDefError::MissingSkillPlaceholder);
    }

    // Validate command path template if present
    if let Some(ref cmd) = paths_raw.commands {
        if !cmd.path.contains("{namespace}") || !cmd.path.contains("{command_name}") {
            return Err(AdapterDefError::MissingCommandPlaceholders);
        }
    }

    let commands = paths_raw.commands.map(|cmd| CommandConfig {
        path: cmd.path,
        display_name: cmd.display_name,
        category: cmd.category,
        tags: cmd.tags,
    });

    let frontmatter = raw.frontmatter.map(|fm| AdapterFrontmatter {
        extra_fields: fm.extra_fields,
    }).unwrap_or(AdapterFrontmatter { extra_fields: vec![] });

    Ok(AdapterDef::Config(ConfigAdapter {
        id: raw.id,
        description: raw.description.unwrap_or_default(),
        author: raw.author,
        protocol: raw.protocol,
        paths: AdapterPaths {
            skill: paths_raw.skill,
            commands,
        },
        frontmatter,
        validate: raw.validate,
    }))
}

fn build_script_adapter(raw: RawAdapterDef, adapter_dir: Option<PathBuf>) -> Result<AdapterDef, AdapterDefError> {
    let generate = raw.generate.ok_or(AdapterDefError::MissingGenerateScript)?;

    Ok(AdapterDef::Script(ScriptAdapter {
        id: raw.id,
        description: raw.description.unwrap_or_default(),
        author: raw.author,
        protocol: raw.protocol,
        generate,
        validate: raw.validate,
        adapter_dir,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_adapter() {
        let yaml = r#"
id: gemini
type: config
protocol: 1
description: "Adapter for Gemini CLI"
author: "community"
paths:
  skill: ".gemini/skills/{name}/SKILL.md"
  commands:
    path: ".gemini/commands/{namespace}/{command_name}.md"
    display_name: "{skill}: {command}"
    category: "Workflow"
    tags: ["workflow", "{skill}", "{command}"]
frontmatter:
  extra_fields:
    - model-preference
    - allowed-tools
"#;
        let def = parse_adapter_def(yaml).unwrap();
        assert_eq!(def.id(), "gemini");
        assert_eq!(def.adapter_type_name(), "config");
        assert!(def.supports_commands());
        assert_eq!(def.extra_fields().len(), 2);
        assert_eq!(def.skill_path("my-skill"), ".gemini/skills/my-skill/SKILL.md");
    }

    #[test]
    fn parse_script_adapter() {
        let yaml = r#"
id: cursor
type: script
protocol: 1
description: "Adapter for Cursor"
generate: ./generate.py
validate: ./validate.py
"#;
        let def = parse_adapter_def(yaml).unwrap();
        assert_eq!(def.id(), "cursor");
        assert_eq!(def.adapter_type_name(), "script");
        assert!(!def.supports_commands());
        assert!(def.validate_script().is_some());
    }

    #[test]
    fn parse_config_no_commands() {
        let yaml = r#"
id: simple
type: config
paths:
  skill: ".simple/skills/{name}/SKILL.md"
"#;
        let def = parse_adapter_def(yaml).unwrap();
        assert!(!def.supports_commands());
        assert_eq!(def.command_path("ns", "cmd"), None);
    }

    #[test]
    fn missing_required_field() {
        let yaml = r#"
type: config
paths:
  skill: ".x/{name}/SKILL.md"
"#;
        let err = parse_adapter_def(yaml);
        assert!(err.is_err());
    }

    #[test]
    fn unknown_type() {
        let yaml = r#"
id: test
type: wasm
"#;
        let err = parse_adapter_def(yaml).unwrap_err();
        assert!(matches!(err, AdapterDefError::UnknownType(_)));
    }

    #[test]
    fn missing_skill_placeholder() {
        let yaml = r#"
id: bad
type: config
paths:
  skill: ".bad/SKILL.md"
"#;
        let err = parse_adapter_def(yaml).unwrap_err();
        assert!(matches!(err, AdapterDefError::MissingSkillPlaceholder));
    }

    #[test]
    fn missing_command_placeholders() {
        let yaml = r#"
id: bad
type: config
paths:
  skill: ".x/{name}/SKILL.md"
  commands:
    path: ".x/commands/missing.md"
"#;
        let err = parse_adapter_def(yaml).unwrap_err();
        assert!(matches!(err, AdapterDefError::MissingCommandPlaceholders));
    }

    #[test]
    fn script_missing_generate() {
        let yaml = r#"
id: broken
type: script
"#;
        let err = parse_adapter_def(yaml).unwrap_err();
        assert!(matches!(err, AdapterDefError::MissingGenerateScript));
    }

    #[test]
    fn unsupported_protocol() {
        let yaml = r#"
id: future
type: config
protocol: 99
paths:
  skill: ".x/{name}/SKILL.md"
"#;
        let err = parse_adapter_def(yaml).unwrap_err();
        assert!(matches!(err, AdapterDefError::UnsupportedProtocol { .. }));
    }

    #[test]
    fn default_protocol_is_1() {
        let yaml = r#"
id: test
type: config
paths:
  skill: ".x/{name}/SKILL.md"
"#;
        let def = parse_adapter_def(yaml).unwrap();
        assert_eq!(def.protocol(), 1);
    }

    #[test]
    fn built_in_claude_code() {
        let def = AdapterDef::claude_code();
        assert_eq!(def.id(), "claude-code");
        assert!(def.supports_commands());
        assert_eq!(def.skill_path("foo"), ".claude/skills/foo/SKILL.md");
        assert_eq!(
            def.command_path("ns", "bar"),
            Some(".claude/commands/ns/bar.md".to_string())
        );
    }

    #[test]
    fn built_in_pi_has_extra_fields() {
        let def = AdapterDef::pi();
        assert_eq!(def.extra_fields(), &["allowed-tools", "disable-model-invocation"]);
    }

    #[test]
    fn built_in_codex() {
        let def = AdapterDef::codex();
        assert!(!def.supports_commands());
        assert_eq!(def.skill_path("x"), ".codex/skills/x/SKILL.md");
    }

    #[test]
    fn built_in_by_id_lookup() {
        assert!(AdapterDef::built_in_by_id("claude-code").is_some());
        assert!(AdapterDef::built_in_by_id("codex").is_some());
        assert!(AdapterDef::built_in_by_id("pi").is_some());
        assert!(AdapterDef::built_in_by_id("unknown").is_none());
    }
}
