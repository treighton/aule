use std::collections::HashMap;
use std::path::Path;

use aule_schema::contract::Determinism;
use aule_schema::manifest::{
    AdapterConfig, ManifestMetadata, ManifestV2, SkillDefinition, Tool,
};

use crate::types::{
    DiscoveredSkill, InferError, InferredSignals, LlmAssessment, SourceFormat,
};

/// Build a ManifestV2 from discovered skills (Stage 1 — deterministic extraction).
pub fn build_from_discovered(
    skills: &[DiscoveredSkill],
    repo_root: &Path,
) -> Result<ManifestV2, InferError> {
    if skills.is_empty() {
        return Err(InferError::ManifestBuild(
            "no skills to build manifest from".to_string(),
        ));
    }

    let repo_name = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("skill")
        .to_string();

    let mut skill_map: HashMap<String, SkillDefinition> = HashMap::new();
    let mut files_set: Vec<String> = Vec::new();

    for skill in skills {
        let mut commands: Option<HashMap<String, String>> = None;
        if !skill.commands.is_empty() {
            let cmd_map: HashMap<String, String> = skill
                .commands
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string_lossy().to_string()))
                .collect();
            commands = Some(cmd_map);
        }

        let def = SkillDefinition {
            description: skill
                .description
                .clone()
                .unwrap_or_else(|| "TODO: add description".to_string()),
            entrypoint: skill.entrypoint.to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            inputs: None,
            outputs: None,
            permissions: Vec::new(),
            determinism: Determinism::Probabilistic,
            errors: None,
            behavior: None,
            commands,
        };

        skill_map.insert(skill.name.clone(), def);

        // Derive files globs from source format
        match &skill.source_format {
            SourceFormat::ClaudeSkill => {
                add_unique(&mut files_set, ".claude/skills/**".to_string());
            }
            SourceFormat::CodexSkill => {
                add_unique(&mut files_set, ".codex/skills/**".to_string());
            }
            SourceFormat::ClaudeCommand => {
                add_unique(&mut files_set, ".claude/commands/**".to_string());
            }
            SourceFormat::ClaudePlugin => {
                add_unique(&mut files_set, "plugin.json".to_string());
                // Also include the entrypoint directory
                if let Some(parent) = skill.entrypoint.parent() {
                    let glob = format!("{}/**", parent.display());
                    if glob != "/**" {
                        add_unique(&mut files_set, glob);
                    }
                }
            }
            SourceFormat::StandaloneSkillMd => {
                add_unique(
                    &mut files_set,
                    skill.entrypoint.to_string_lossy().to_string(),
                );
            }
        }

        // Add command files
        for cmd_path in skill.commands.values() {
            if let Some(parent) = cmd_path.parent() {
                let glob = format!("{}/**", parent.display());
                if glob != "/**" {
                    add_unique(&mut files_set, glob);
                }
            }
        }
    }

    if files_set.is_empty() {
        files_set.push("content/**".to_string());
    }

    // Use first skill's description as package description
    let description = skills
        .first()
        .and_then(|s| s.description.clone())
        .unwrap_or_else(|| format!("Skills inferred from {}", repo_name));

    let mut adapters = HashMap::new();
    adapters.insert(
        "claude-code".to_string(),
        AdapterConfig {
            enabled: true,
            extra: HashMap::new(),
        },
    );

    let manifest = ManifestV2 {
        schema_version: "0.2.0".to_string(),
        name: repo_name,
        description,
        version: "1.0.0".to_string(),
        files: files_set,
        skills: skill_map,
        tools: None,
        hooks: None,
        identity: None,
        adapters,
        dependencies: None,
        metadata: None,
        extensions: None,
    };

    validate_manifest(&manifest, repo_root)?;

    Ok(manifest)
}

