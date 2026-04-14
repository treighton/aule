use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// --- Stage 1: Discovery Types ---

/// A skill discovered by scanning known locations in a repository.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub name: String,
    pub description: Option<String>,
    pub entrypoint: PathBuf,
    pub commands: HashMap<String, PathBuf>,
    pub source_format: SourceFormat,
}

/// The format/location where a skill was discovered.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceFormat {
    /// .claude/skills/
    ClaudeSkill,
    /// .codex/skills/
    CodexSkill,
    /// .claude/commands/ (command-only, no skill body)
    ClaudeCommand,
    /// plugin.json
    ClaudePlugin,
    /// Standalone SKILL.md in repo root or subdirectory
    StandaloneSkillMd,
}

/// Result of scanning a repository for existing skills.
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub skills: Vec<DiscoveredSkill>,
    pub warnings: Vec<String>,
}

// --- Stage 2: Signal Gathering Types ---

/// Metadata gathered from a repository for LLM assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredSignals {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,

    pub readme_content: Option<String>,
    pub file_tree: Vec<String>,
    pub language: Option<String>,
    pub runtime: Option<String>,
    pub runtime_version: Option<String>,
    pub executables: Vec<ExecutableInfo>,

    pub declared_inputs: Option<serde_json::Value>,
    pub declared_outputs: Option<serde_json::Value>,
    pub declared_permissions: Vec<String>,

    pub signal_source: SignalSource,
}

impl Default for InferredSignals {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            description: None,
            author: None,
            license: None,
            readme_content: None,
            file_tree: Vec::new(),
            language: None,
            runtime: None,
            runtime_version: None,
            executables: Vec::new(),
            declared_inputs: None,
            declared_outputs: None,
            declared_permissions: Vec::new(),
            signal_source: SignalSource::Generic,
        }
    }
}

/// An executable file found in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableInfo {
    pub name: String,
    pub path: PathBuf,
    pub kind: ExecutableKind,
}

/// The kind of executable discovered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutableKind {
    Binary,
    Script,
    EntryPoint,
}

/// Which language/ecosystem provided the signals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalSource {
    Npm,
    Python,
    Rust,
    Go,
    Generic,
}

// --- Stage 2: LLM Assessment Types ---

/// The LLM's assessment of whether skills can be inferred from a repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAssessment {
    pub can_infer: bool,
    pub confidence: f32,
    pub reasoning: String,
    pub suggested_skills: Vec<SuggestedSkill>,
    pub suggested_tools: Vec<SuggestedTool>,
}

/// A skill suggested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedSkill {
    pub name: String,
    pub description: String,
    pub entrypoint_suggestion: String,
    pub permissions: Vec<String>,
    pub determinism: String,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
}

/// A tool suggested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedTool {
    pub name: String,
    pub description: String,
    pub using: String,
    pub entrypoint: String,
    pub version: Option<String>,
}

// --- Errors ---

#[derive(Debug, Error)]
pub enum InferError {
    #[error("scan error: {0}")]
    Scan(String),

    #[error("gather error: {0}")]
    Gather(String),

    #[error("no ANTHROPIC_API_KEY set — set it to enable LLM-assisted inference")]
    NoApiKey,

    #[error("LLM service unavailable: {0}")]
    LlmUnavailable(String),

    #[error("LLM rate limited{}", .0.as_ref().map(|s| format!(", retry after: {}", s)).unwrap_or_default())]
    LlmRateLimit(Option<String>),

    #[error("failed to parse LLM response: {0}")]
    LlmResponseParse(String),

    #[error("manifest build error: {0}")]
    ManifestBuild(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("source already has a skill.yaml")]
    AlreadyHasManifest,
}
