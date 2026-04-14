use std::path::PathBuf;

use clap::Subcommand;

use aule_adapter::{
    AdapterDef, AdapterEntry, AdapterRegistry,
    parse_adapter_def_from_path,
    registry::user_adapters_dir,
    generate_any, GenerateOptions,
};
use aule_schema::manifest::ManifestAny;

use super::CliError;
use crate::output;

#[derive(Subcommand)]
pub enum AdaptersCommands {
    /// List all available adapters
    List,
    /// Install a new adapter from a local path or git URL
    Add {
        /// Path to a local adapter directory containing adapter.yaml
        #[arg(long)]
        path: Option<PathBuf>,
        /// Git URL to clone an adapter from
        #[arg(long)]
        git: Option<String>,
        /// Overwrite existing adapter
        #[arg(long)]
        force: bool,
    },
    /// Remove a user-installed adapter
    Remove {
        /// Adapter ID to remove
        id: String,
    },
    /// Show detailed information about an adapter
    Info {
        /// Adapter ID
        id: String,
    },
    /// Test an adapter by generating against a synthetic or provided skill
    Test {
        /// Adapter ID
        id: String,
        /// Path to a skill directory to test against (uses synthetic skill if omitted)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

pub fn run(command: AdaptersCommands, json: bool) -> Result<(), CliError> {
    match command {
        AdaptersCommands::List => run_list(json),
        AdaptersCommands::Add { path, git, force } => run_add(path, git, force, json),
        AdaptersCommands::Remove { id } => run_remove(&id, json),
        AdaptersCommands::Info { id } => run_info(&id, json),
        AdaptersCommands::Test { id, path } => run_test(&id, path, json),
    }
}

fn run_list(json: bool) -> Result<(), CliError> {
    let registry = AdapterRegistry::discover(None);
    let entries = registry.all();

    if json {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| adapter_entry_to_json(e))
            .collect();
        output::print_json(&serde_json::json!(items));
    } else {
        if entries.is_empty() {
            println!("No adapters available.");
            return Ok(());
        }
        println!(
            "{:<20} {:<10} {:<18} {}",
            "ID", "TYPE", "SOURCE", "DESCRIPTION"
        );
        println!("{}", "-".repeat(72));
        for e in &entries {
            println!(
                "{:<20} {:<10} {:<18} {}",
                e.def.id(),
                e.def.adapter_type_name(),
                e.source,
                e.def.description(),
            );
        }
    }

    Ok(())
}

fn run_add(
    path: Option<PathBuf>,
    git: Option<String>,
    force: bool,
    json: bool,
) -> Result<(), CliError> {
    let source_dir = match (&path, &git) {
        (Some(_), Some(_)) => {
            return Err(CliError::User(
                "Specify either --path or --git, not both.".to_string(),
            ));
        }
        (None, None) => {
            return Err(CliError::User(
                "Specify --path <dir> or --git <url>.".to_string(),
            ));
        }
        (Some(p), None) => {
            if !p.join("adapter.yaml").exists() {
                return Err(CliError::User(format!(
                    "No adapter.yaml found in {}",
                    p.display()
                )));
            }
            p.clone()
        }
        (None, Some(url)) => clone_git_adapter(url)?,
    };

    // Parse the adapter definition to get its ID
    let adapter_yaml = source_dir.join("adapter.yaml");
    let def = parse_adapter_def_from_path(&adapter_yaml)
        .map_err(|e| CliError::User(format!("Failed to parse adapter.yaml: {}", e)))?;

    let id = def.id().to_string();
    let dest = user_adapters_dir().join(&id);

    if dest.exists() && !force {
        return Err(CliError::User(format!(
            "Adapter '{}' already exists at {}. Use --force to overwrite.",
            id,
            dest.display()
        )));
    }

    // Create the user adapters directory if needed
    std::fs::create_dir_all(user_adapters_dir())
        .map_err(|e| CliError::Internal(format!("Failed to create adapters directory: {}", e)))?;

    // Remove existing if force
    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .map_err(|e| CliError::Internal(format!("Failed to remove existing adapter: {}", e)))?;
    }

    // Copy the adapter directory
    copy_dir_recursive(&source_dir, &dest)?;

    if json {
        output::print_json(&serde_json::json!({
            "status": "ok",
            "id": id,
            "path": dest.display().to_string(),
        }));
    } else {
        println!("Installed adapter '{}' to {}", id, dest.display());
    }

    Ok(())
}

fn run_remove(id: &str, json: bool) -> Result<(), CliError> {
    let dest = user_adapters_dir().join(id);

    if !dest.exists() {
        // Check if it's a built-in
        if AdapterDef::built_in_by_id(id).is_some() {
            return Err(CliError::User(
                "Cannot remove built-in adapter. Override with `skill adapters add`.".to_string(),
            ));
        }
        return Err(CliError::User(format!(
            "Adapter '{}' not found in user-installed adapters.",
            id
        )));
    }

    std::fs::remove_dir_all(&dest)
        .map_err(|e| CliError::Internal(format!("Failed to remove adapter: {}", e)))?;

    if json {
        output::print_json(&serde_json::json!({
            "status": "ok",
            "id": id,
        }));
    } else {
        println!("Removed adapter '{}'.", id);
    }

    Ok(())
}

