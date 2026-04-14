use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::types::{DiscoveredSkill, InferError, ScanResult, SourceFormat};

/// Scan a repository for existing skills in known locations.
/// Runs all scanners and merges/deduplicates results.
pub fn scan_all(repo_root: &Path) -> Result<ScanResult, InferError> {
    let mut all_skills: Vec<DiscoveredSkill> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let scanners: Vec<(&str, fn(&Path) -> Result<Vec<DiscoveredSkill>, InferError>)> = vec![
        ("ClaudeSkill", scan_claude_skills),
        ("CodexSkill", scan_codex_skills),
        ("ClaudeCommand", scan_claude_commands),
        ("Plugin", scan_plugin_json),
        ("SkillMd", scan_skill_md),
    ];

    for (label, scanner) in scanners {
        match scanner(repo_root) {
            Ok(skills) => all_skills.extend(skills),
            Err(e) => warnings.push(format!("{} scanner: {}", label, e)),
        }
    }

    // Deduplicate by name, preferring richer sources
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut deduped: Vec<DiscoveredSkill> = Vec::new();

    for skill in all_skills {
        if let Some(&idx) = seen.get(&skill.name) {
            if source_priority(&skill.source_format) > source_priority(&deduped[idx].source_format)
            {
                deduped[idx] = skill;
            }
        } else {
            seen.insert(skill.name.clone(), deduped.len());
            deduped.push(skill);
        }
    }

    Ok(ScanResult {
        skills: deduped,
        warnings,
    })
}

fn source_priority(format: &SourceFormat) -> u8 {
    match format {
        SourceFormat::ClaudePlugin => 4,
        SourceFormat::ClaudeSkill => 3,
        SourceFormat::CodexSkill => 3,
        SourceFormat::StandaloneSkillMd => 2,
        SourceFormat::ClaudeCommand => 1,
    }
}

/// Parse YAML frontmatter from a markdown file.
/// Returns (name, description) if frontmatter exists and contains those fields.
fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    let content = content.trim();
    if !content.starts_with("---") {
        return (None, None);
    }

    let rest = &content[3..];
    let end = rest.find("\n---");
    let frontmatter = match end {
        Some(idx) => &rest[..idx],
        None => return (None, None),
    };

    let value: Result<serde_yaml::Value, _> = serde_yaml::from_str(frontmatter);
    match value {
        Ok(v) => {
            let name = v.get("name").and_then(|n| n.as_str()).map(String::from);
            let desc = v
                .get("description")
                .and_then(|d| d.as_str())
                .map(String::from);
            (name, desc)
        }
        Err(_) => (None, None),
    }
}

/// Derive a skill name from a file path.
fn name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Scan `.claude/skills/**/*.md` for skills.
fn scan_claude_skills(repo_root: &Path) -> Result<Vec<DiscoveredSkill>, InferError> {
    scan_skill_dir(
        repo_root,
        ".claude/skills",
        SourceFormat::ClaudeSkill,
    )
}

/// Scan `.codex/skills/**/*.md` for skills.
fn scan_codex_skills(repo_root: &Path) -> Result<Vec<DiscoveredSkill>, InferError> {
    scan_skill_dir(
        repo_root,
        ".codex/skills",
        SourceFormat::CodexSkill,
    )
}