/// Build a ManifestV2 from LLM assessment (Stage 2 — LLM suggestions).
pub fn build_from_assessment(
    assessment: &LlmAssessment,
    signals: &InferredSignals,
    repo_root: &Path,
) -> Result<ManifestV2, InferError> {
    if !assessment.can_infer || assessment.suggested_skills.is_empty() {
        return Err(InferError::ManifestBuild(
            "LLM assessment indicates no skills can be inferred".to_string(),
        ));
    }

    let repo_name = signals
        .name
        .clone()
        .or_else(|| {
            repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "skill".to_string());

    let mut skill_map: HashMap<String, SkillDefinition> = HashMap::new();
    let mut files_set: Vec<String> = Vec::new();

    for suggested in &assessment.suggested_skills {
        let determinism = match suggested.determinism.as_str() {
            "deterministic" => Determinism::Deterministic,
            "bounded" => Determinism::Bounded,
            _ => Determinism::Probabilistic,
        };

        let inputs = suggested.inputs.as_ref().and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v.clone()).ok()
            }
        });

        let outputs = suggested.outputs.as_ref().and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v.clone()).ok()
            }
        });

        let def = SkillDefinition {
            description: suggested.description.clone(),
            entrypoint: suggested.entrypoint_suggestion.clone(),
            version: signals
                .version
                .clone()
                .unwrap_or_else(|| "1.0.0".to_string()),
            inputs,
            outputs,
            permissions: suggested.permissions.clone(),
            determinism,
            errors: None,
            behavior: None,
            commands: None,
        };

        skill_map.insert(suggested.name.clone(), def);
        add_unique(
            &mut files_set,
            suggested.entrypoint_suggestion.clone(),
        );
    }

    // Build tools map
    let tools = if assessment.suggested_tools.is_empty() {
        None
    } else {
        let mut tool_map: HashMap<String, Tool> = HashMap::new();
        for suggested in &assessment.suggested_tools {
            tool_map.insert(
                suggested.name.clone(),
                Tool {
                    description: suggested.description.clone(),
                    using: suggested.using.clone(),
                    version: suggested.version.clone(),
                    entrypoint: suggested.entrypoint.clone(),
                    input: None,
                    output: None,
                },
            );

            // Add tool entrypoint directory to files
            if let Some(parent) = std::path::Path::new(&suggested.entrypoint).parent() {
                let glob = format!("{}/**", parent.display());
                if glob != "/**" {
                    add_unique(&mut files_set, glob);
                }
            }
        }
        Some(tool_map)
    };

    if files_set.is_empty() {
        files_set.push("content/**".to_string());
    }

    let description = assessment
        .suggested_skills
        .first()
        .map(|s| s.description.clone())
        .unwrap_or_else(|| format!("Skills inferred from {}", repo_name));

    let mut adapters = HashMap::new();
    adapters.insert(
        "claude-code".to_string(),
        AdapterConfig {
            enabled: true,
            extra: HashMap::new(),
        },
    );

    // Populate metadata from signals
    let metadata = if signals.author.is_some() || signals.license.is_some() {
        Some(ManifestMetadata {
            author: signals.author.clone(),
            license: signals.license.clone(),
            homepage: None,
            repository: None,
            tags: None,
            extra: HashMap::new(),
        })
    } else {
        None
    };

    let manifest = ManifestV2 {
        schema_version: "0.2.0".to_string(),
        name: repo_name,
        description,
        version: signals
            .version
            .clone()
            .unwrap_or_else(|| "1.0.0".to_string()),
        files: files_set,
        skills: skill_map,
        tools,
        hooks: None,
        identity: None,
        adapters,
        dependencies: None,
        metadata,
        extensions: None,
    };

    validate_manifest(&manifest, repo_root)?;

    Ok(manifest)
}

/// Serialize a ManifestV2 to YAML.
pub fn serialize_manifest(manifest: &ManifestV2) -> Result<String, InferError> {
    serde_yaml::to_string(manifest).map_err(|e| InferError::ManifestBuild(e.to_string()))
}

/// Validate a built manifest by parsing it back through aule-schema.
fn validate_manifest(manifest: &ManifestV2, repo_root: &Path) -> Result<(), InferError> {
    // Round-trip: serialize then parse to verify structural validity
    let yaml = serde_yaml::to_string(manifest)
        .map_err(|e| InferError::ManifestBuild(format!("serialization failed: {}", e)))?;

    aule_schema::manifest::parse_manifest_any(&yaml).map_err(|e| {
        InferError::ManifestBuild(format!("produced manifest failed validation: {}", e))
    })?;

    // Validate that entrypoint files exist (only for local paths)
    for (name, skill) in &manifest.skills {
        let ep = repo_root.join(&skill.entrypoint);
        if !ep.exists() {
            // Not an error — the file might not exist yet (e.g., LLM-suggested)
            // Just warn via eprintln, don't fail the build
            eprintln!(
                "warning: skill '{}' entrypoint '{}' does not exist at {}",
                name,
                skill.entrypoint,
                ep.display()
            );
        }
    }

    Ok(())
}