fn run_info(id: &str, json: bool) -> Result<(), CliError> {
    let registry = AdapterRegistry::discover(None);
    let entry = registry
        .by_id(id)
        .ok_or_else(|| CliError::User(format!("Adapter '{}' not found.", id)))?;

    if json {
        output::print_json(&adapter_entry_to_json_detailed(entry));
    } else {
        println!("ID:          {}", entry.def.id());
        println!("Type:        {}", entry.def.adapter_type_name());
        println!("Source:      {}", entry.source);
        println!("Protocol:    v{}", entry.def.protocol());
        println!("Description: {}", entry.def.description());

        match &entry.def {
            AdapterDef::Config(c) => {
                println!("Paths:");
                println!("  skill: {}", c.paths.skill);
                if let Some(ref cmd) = c.paths.commands {
                    println!("  commands: {}", cmd.path);
                }
                if !c.frontmatter.extra_fields.is_empty() {
                    println!("Extra fields: {}", c.frontmatter.extra_fields.join(", "));
                }
                if let Some(ref author) = c.author {
                    println!("Author:      {}", author);
                }
            }
            AdapterDef::Script(s) => {
                println!("Generate:    {}", s.generate);
                if let Some(ref v) = s.validate {
                    println!("Validate:    {}", v);
                }
                if let Some(ref dir) = s.adapter_dir {
                    println!("Adapter dir: {}", dir.display());
                }
                if let Some(ref author) = s.author {
                    println!("Author:      {}", author);
                }
            }
        }
    }

    Ok(())
}

fn run_test(id: &str, skill_path: Option<PathBuf>, json: bool) -> Result<(), CliError> {
    let registry = AdapterRegistry::discover(None);
    let entry = registry
        .by_id(id)
        .ok_or_else(|| CliError::User(format!("Adapter '{}' not found.", id)))?;

    let mut checks: Vec<(String, bool, String)> = Vec::new();

    // Check 1: Adapter loaded successfully
    checks.push(("adapter_loaded".to_string(), true, "Adapter definition loaded".to_string()));

    // Check 2: For script adapters, verify the generate script exists and is executable
    if let AdapterDef::Script(ref s) = entry.def {
        if let Some(ref dir) = s.adapter_dir {
            let script_path = dir.join(&s.generate);
            let exists = script_path.exists();
            checks.push((
                "generate_script_exists".to_string(),
                exists,
                if exists {
                    format!("Generate script found at {}", script_path.display())
                } else {
                    format!("Generate script not found at {}", script_path.display())
                },
            ));
        }
    }

    // Check 3: For config adapters, verify path template resolves
    if let AdapterDef::Config(ref c) = entry.def {
        let resolved = c.paths.skill.replace("{name}", "test-skill");
        let valid = resolved.contains("test-skill") && !resolved.contains("{name}");
        checks.push((
            "path_template".to_string(),
            valid,
            format!("Skill path resolves to: {}", resolved),
        ));
    }

    // Check 4: Try generation against a real or synthetic skill
    let gen_result = if let Some(ref sp) = skill_path {
        try_generate_with_skill(id, sp)
    } else {
        try_generate_with_synthetic(id)
    };

    match gen_result {
        Ok(file_count) => {
            checks.push((
                "generation".to_string(),
                true,
                format!("Generated {} file(s) successfully", file_count),
            ));
        }
        Err(e) => {
            checks.push((
                "generation".to_string(),
                false,
                format!("Generation failed: {}", e),
            ));
        }
    }

    let all_passed = checks.iter().all(|(_, pass, _)| *pass);

    if json {
        let check_items: Vec<serde_json::Value> = checks
            .iter()
            .map(|(name, pass, msg)| {
                serde_json::json!({
                    "check": name,
                    "passed": pass,
                    "message": msg,
                })
            })
            .collect();
        output::print_json(&serde_json::json!({
            "adapter": id,
            "passed": all_passed,
            "checks": check_items,
        }));
    } else {
        println!("Testing adapter '{}'...\n", id);
        for (name, pass, msg) in &checks {
            let icon = if *pass { "PASS" } else { "FAIL" };
            println!("  [{}] {}: {}", icon, name, msg);
        }
        println!();
        if all_passed {
            println!("All checks passed.");
        } else {
            println!("Some checks failed.");
        }
    }

    if all_passed {
        Ok(())
    } else {
        Err(CliError::User("Adapter test failed.".to_string()))
    }
}

// --- Helper functions ---