/// Generic scanner for a skill directory with markdown files.
fn scan_skill_dir(
    repo_root: &Path,
    rel_dir: &str,
    format: SourceFormat,
) -> Result<Vec<DiscoveredSkill>, InferError> {
    let dir = repo_root.join(rel_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let pattern = format!("{}/**/*.md", dir.display());
    let paths = glob::glob(&pattern).map_err(|e| InferError::Scan(e.to_string()))?;

    let mut skills = Vec::new();
    for entry in paths.filter_map(|e| e.ok()) {
        if !entry.is_file() {
            continue;
        }

        let content = match std::fs::read_to_string(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = entry
            .strip_prefix(repo_root)
            .unwrap_or(&entry)
            .to_path_buf();

        let (fm_name, fm_desc) = parse_frontmatter(&content);
        let name = fm_name.unwrap_or_else(|| {
            // Try parent directory name, then file stem
            entry
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .filter(|n| *n != "skills")
                .map(String::from)
                .unwrap_or_else(|| name_from_path(&entry))
        });

        skills.push(DiscoveredSkill {
            name,
            description: fm_desc,
            entrypoint: rel_path,
            commands: HashMap::new(),
            source_format: format.clone(),
        });
    }

    Ok(skills)
}

/// Scan `.claude/commands/**/*.md` for commands.
fn scan_claude_commands(repo_root: &Path) -> Result<Vec<DiscoveredSkill>, InferError> {
    let dir = repo_root.join(".claude/commands");
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let pattern = format!("{}/**/*.md", dir.display());
    let paths = glob::glob(&pattern).map_err(|e| InferError::Scan(e.to_string()))?;

    // Group commands — commands don't map 1:1 to skills, they belong to
    // a synthetic "commands" skill
    let mut commands: HashMap<String, PathBuf> = HashMap::new();

    for entry in paths.filter_map(|e| e.ok()) {
        if !entry.is_file() {
            continue;
        }

        let rel_path = entry
            .strip_prefix(repo_root)
            .unwrap_or(&entry)
            .to_path_buf();
        let cmd_name = name_from_path(&entry);
        commands.insert(cmd_name, rel_path);
    }

    if commands.is_empty() {
        return Ok(Vec::new());
    }

    // Create a single synthetic skill holding all commands
    let first_cmd_path = commands.values().next().cloned().unwrap();
    Ok(vec![DiscoveredSkill {
        name: repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("commands")
            .to_string(),
        description: Some("Commands discovered from .claude/commands/".to_string()),
        entrypoint: first_cmd_path,
        commands,
        source_format: SourceFormat::ClaudeCommand,
    }])
}

/// Scan `plugin.json` for skills, commands, and agents.
fn scan_plugin_json(repo_root: &Path) -> Result<Vec<DiscoveredSkill>, InferError> {
    let plugin_path = repo_root.join("plugin.json");
    if !plugin_path.exists() {
        return Ok(Vec::new());
    }

    let content =
        std::fs::read_to_string(&plugin_path).map_err(|e| InferError::Scan(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| InferError::Scan(e.to_string()))?;

    let mut skills = Vec::new();

    // Extract skills from plugin.json
    if let Some(plugin_skills) = value.get("skills").and_then(|s| s.as_array()) {
        for s in plugin_skills {
            let name = s
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();
            let desc = s
                .get("description")
                .and_then(|d| d.as_str())
                .map(String::from);
            let entrypoint = s
                .get("entrypoint")
                .or_else(|| s.get("path"))
                .and_then(|p| p.as_str())
                .unwrap_or("SKILL.md")
                .to_string();

            skills.push(DiscoveredSkill {
                name,
                description: desc,
                entrypoint: PathBuf::from(entrypoint),
                commands: HashMap::new(),
                source_format: SourceFormat::ClaudePlugin,
            });
        }
    }

    // Extract commands
    let mut commands: HashMap<String, PathBuf> = HashMap::new();
    if let Some(cmds) = value.get("commands").and_then(|c| c.as_array()) {
        for cmd in cmds {
            if let (Some(name), Some(path)) = (
                cmd.get("name").and_then(|n| n.as_str()),
                cmd.get("path")
                    .or_else(|| cmd.get("entrypoint"))
                    .and_then(|p| p.as_str()),
            ) {
                commands.insert(name.to_string(), PathBuf::from(path));
            }
        }
    }

    // If we found commands but no skills, create a synthetic skill
    if skills.is_empty() && !commands.is_empty() {
        let plugin_name = value
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("plugin")
            .to_string();
        let desc = value
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);
        let first_cmd = commands.values().next().cloned().unwrap();

        skills.push(DiscoveredSkill {
            name: plugin_name,
            description: desc,
            entrypoint: first_cmd,
            commands,
            source_format: SourceFormat::ClaudePlugin,
        });
    } else if !commands.is_empty() {
        // Attach commands to the first skill
        if let Some(first) = skills.first_mut() {
            first.commands = commands;
        }
    }

    Ok(skills)
}

/// Scan for standalone `SKILL.md` files.
fn scan_skill_md(repo_root: &Path) -> Result<Vec<DiscoveredSkill>, InferError> {
    let mut skills = Vec::new();

    // Check root SKILL.md
    let root_skill = repo_root.join("SKILL.md");
    if root_skill.exists() && root_skill.is_file() {
        let content = std::fs::read_to_string(&root_skill).unwrap_or_default();
        let (fm_name, fm_desc) = parse_frontmatter(&content);
        let name = fm_name.unwrap_or_else(|| {
            repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("skill")
                .to_string()
        });

        skills.push(DiscoveredSkill {
            name,
            description: fm_desc,
            entrypoint: PathBuf::from("SKILL.md"),
            commands: HashMap::new(),
            source_format: SourceFormat::StandaloneSkillMd,
        });
    }

    // Also check immediate subdirectories (1 level deep)
    if let Ok(entries) = std::fs::read_dir(repo_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() && skill_md.is_file() {
                    let content = std::fs::read_to_string(&skill_md).unwrap_or_default();
                    let (fm_name, fm_desc) = parse_frontmatter(&content);
                    let rel_path = skill_md
                        .strip_prefix(repo_root)
                        .unwrap_or(&skill_md)
                        .to_path_buf();
                    let name = fm_name.unwrap_or_else(|| name_from_path(&path));

                    skills.push(DiscoveredSkill {
                        name,
                        description: fm_desc,
                        entrypoint: rel_path,
                        commands: HashMap::new(),
                        source_format: SourceFormat::StandaloneSkillMd,
                    });
                }
            }
        }
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_parse_frontmatter_with_name_and_desc() {
        let content = "---\nname: foo\ndescription: A foo skill\n---\n# Content";
        let (name, desc) = parse_frontmatter(content);
        assert_eq!(name, Some("foo".to_string()));
        assert_eq!(desc, Some("A foo skill".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let (name, desc) = parse_frontmatter("# Just a heading");
        assert!(name.is_none());
        assert!(desc.is_none());
    }

    #[test]
    fn test_parse_frontmatter_malformed() {
        let content = "---\ninvalid: [yaml: {{\n---\n";
        let (name, desc) = parse_frontmatter(content);
        assert!(name.is_none());
        assert!(desc.is_none());
    }

    #[test]
    fn test_scan_claude_skills() {
        let dir = setup_dir();
        let skills_dir = dir.path().join(".claude/skills/foo");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: foo\ndescription: Foo skill\n---\n# Foo",
        )
        .unwrap();

        let result = scan_claude_skills(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "foo");
        assert_eq!(result[0].description, Some("Foo skill".to_string()));
        assert_eq!(result[0].source_format, SourceFormat::ClaudeSkill);
    }

    #[test]
    fn test_scan_codex_skills() {
        let dir = setup_dir();
        let skills_dir = dir.path().join(".codex/skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("bar.md"),
            "---\nname: bar\n---\n# Bar",
        )
        .unwrap();

        let result = scan_codex_skills(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "bar");
    }

    #[test]
    fn test_scan_claude_commands() {
        let dir = setup_dir();
        let cmds_dir = dir.path().join(".claude/commands");
        fs::create_dir_all(&cmds_dir).unwrap();
        fs::write(cmds_dir.join("deploy.md"), "# Deploy").unwrap();
        fs::write(cmds_dir.join("test.md"), "# Test").unwrap();

        let result = scan_claude_commands(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].commands.len(), 2);
        assert!(result[0].commands.contains_key("deploy"));
        assert!(result[0].commands.contains_key("test"));
    }

    #[test]
    fn test_scan_plugin_json() {
        let dir = setup_dir();
        let plugin = serde_json::json!({
            "name": "my-plugin",
            "description": "A test plugin",
            "skills": [
                {
                    "name": "skill-one",
                    "description": "First skill",
                    "entrypoint": "skills/one.md"
                },
                {
                    "name": "skill-two",
                    "description": "Second skill",
                    "path": "skills/two.md"
                }
            ],
            "commands": [
                { "name": "cmd-a", "path": "commands/a.md" }
            ]
        });
        fs::write(
            dir.path().join("plugin.json"),
            serde_json::to_string_pretty(&plugin).unwrap(),
        )
        .unwrap();

        let result = scan_plugin_json(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "skill-one");
        assert_eq!(result[0].commands.len(), 1);
        assert_eq!(result[1].name, "skill-two");
    }

    #[test]
    fn test_scan_skill_md_root() {
        let dir = setup_dir();
        fs::write(
            dir.path().join("SKILL.md"),
            "---\nname: root-skill\ndescription: Root\n---\n# Root",
        )
        .unwrap();

        let result = scan_skill_md(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "root-skill");
        assert_eq!(result[0].source_format, SourceFormat::StandaloneSkillMd);
    }

    #[test]
    fn test_scan_skill_md_subdirectory() {
        let dir = setup_dir();
        let sub = dir.path().join("my-tool");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("SKILL.md"), "# Tool skill").unwrap();

        let result = scan_skill_md(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my-tool");
    }

    #[test]
    fn test_scan_all_empty() {
        let dir = setup_dir();
        let result = scan_all(dir.path()).unwrap();
        assert!(result.skills.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_scan_all_mixed() {
        let dir = setup_dir();

        // Claude skill
        let skills_dir = dir.path().join(".claude/skills/alpha");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: alpha\ndescription: Alpha\n---\n",
        )
        .unwrap();

        // Command
        let cmds_dir = dir.path().join(".claude/commands");
        fs::create_dir_all(&cmds_dir).unwrap();
        fs::write(cmds_dir.join("deploy.md"), "# Deploy").unwrap();

        // Root SKILL.md
        fs::write(
            dir.path().join("SKILL.md"),
            "---\nname: root\n---\n# Root",
        )
        .unwrap();

        let result = scan_all(dir.path()).unwrap();
        // Should have: alpha, a commands skill (name = temp dir name), root
        assert!(result.skills.len() >= 2);

        let names: Vec<&str> = result.skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"root"));
    }

    #[test]
    fn test_scan_all_dedup_prefers_richer() {
        let dir = setup_dir();

        // Same skill in plugin.json (higher priority) and SKILL.md (lower)
        let plugin = serde_json::json!({
            "name": "test",
            "skills": [{
                "name": "my-skill",
                "description": "From plugin",
                "entrypoint": "plugin-skill.md"
            }]
        });
        fs::write(
            dir.path().join("plugin.json"),
            serde_json::to_string(&plugin).unwrap(),
        )
        .unwrap();

        fs::write(
            dir.path().join("SKILL.md"),
            "---\nname: my-skill\ndescription: From SKILL.md\n---\n",
        )
        .unwrap();

        let result = scan_all(dir.path()).unwrap();
        let skill = result.skills.iter().find(|s| s.name == "my-skill").unwrap();
        // Plugin has higher priority
        assert_eq!(skill.source_format, SourceFormat::ClaudePlugin);
        assert_eq!(skill.description, Some("From plugin".to_string()));
    }

    #[test]
    fn test_empty_skill_directory() {
        let dir = setup_dir();
        fs::create_dir_all(dir.path().join(".claude/skills")).unwrap();
        let result = scan_claude_skills(dir.path()).unwrap();
        assert!(result.is_empty());
    }
}