fn add_unique(vec: &mut Vec<String>, item: String) {
    if !vec.contains(&item) {
        vec.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_build_from_discovered_single_skill() {
        let dir = setup();
        let skill_dir = dir.path().join(".claude/skills/foo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Foo skill").unwrap();

        let skills = vec![DiscoveredSkill {
            name: "foo".to_string(),
            description: Some("A foo skill".to_string()),
            entrypoint: PathBuf::from(".claude/skills/foo/SKILL.md"),
            commands: HashMap::new(),
            source_format: SourceFormat::ClaudeSkill,
        }];

        let manifest = build_from_discovered(&skills, dir.path()).unwrap();
        assert_eq!(manifest.schema_version, "0.2.0");
        assert_eq!(manifest.skills.len(), 1);
        assert!(manifest.skills.contains_key("foo"));
        assert!(manifest.files.contains(&".claude/skills/**".to_string()));
    }

    #[test]
    fn test_build_from_discovered_with_commands() {
        let dir = setup();
        let skill_dir = dir.path().join(".claude/skills/bar");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Bar").unwrap();
        let cmds_dir = dir.path().join(".claude/commands");
        fs::create_dir_all(&cmds_dir).unwrap();
        fs::write(cmds_dir.join("deploy.md"), "# Deploy").unwrap();

        let mut commands = HashMap::new();
        commands.insert(
            "deploy".to_string(),
            PathBuf::from(".claude/commands/deploy.md"),
        );

        let skills = vec![DiscoveredSkill {
            name: "bar".to_string(),
            description: Some("Bar skill".to_string()),
            entrypoint: PathBuf::from(".claude/skills/bar/SKILL.md"),
            commands,
            source_format: SourceFormat::ClaudeSkill,
        }];

        let manifest = build_from_discovered(&skills, dir.path()).unwrap();
        let bar = manifest.skills.get("bar").unwrap();
        assert!(bar.commands.is_some());
        assert_eq!(bar.commands.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_build_from_discovered_empty() {
        let dir = setup();
        let result = build_from_discovered(&[], dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_build_from_assessment() {
        let dir = setup();
        fs::write(dir.path().join("README.md"), "# Tool").unwrap();

        let assessment = LlmAssessment {
            can_infer: true,
            confidence: 0.85,
            reasoning: "Well-documented tool".to_string(),
            suggested_skills: vec![SuggestedSkill {
                name: "my-tool".to_string(),
                description: "A great tool".to_string(),
                entrypoint_suggestion: "README.md".to_string(),
                permissions: vec!["filesystem.read".to_string()],
                determinism: "deterministic".to_string(),
                inputs: None,
                outputs: None,
            }],
            suggested_tools: vec![SuggestedTool {
                name: "run".to_string(),
                description: "Run the tool".to_string(),
                using: "node".to_string(),
                entrypoint: "bin/run.js".to_string(),
                version: Some(">=18".to_string()),
            }],
        };

        let signals = InferredSignals {
            name: Some("my-tool".to_string()),
            version: Some("2.0.0".to_string()),
            author: Some("Jane".to_string()),
            license: Some("MIT".to_string()),
            ..InferredSignals::default()
        };

        let manifest = build_from_assessment(&assessment, &signals, dir.path()).unwrap();
        assert_eq!(manifest.name, "my-tool");
        assert_eq!(manifest.version, "2.0.0");
        assert!(manifest.skills.contains_key("my-tool"));
        assert!(manifest.tools.is_some());
        assert!(manifest.metadata.is_some());

        let skill = manifest.skills.get("my-tool").unwrap();
        assert_eq!(skill.permissions, vec!["filesystem.read"]);
        assert_eq!(skill.determinism, Determinism::Deterministic);
    }

    #[test]
    fn test_build_from_assessment_cannot_infer() {
        let dir = setup();
        let assessment = LlmAssessment {
            can_infer: false,
            confidence: 0.1,
            reasoning: "Data only".to_string(),
            suggested_skills: vec![],
            suggested_tools: vec![],
        };
        let signals = InferredSignals::default();

        let result = build_from_assessment(&assessment, &signals, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_manifest() {
        let dir = setup();
        let skill_dir = dir.path().join(".claude/skills/test");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test").unwrap();

        let skills = vec![DiscoveredSkill {
            name: "test".to_string(),
            description: Some("Test skill".to_string()),
            entrypoint: PathBuf::from(".claude/skills/test/SKILL.md"),
            commands: HashMap::new(),
            source_format: SourceFormat::ClaudeSkill,
        }];

        let manifest = build_from_discovered(&skills, dir.path()).unwrap();
        let yaml = serialize_manifest(&manifest).unwrap();

        assert!(yaml.contains("schemaVersion"));
        assert!(yaml.contains("0.2.0"));
        assert!(yaml.contains("test"));

        // Round-trip
        let parsed = aule_schema::manifest::parse_manifest_any(&yaml).unwrap();
        assert_eq!(parsed.as_v2().unwrap().name, manifest.name);
    }

    #[test]
    fn test_round_trip_build_serialize_parse() {
        let dir = setup();
        fs::write(dir.path().join("SKILL.md"), "---\nname: rt\n---\n# RT").unwrap();

        let skills = vec![DiscoveredSkill {
            name: "rt".to_string(),
            description: Some("Round trip".to_string()),
            entrypoint: PathBuf::from("SKILL.md"),
            commands: HashMap::new(),
            source_format: SourceFormat::StandaloneSkillMd,
        }];

        let manifest = build_from_discovered(&skills, dir.path()).unwrap();
        let yaml = serialize_manifest(&manifest).unwrap();
        let parsed = aule_schema::manifest::parse_manifest_any(&yaml).unwrap();
        let v2 = parsed.as_v2().unwrap();

        assert_eq!(v2.name, manifest.name);
        assert_eq!(v2.skills.len(), manifest.skills.len());
        assert_eq!(v2.schema_version, "0.2.0");
    }
}
