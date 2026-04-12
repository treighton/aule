use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScaffoldError {
    #[error("skill package already exists at {0}")]
    AlreadyExists(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

const MANIFEST_TEMPLATE: &str = r#"schemaVersion: "0.1.0"
name: "{name}"
description: "TODO: describe your skill"
version: "0.1.0"

content:
  skill: "content/skill.md"
  # commands:
  #   my-command: "content/commands/my-command.md"

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

# metadata:
#   author: "your-name"
#   license: "MIT"
#   tags: []

# dependencies:
#   tools: []
#   skills: []
"#;

const SKILL_TEMPLATE: &str = r#"Your skill content goes here.

Describe what this skill does, how it should behave, and any instructions
for the AI agent that will execute it.
"#;

pub fn scaffold(dir: &Path, name: &str) -> Result<Vec<String>, ScaffoldError> {
    let manifest_path = dir.join("skill.yaml");
    if manifest_path.exists() {
        return Err(ScaffoldError::AlreadyExists(dir.display().to_string()));
    }

    let mut created = Vec::new();

    std::fs::create_dir_all(dir)?;
    std::fs::create_dir_all(dir.join("content/commands"))?;

    let manifest_content = MANIFEST_TEMPLATE.replace("{name}", name);
    std::fs::write(&manifest_path, manifest_content)?;
    created.push("skill.yaml".to_string());

    let skill_path = dir.join("content/skill.md");
    std::fs::write(&skill_path, SKILL_TEMPLATE)?;
    created.push("content/skill.md".to_string());

    created.push("content/commands/".to_string());

    Ok(created)
}
