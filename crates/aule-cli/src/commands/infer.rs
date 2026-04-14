use std::io::Write;
use std::path::PathBuf;

use aule_infer::builder;
use aule_infer::gatherer;
use aule_infer::scanner;
use aule_infer::assessor;
use aule_resolver::is_git_url;

use super::CliError;
use crate::output;

pub fn run(
    source: String,
    install: bool,
    output_path: Option<PathBuf>,
    json: bool,
    yes: bool,
    force: bool,
    git_ref: Option<String>,
) -> Result<(), CliError> {
    // Resolve source: local path or git clone
    let (repo_path, temp_dir) = resolve_source(&source, git_ref.as_deref())?;

    // Check for existing skill.yaml
    let manifest_path = repo_path.join("skill.yaml");
    if manifest_path.exists() && !force {
        if json {
            let value = serde_json::json!({
                "status": "already_exists",
                "message": "This source already has a skill.yaml. Use `skill install` directly.",
            });
            output::print_json(&value);
        } else {
            println!("This source already has a skill.yaml. Use `skill install` directly.");
            println!("Use --force to re-infer anyway.");
        }
        cleanup_temp(&temp_dir);
        return Ok(());
    }

    // Stage 1: Discovery
    if !json {
        println!("Scanning known skill locations...");
    }

    let scan_result = scanner::scan_all(&repo_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    if !scan_result.skills.is_empty() {
        // Stage 1 success — build manifest from discovered skills
        let manifest = builder::build_from_discovered(&scan_result.skills, &repo_path)
            .map_err(|e| CliError::User(e.to_string()))?;

        if !json {
            print_discovery_summary(&scan_result.skills);
            print_manifest_summary(&manifest);
        }

        let yaml = builder::serialize_manifest(&manifest)
            .map_err(|e| CliError::Internal(e.to_string()))?;

        if json {
            let value = serde_json::json!({
                "stage": "discovery",
                "skills_found": scan_result.skills.len(),
                "manifest": serde_yaml::from_str::<serde_yaml::Value>(&yaml)
                    .unwrap_or(serde_yaml::Value::Null),
                "warnings": scan_result.warnings,
            });
            output::print_json(&value);
        } else {
            write_output(&yaml, output_path.as_deref())?;
        }

        if install {
            write_manifest_and_install(&yaml, &repo_path, &source, json)?;
        }

        cleanup_temp(&temp_dir);
        return Ok(());
    }

    // Stage 2: LLM Suggest
    if !json {
        println!("No skills found in known locations.\n");
        println!("Analyzing repository for inferrable skills...");
    }

    let signals = gatherer::gather_signals(&repo_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    if !json {
        print_gathered_summary(&signals);
    }

    let assessment = assessor::assess(&signals).map_err(|e| {
        match &e {
            aule_infer::InferError::NoApiKey => CliError::User(format!(
                "{}\nSet ANTHROPIC_API_KEY to enable LLM inference, or add skill artifacts to the repo manually.",
                e
            )),
            _ => CliError::User(e.to_string()),
        }
    })?;

    if !assessment.can_infer {
        if json {
            let value = serde_json::json!({
                "error": "no_skills_found",
                "message": "This repo doesn't appear to contain skill-shaped content",
                "reasoning": assessment.reasoning,
            });
            output::print_json(&value);
        } else {
            println!("\nThis repo doesn't appear to contain skill-shaped content:");
            println!("  \"{}\"", assessment.reasoning);
        }
        cleanup_temp(&temp_dir);
        return Err(CliError::User(
            "no inferrable skills found".to_string(),
        ));
    }

    // Show LLM suggestions
    if !json {
        print_assessment_summary(&assessment);
    }

    // Interactive confirmation (unless --yes or --json)
    if !yes && !json {
        if !prompt_confirm("Accept and generate skill.yaml?")? {
            println!("Cancelled.");
            cleanup_temp(&temp_dir);
            return Ok(());
        }
    }

    let manifest = builder::build_from_assessment(&assessment, &signals, &repo_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    let yaml = builder::serialize_manifest(&manifest)
        .map_err(|e| CliError::Internal(e.to_string()))?;

    if json {
        let value = serde_json::json!({
            "stage": "suggest",
            "assessment": {
                "can_infer": assessment.can_infer,
                "confidence": assessment.confidence,
                "reasoning": assessment.reasoning,
            },
            "manifest": serde_yaml::from_str::<serde_yaml::Value>(&yaml)
                .unwrap_or(serde_yaml::Value::Null),
        });
        output::print_json(&value);
    } else {
        write_output(&yaml, output_path.as_deref())?;
    }

    if install {
        write_manifest_and_install(&yaml, &repo_path, &source, json)?;
    }

    cleanup_temp(&temp_dir);
    Ok(())
}

fn resolve_source(source: &str, git_ref: Option<&str>) -> Result<(PathBuf, Option<PathBuf>), CliError> {
    if is_git_url(source) {
        let temp_dir = std::env::temp_dir().join(format!("skill-infer-{}", std::process::id()));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }

        let mut args = vec![
            "clone".to_string(),
            "--depth".to_string(),
            "1".to_string(),
        ];

        if let Some(r) = git_ref {
            args.push("--branch".to_string());
            args.push(r.to_string());
        }

        args.push(source.to_string());
        args.push(temp_dir.to_string_lossy().to_string());

        let status = std::process::Command::new("git")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .map_err(|e| CliError::Internal(format!("failed to run git clone: {}", e)))?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(CliError::User(format!("failed to clone {}", source)));
        }

        Ok((temp_dir.clone(), Some(temp_dir)))
    } else {
        let path = PathBuf::from(source);
        let path = std::fs::canonicalize(&path)
            .map_err(|e| CliError::User(format!("invalid path '{}': {}", source, e)))?;
        Ok((path, None))
    }
}

fn write_output(yaml: &str, output_path: Option<&std::path::Path>) -> Result<(), CliError> {
    match output_path {
        Some(path) => {
            std::fs::write(path, yaml)?;
            println!("\nWrote skill.yaml to {}", path.display());
        }
        None => {
            println!("\n--- Generated skill.yaml ---");
            println!("{}", yaml);
        }
    }
    Ok(())
}

fn write_manifest_and_install(
    yaml: &str,
    repo_path: &std::path::Path,
    source: &str,
    json: bool,
) -> Result<(), CliError> {
    let manifest_path = repo_path.join("skill.yaml");
    std::fs::write(&manifest_path, yaml)?;

    if !json {
        println!("Wrote skill.yaml to {}", manifest_path.display());
        println!("Installing...");
    }

    // Delegate to install command (infer=false since we already wrote skill.yaml)
    super::install::run(
        source.to_string(),
        None,
        None,
        None,
        json,
        false,
    )
}

fn cleanup_temp(temp_dir: &Option<PathBuf>) {
    if let Some(dir) = temp_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
}

fn print_discovery_summary(skills: &[aule_infer::DiscoveredSkill]) {
    use aule_infer::SourceFormat;

    let mut by_source: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for skill in skills {
        let label = match &skill.source_format {
            SourceFormat::ClaudeSkill => ".claude/skills/",
            SourceFormat::CodexSkill => ".codex/skills/",
            SourceFormat::ClaudeCommand => ".claude/commands/",
            SourceFormat::ClaudePlugin => "plugin.json",
            SourceFormat::StandaloneSkillMd => "SKILL.md",
        };
        by_source.entry(label).or_default().push(&skill.name);
    }

    for (source, names) in &by_source {
        println!("  Found {} skill(s) in {}", names.len(), source);
    }

    let total_commands: usize = skills.iter().map(|s| s.commands.len()).sum();
    if total_commands > 0 {
        println!("  Found {} command(s)", total_commands);
    }
}

fn print_manifest_summary(manifest: &aule_schema::manifest::ManifestV2) {
    let skill_names: Vec<&String> = manifest.skills.keys().collect();
    let adapters: Vec<&String> = manifest.adapters.keys().collect();

    println!("\nGenerated skill.yaml (v0.2.0):");
    println!("  name: {}", manifest.name);
    println!(
        "  skills: {} ({})",
        manifest.skills.len(),
        skill_names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if let Some(ref tools) = manifest.tools {
        println!("  tools: {}", tools.len());
    }
    println!(
        "  adapters: {}",
        adapters
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn print_gathered_summary(signals: &aule_infer::InferredSignals) {
    let mut parts = Vec::new();

    if let Some(ref lang) = signals.language {
        parts.push(format!("{} project", lang));
    }

    if signals.readme_content.is_some() {
        let word_count = signals
            .readme_content
            .as_ref()
            .map(|c| c.split_whitespace().count())
            .unwrap_or(0);
        parts.push(format!("README ({:.1}k words)", word_count as f64 / 1000.0));
    }

    if !signals.executables.is_empty() {
        parts.push(format!("{} executable(s)", signals.executables.len()));
    }

    if !parts.is_empty() {
        println!("  Gathered: {}", parts.join(", "));
    }
}

fn print_assessment_summary(assessment: &aule_infer::LlmAssessment) {
    println!(
        "\nLLM Assessment (confidence: {:.2}):",
        assessment.confidence
    );
    println!("  \"{}\"", assessment.reasoning);

    if !assessment.suggested_skills.is_empty() {
        println!("\nSuggested skills:");
        for (i, skill) in assessment.suggested_skills.iter().enumerate() {
            println!(
                "  {}. {} — \"{}\"",
                i + 1,
                skill.name,
                skill.description
            );
            println!(
                "     permissions: [{}]",
                skill.permissions.join(", ")
            );
            println!("     determinism: {}", skill.determinism);
        }
    }

    if !assessment.suggested_tools.is_empty() {
        println!("\nSuggested tools:");
        for (i, tool) in assessment.suggested_tools.iter().enumerate() {
            println!(
                "  {}. {} ({}) — \"{}\"",
                i + 1,
                tool.name,
                tool.using,
                tool.description
            );
        }
    }

    println!();
}

fn prompt_confirm(message: &str) -> Result<bool, CliError> {
    print!("? {} [Y/n] ", message);
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input.is_empty() || input == "y" || input == "yes")
}
