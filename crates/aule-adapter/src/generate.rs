use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

use aule_schema::manifest::Manifest;
use crate::target::RuntimeTarget;

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub relative_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct GenerateOptions {
    /// Only generate for these targets. If empty, generate for all enabled adapters.
    pub targets: Vec<String>,
    /// Output root directory. If None, uses the base_path.
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("manifest validation failed: {0}")]
    ValidationFailed(String),
    #[error("content file not found: {0}")]
    ContentNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no enabled adapter targets found")]
    NoTargets,
}

/// Build YAML frontmatter string from manifest fields for a skill file.
fn build_skill_frontmatter(manifest: &Manifest) -> String {
    let mut lines = vec!["---".to_string()];

    lines.push(format!("name: {}", manifest.name));
    lines.push(format!("description: {}", manifest.description));

    if let Some(ref meta) = manifest.metadata {
        if let Some(ref license) = meta.license {
            lines.push(format!("license: {}", license));
        }
    }

    // compatibility from tool dependencies
    if let Some(ref deps) = manifest.dependencies {
        if !deps.tools.is_empty() {
            let tools: Vec<String> = deps.tools.iter().map(|t| {
                format!("{} CLI", t.name)
            }).collect();
            lines.push(format!("compatibility: Requires {}.", tools.join(", ")));
        }
    }

    // metadata block
    if let Some(ref meta) = manifest.metadata {
        let has_metadata = meta.author.is_some() || !meta.extra.is_empty();
        if has_metadata {
            lines.push("metadata:".to_string());
            if let Some(ref author) = meta.author {
                lines.push(format!("  author: {}", author));
            }
            // version from manifest version field
            lines.push(format!("  version: \"{}\"", manifest.version));
            // Pass through extra metadata fields
            for (key, value) in &meta.extra {
                match value {
                    serde_json::Value::String(s) => {
                        lines.push(format!("  {}: \"{}\"", key, s));
                    }
                    other => {
                        lines.push(format!("  {}: {}", key, other));
                    }
                }
            }
        }
    }

    lines.push("---".to_string());
    lines.join("\n")
}

/// Build YAML frontmatter for a Claude Code command file.
fn build_claude_command_frontmatter(manifest: &Manifest, command_name: &str) -> String {
    let display_name = format!("OPSX: {}", titlecase(command_name));
    let mut lines = vec!["---".to_string()];
    lines.push(format!("name: \"{}\"", display_name));
    lines.push(format!(
        "description: \"{}\"",
        manifest.description
    ));
    lines.push("category: Workflow".to_string());
    lines.push(format!(
        "tags: [workflow, {}, experimental]",
        command_name
    ));
    lines.push("---".to_string());
    lines.join("\n")
}

fn titlecase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Generate the SKILL.md file for a given target.
pub fn generate_skill_file(
    manifest: &Manifest,
    target: &RuntimeTarget,
    content: &str,
) -> GeneratedFile {
    let frontmatter = build_skill_frontmatter(manifest);
    let full_content = format!("{}\n\n{}", frontmatter, content);

    GeneratedFile {
        relative_path: target.skill_path(&manifest.name),
        content: full_content,
    }
}

/// Generate command files for a target (if supported).
pub fn generate_command_files(
    manifest: &Manifest,
    target: &RuntimeTarget,
    commands: &HashMap<String, String>,
) -> Vec<GeneratedFile> {
    if !target.supports_commands {
        return Vec::new();
    }

    let namespace = derive_namespace(&manifest.name);

    commands
        .iter()
        .filter_map(|(name, body)| {
            let path = target.command_path(&namespace, name)?;
            let frontmatter = build_claude_command_frontmatter(manifest, name);
            let full_content = format!("{}\n\n{}", frontmatter, body);
            Some(GeneratedFile {
                relative_path: path,
                content: full_content,
            })
        })
        .collect()
}

/// Derive a command namespace from the skill name.
/// e.g., "openspec-explore" -> "opsx" (based on existing convention)
/// For now, uses the skill name as-is; can be overridden in manifest.
fn derive_namespace(skill_name: &str) -> String {
    // Convention: use the skill name prefix before first hyphen, or full name
    skill_name
        .split('-')
        .next()
        .unwrap_or(skill_name)
        .to_string()
}

/// Write a `.generated` marker file.
fn write_generated_marker(dir: &Path, skill_name: &str) -> std::io::Result<()> {
    let marker = format!(
        "generated_by: aule-adapter\nskill: {}\ntimestamp: {}\n",
        skill_name,
        chrono_now()
    );
    std::fs::write(dir.join(".generated"), marker)
}

fn chrono_now() -> String {
    // Simple ISO timestamp without chrono dependency in this crate
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

/// Main generation entry point.
pub fn generate(
    manifest: &Manifest,
    base_path: &Path,
    options: &GenerateOptions,
) -> Result<Vec<GeneratedFile>, GenerateError> {
    // Validate content files exist
    let skill_content_path = base_path.join(&manifest.content.skill);
    if !skill_content_path.exists() {
        return Err(GenerateError::ContentNotFound(
            manifest.content.skill.clone(),
        ));
    }

    let skill_content = std::fs::read_to_string(&skill_content_path)?;

    // Load command content if present
    let mut command_contents: HashMap<String, String> = HashMap::new();
    if let Some(ref commands) = manifest.content.commands {
        for (name, path) in commands {
            let cmd_path = base_path.join(path);
            if !cmd_path.exists() {
                return Err(GenerateError::ContentNotFound(path.clone()));
            }
            command_contents.insert(name.clone(), std::fs::read_to_string(&cmd_path)?);
        }
    }

    // Determine which targets to generate for
    let targets = resolve_targets(manifest, options)?;
    if targets.is_empty() {
        return Err(GenerateError::NoTargets);
    }

    let output_root = options.output_dir.as_deref().unwrap_or(base_path);
    let mut generated = Vec::new();

    for target in &targets {
        // Generate skill file
        let skill_file = generate_skill_file(manifest, target, &skill_content);
        let skill_out = output_root.join(&skill_file.relative_path);
        if let Some(parent) = skill_out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&skill_out, &skill_file.content)?;
        generated.push(skill_file);

        // Write .generated marker in the skill directory
        if let Some(parent) = skill_out.parent() {
            write_generated_marker(parent, &manifest.name)?;
        }

        // Generate command files
        let cmd_files = generate_command_files(manifest, target, &command_contents);
        for cmd_file in cmd_files {
            let cmd_out = output_root.join(&cmd_file.relative_path);
            if let Some(parent) = cmd_out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&cmd_out, &cmd_file.content)?;
            generated.push(cmd_file);
        }
    }

    Ok(generated)
}