fn adapter_entry_to_json(entry: &AdapterEntry) -> serde_json::Value {
    serde_json::json!({
        "id": entry.def.id(),
        "type": entry.def.adapter_type_name(),
        "source": entry.source.to_string(),
        "description": entry.def.description(),
    })
}

fn adapter_entry_to_json_detailed(entry: &AdapterEntry) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "id": entry.def.id(),
        "type": entry.def.adapter_type_name(),
        "source": entry.source.to_string(),
        "protocol": entry.def.protocol(),
        "description": entry.def.description(),
    });

    match &entry.def {
        AdapterDef::Config(c) => {
            obj["paths"] = serde_json::json!({
                "skill": c.paths.skill,
            });
            if let Some(ref cmd) = c.paths.commands {
                obj["paths"]["commands"] = serde_json::json!(cmd.path);
            }
            if !c.frontmatter.extra_fields.is_empty() {
                obj["extra_fields"] = serde_json::json!(c.frontmatter.extra_fields);
            }
            if let Some(ref author) = c.author {
                obj["author"] = serde_json::json!(author);
            }
        }
        AdapterDef::Script(s) => {
            obj["generate"] = serde_json::json!(s.generate);
            if let Some(ref v) = s.validate {
                obj["validate"] = serde_json::json!(v);
            }
            if let Some(ref dir) = s.adapter_dir {
                obj["adapter_dir"] = serde_json::json!(dir.display().to_string());
            }
            if let Some(ref author) = s.author {
                obj["author"] = serde_json::json!(author);
            }
        }
    }

    obj
}

fn clone_git_adapter(url: &str) -> Result<PathBuf, CliError> {
    let tmp = tempfile::TempDir::new()
        .map_err(|e| CliError::Internal(format!("Failed to create temp dir: {}", e)))?;

    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", url, &tmp.path().display().to_string()])
        .status()
        .map_err(|e| CliError::Internal(format!("Failed to run git clone: {}", e)))?;

    if !status.success() {
        return Err(CliError::User(format!(
            "git clone failed for URL: {}",
            url
        )));
    }

    if !tmp.path().join("adapter.yaml").exists() {
        return Err(CliError::User(format!(
            "No adapter.yaml found in cloned repository: {}",
            url
        )));
    }

    // We need the TempDir to persist, so leak it intentionally
    // (the caller will copy the contents and the OS will clean up on process exit)
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    Ok(path)
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), CliError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn try_generate_with_skill(adapter_id: &str, skill_path: &std::path::Path) -> Result<usize, String> {
    use super::validate;

    let manifest = validate::validate_and_load_any(skill_path)
        .map_err(|e| format!("Failed to load skill: {}", e))?;

    let registry = AdapterRegistry::discover(Some(skill_path));
    let options = GenerateOptions {
        targets: vec![adapter_id.to_string()],
        output_dir: Some(tempfile::TempDir::new().map_err(|e| e.to_string())?.path().to_path_buf()),
        registry: Some(registry),
    };

    let generated = generate_any(&manifest, skill_path, &options)
        .map_err(|e| format!("{}", e))?;

    Ok(generated.len())
}

fn try_generate_with_synthetic(adapter_id: &str) -> Result<usize, String> {
    let tmp = tempfile::TempDir::new().map_err(|e| e.to_string())?;
    let skill_dir = tmp.path();

    // Create synthetic skill.yaml
    let manifest_yaml = format!(
        r#"schemaVersion: "0.1.0"
name: "test-skill"
description: "Synthetic test skill for adapter testing"
version: "1.0.0"
content:
  skill: "content/skill.md"
contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions: []
adapters:
  {}:
    enabled: true
"#,
        adapter_id
    );

    std::fs::write(skill_dir.join("skill.yaml"), &manifest_yaml)
        .map_err(|e| e.to_string())?;

    // Create content directory and skill.md
    let content_dir = skill_dir.join("content");
    std::fs::create_dir_all(&content_dir).map_err(|e| e.to_string())?;
    std::fs::write(content_dir.join("skill.md"), "Test skill body.")
        .map_err(|e| e.to_string())?;

    let out_dir = tempfile::TempDir::new().map_err(|e| e.to_string())?;

    let registry = AdapterRegistry::discover(None);
    let options = GenerateOptions {
        targets: vec![adapter_id.to_string()],
        output_dir: Some(out_dir.path().to_path_buf()),
        registry: Some(registry),
    };

    // Load the manifest
    let yaml = std::fs::read_to_string(skill_dir.join("skill.yaml"))
        .map_err(|e| e.to_string())?;
    let manifest: aule_schema::manifest::Manifest = serde_yaml::from_str(&yaml)
        .map_err(|e| format!("Failed to parse synthetic manifest: {}", e))?;

    let manifest_any = ManifestAny::V1(manifest);
    let generated = generate_any(&manifest_any, skill_dir, &options)
        .map_err(|e| format!("{}", e))?;

    Ok(generated.len())
}