fn resolve_targets(
    manifest: &Manifest,
    options: &GenerateOptions,
) -> Result<Vec<RuntimeTarget>, GenerateError> {
    if !options.targets.is_empty() {
        // Use explicitly requested targets
        Ok(options
            .targets
            .iter()
            .filter_map(|id| RuntimeTarget::by_id(id))
            .collect())
    } else {
        // Use all enabled adapters from manifest
        Ok(manifest
            .adapters
            .iter()
            .filter(|(_, config)| config.enabled)
            .filter_map(|(id, _)| RuntimeTarget::by_id(id))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aule_schema::manifest::parse_manifest;
    use std::fs;
    use tempfile::TempDir;

    fn setup_skill_package(dir: &Path, yaml: &str, skill_content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("skill.yaml"), yaml).unwrap();
        fs::create_dir_all(dir.join("content")).unwrap();
        fs::write(dir.join("content/skill.md"), skill_content).unwrap();
    }

    const TEST_MANIFEST: &str = r#"
schemaVersion: "0.1.0"
name: "test-skill"
description: "A test skill"
version: "1.0.0"
content:
  skill: "content/skill.md"
contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions: []
adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true
metadata:
  author: "test"
  license: "MIT"
dependencies:
  tools:
    - name: "openspec"
"#;

    const TEST_SKILL_BODY: &str = "This is the skill body.\n\nIt has multiple lines.";

    #[test]
    fn generate_claude_code_skill() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = RuntimeTarget::claude_code();
        let file = generate_skill_file(&manifest, &target, TEST_SKILL_BODY);

        assert_eq!(file.relative_path, ".claude/skills/test-skill/SKILL.md");
        assert!(file.content.contains("name: test-skill"));
        assert!(file.content.contains("description: A test skill"));
        assert!(file.content.contains("license: MIT"));
        assert!(file.content.contains("compatibility: Requires openspec CLI."));
        assert!(file.content.ends_with(TEST_SKILL_BODY));
    }

    #[test]
    fn generate_codex_skill() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = RuntimeTarget::codex();
        let file = generate_skill_file(&manifest, &target, TEST_SKILL_BODY);

        assert_eq!(file.relative_path, ".codex/skills/test-skill/SKILL.md");
        assert!(file.content.ends_with(TEST_SKILL_BODY));
    }

    #[test]
    fn codex_skips_commands() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = RuntimeTarget::codex();
        let commands: HashMap<String, String> =
            HashMap::from([("explore".to_string(), "explore body".to_string())]);

        let files = generate_command_files(&manifest, &target, &commands);
        assert!(files.is_empty());
    }

    #[test]
    fn claude_code_generates_commands() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = RuntimeTarget::claude_code();
        let commands: HashMap<String, String> =
            HashMap::from([("explore".to_string(), "explore body".to_string())]);

        let files = generate_command_files(&manifest, &target, &commands);
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.contains(".claude/commands/"));
        assert!(files[0].relative_path.contains("explore.md"));
    }

    #[test]
    fn body_passthrough_is_identical() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = RuntimeTarget::claude_code();
        let body = "Exact content with special chars: 日本語 & <tags> \"quotes\"";
        let file = generate_skill_file(&manifest, &target, body);

        // Body should appear after the frontmatter exactly as provided
        let after_frontmatter = file.content.split("---\n\n").nth(1).unwrap();
        assert_eq!(after_frontmatter, body);
    }

    #[test]
    fn full_generate_writes_files() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        setup_skill_package(&src, TEST_MANIFEST, TEST_SKILL_BODY);

        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec![],
            output_dir: Some(output.clone()),
        };

        let files = generate(&manifest, &src, &options).unwrap();

        // Should generate for both claude-code and codex
        assert!(files.len() >= 2);

        // Verify files exist on disk
        assert!(output.join(".claude/skills/test-skill/SKILL.md").exists());
        assert!(output.join(".codex/skills/test-skill/SKILL.md").exists());

        // Verify .generated marker
        assert!(output.join(".claude/skills/test-skill/.generated").exists());
    }

    #[test]
    fn generate_single_target() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        setup_skill_package(&src, TEST_MANIFEST, TEST_SKILL_BODY);

        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec!["claude-code".to_string()],
            output_dir: Some(output.clone()),
        };

        let _files = generate(&manifest, &src, &options).unwrap();

        assert!(output.join(".claude/skills/test-skill/SKILL.md").exists());
        assert!(!output.join(".codex/skills/test-skill/SKILL.md").exists());
    }

    #[test]
    fn generate_fails_on_missing_content() {
        let tmp = TempDir::new().unwrap();
        // Don't create content/skill.md
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();

        let options = GenerateOptions::default();
        let result = generate(&manifest, tmp.path(), &options);
        assert!(result.is_err());
    }
}
