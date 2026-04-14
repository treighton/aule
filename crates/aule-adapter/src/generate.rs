use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

use aule_schema::manifest::{Manifest, ManifestAny, ManifestV2, SkillDefinition, Tool};
use crate::adapter_def::{AdapterDef, ConfigAdapter, ScriptAdapter};
use crate::registry::AdapterRegistry;
use crate::script::{self, ScriptInput, ScriptContent, ScriptOptions};
use crate::paths::resolve_output_path;

// RuntimeTarget is kept in target.rs for backward compatibility but
// no longer used directly in generation. Tests may still reference it.

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
    /// Adapter registry to use. If None, uses built-in adapters only.
    pub registry: Option<AdapterRegistry>,
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
    #[error("script adapter error: {0}")]
    ScriptError(String),
}

// PI_EXTRA_FIELDS has been replaced by per-adapter extra_fields in AdapterDef.

/// Build YAML frontmatter string from manifest fields for a skill file.
fn build_skill_frontmatter(
    manifest: &Manifest,
    adapter_extra: &HashMap<String, serde_json::Value>,
    allowed_extra_fields: &[String],
) -> String {
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

    // Adapter-specific extra fields from adapter config
    append_adapter_extras(&mut lines, adapter_extra, allowed_extra_fields);

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

/// Append adapter extra fields to frontmatter lines.
///
/// Only includes fields listed in `allowed_fields` (from the adapter definition).
fn append_adapter_extras(
    lines: &mut Vec<String>,
    extras: &HashMap<String, serde_json::Value>,
    allowed_fields: &[String],
) {
    for key in allowed_fields {
        if let Some(value) = extras.get(key.as_str()) {
            match value {
                serde_json::Value::String(s) => {
                    lines.push(format!("{}: {}", key, s));
                }
                serde_json::Value::Bool(b) => {
                    lines.push(format!("{}: {}", key, b));
                }
                other => {
                    lines.push(format!("{}: {}", key, other));
                }
            }
        }
    }
}

/// Build YAML frontmatter for a command file (v0.1.0).
///
/// Uses the adapter's CommandConfig for display name, category, and tags.
fn build_command_frontmatter(
    manifest: &Manifest,
    command_name: &str,
    adapter: &ConfigAdapter,
) -> String {
    let cmd_config = adapter.paths.commands.as_ref().expect("command config required");
    let display_name = cmd_config.display_name
        .replace("{command}", &titlecase(command_name))
        .replace("{skill}", &manifest.name);
    let mut lines = vec!["---".to_string()];
    lines.push(format!("name: \"{}\"", display_name));
    lines.push(format!(
        "description: \"{}\"",
        manifest.description
    ));
    lines.push(format!("category: {}", cmd_config.category));
    // Build tags, resolving placeholders
    let tags: Vec<String> = cmd_config.tags.iter().map(|t| {
        t.replace("{command}", command_name)
            .replace("{skill}", &manifest.name)
    }).collect();
    lines.push(format!("tags: [{}]", tags.join(", ")));
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

/// Generate the SKILL.md file for a given config-based adapter.
pub fn generate_skill_file(
    manifest: &Manifest,
    adapter: &AdapterDef,
    content: &str,
    adapter_extra: &HashMap<String, serde_json::Value>,
) -> GeneratedFile {
    let frontmatter = build_skill_frontmatter(manifest, adapter_extra, adapter.extra_fields());
    let full_content = format!("{}\n\n{}", frontmatter, content);

    GeneratedFile {
        relative_path: adapter.skill_path(&manifest.name),
        content: full_content,
    }
}

/// Generate command files for a config-based adapter (if supported).
pub fn generate_command_files(
    manifest: &Manifest,
    adapter: &AdapterDef,
    commands: &HashMap<String, String>,
) -> Vec<GeneratedFile> {
    if !adapter.supports_commands() {
        return Vec::new();
    }

    let config_adapter = match adapter {
        AdapterDef::Config(c) => c,
        AdapterDef::Script(_) => return Vec::new(),
    };

    let namespace = derive_namespace(&manifest.name);

    commands
        .iter()
        .filter_map(|(name, body)| {
            let path = adapter.command_path(&namespace, name)?;
            let frontmatter = build_command_frontmatter(manifest, name, config_adapter);
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

    // Determine which adapters to generate for
    let registry = options.registry.as_ref()
        .map(|r| std::borrow::Cow::Borrowed(r))
        .unwrap_or_else(|| std::borrow::Cow::Owned(AdapterRegistry::built_in_only()));
    let adapters = resolve_adapters(manifest.adapters.iter(), options, &registry)?;
    if adapters.is_empty() {
        return Err(GenerateError::NoTargets);
    }

    let explicit_output = options.output_dir.is_some();
    let output_root = options.output_dir.as_deref().unwrap_or(base_path);
    let mut generated = Vec::new();

    for adapter in &adapters {
        // Look up adapter extra config for this adapter
        let empty_extra = HashMap::new();
        let adapter_extra = manifest.adapters.get(adapter.id())
            .map(|c| &c.extra)
            .unwrap_or(&empty_extra);

        // Generate skill file
        let skill_file = generate_skill_file(manifest, adapter, &skill_content, adapter_extra);
        let skill_out = resolve_output_path(output_root, &skill_file.relative_path, explicit_output);
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
        let cmd_files = generate_command_files(manifest, adapter, &command_contents);
        for cmd_file in cmd_files {
            let cmd_out = resolve_output_path(output_root, &cmd_file.relative_path, explicit_output);
            if let Some(parent) = cmd_out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&cmd_out, &cmd_file.content)?;
            generated.push(cmd_file);
        }
    }

    Ok(generated)
}

// --- v0.2.0 Generation ---

/// Build YAML frontmatter for a v0.2.0 skill.
fn build_skill_frontmatter_v2(
    manifest: &ManifestV2,
    skill_name: &str,
    skill_def: &SkillDefinition,
    adapter_extra: &HashMap<String, serde_json::Value>,
    allowed_extra_fields: &[String],
) -> String {
    let mut lines = vec!["---".to_string()];

    lines.push(format!("name: {}", skill_name));
    lines.push(format!("description: {}", skill_def.description));

    if let Some(ref meta) = manifest.metadata {
        if let Some(ref license) = meta.license {
            lines.push(format!("license: {}", license));
        }
    }

    // compatibility from tool dependencies
    if let Some(ref deps) = manifest.dependencies {
        if !deps.tools.is_empty() {
            let tools: Vec<String> = deps
                .tools
                .iter()
                .map(|t| format!("{} CLI", t.name))
                .collect();
            lines.push(format!("compatibility: Requires {}.", tools.join(", ")));
        }
    }

    // Adapter-specific extra fields from adapter config
    append_adapter_extras(&mut lines, adapter_extra, allowed_extra_fields);

    // metadata block
    if let Some(ref meta) = manifest.metadata {
        let has_metadata = meta.author.is_some() || !meta.extra.is_empty();
        if has_metadata {
            lines.push("metadata:".to_string());
            if let Some(ref author) = meta.author {
                lines.push(format!("  author: {}", author));
            }
            lines.push(format!("  version: \"{}\"", manifest.version));
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

/// Generate a wrapper script for a tool.
fn generate_wrapper_script(_tool_name: &str, tool: &Tool) -> String {
    let runtime_cmd = match tool.using.as_str() {
        "node" => "node",
        "python" => "python3",
        "shell" => "",
        other => other, // pass through for unknown runtimes
    };

    let entrypoint = &tool.entrypoint;
    let shebang = "#!/bin/sh";

    if tool.using == "shell" {
        format!(
            "{}\nexec \"$(dirname \"$0\")/../{}\" \"$@\"\n",
            shebang, entrypoint
        )
    } else {
        format!(
            "{}\nexec {} \"$(dirname \"$0\")/../{}\" \"$@\"\n",
            shebang, runtime_cmd, entrypoint
        )
    }
}

/// Generate the `## Tools` documentation section for SKILL.md.
fn generate_tools_section(tools: &HashMap<String, Tool>) -> String {
    let mut lines = vec!["\n## Tools\n".to_string()];

    // Sort tool names for deterministic output
    let mut tool_names: Vec<&String> = tools.keys().collect();
    tool_names.sort();

    for name in tool_names {
        let tool = &tools[name];
        lines.push(format!("### {}\n", name));
        lines.push(format!("{}\n", tool.description));

        // Version constraint
        if let Some(ref ver) = tool.version {
            lines.push(format!("**Runtime:** {} ({})\n", tool.using, ver));
        } else {
            lines.push(format!("**Runtime:** {}\n", tool.using));
        }

        // Invocation example
        lines.push(format!(
            "**Invocation:**\n```\n./tools/{} '{{\"input\": \"...\"}}'",
            name
        ));
        lines.push("```\n".to_string());

        // Input schema summary
        if let Some(ref input) = tool.input {
            if let Some(props) = input.get("properties") {
                if let Some(obj) = props.as_object() {
                    let required: Vec<String> = input
                        .get("required")
                        .and_then(|r| r.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();

                    lines.push("**Input:**".to_string());
                    for (prop_name, prop_schema) in obj {
                        let prop_type = prop_schema
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("any");
                        let req = if required.contains(&prop_name.to_string()) {
                            " (required)"
                        } else {
                            ""
                        };
                        lines.push(format!("- `{}`: {}{}", prop_name, prop_type, req));
                    }
                    lines.push(String::new());
                }
            }
        }

        // Output schema summary
        if let Some(ref output) = tool.output {
            if let Some(props) = output.get("properties") {
                if let Some(obj) = props.as_object() {
                    lines.push("**Output:**".to_string());
                    for (prop_name, prop_schema) in obj {
                        let prop_type = prop_schema
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("any");
                        lines.push(format!("- `{}`: {}", prop_name, prop_type));
                    }
                    lines.push(String::new());
                }
            }
        }
    }

    lines.join("\n")
}

/// Resolve file globs against a base path. Returns deduplicated relative paths.
fn resolve_file_globs(base_path: &Path, patterns: &[String]) -> Result<Vec<String>, GenerateError> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

    let glob_options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    for pattern in patterns {
        let full_pattern = base_path.join(pattern).display().to_string();
        // Try the pattern as-is, and also with a `/*` suffix for `**` patterns
        // to ensure both directory entries and files are matched
        let patterns_to_try = if full_pattern.ends_with("**") {
            vec![full_pattern.clone(), format!("{}/*", full_pattern)]
        } else {
            vec![full_pattern]
        };

        for pat in &patterns_to_try {
            let matches = glob::glob_with(pat, glob_options)
                .map_err(|e| GenerateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            for entry in matches {
                let path = entry.map_err(|e| GenerateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                if path.is_file() {
                    if let Ok(rel) = path.strip_prefix(base_path) {
                        let rel_str = rel.display().to_string();
                        if seen.insert(rel_str.clone()) {
                            files.push(rel_str);
                        }
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Generate adapter output for a v0.2.0 manifest.
pub fn generate_v2(
    manifest: &ManifestV2,
    base_path: &Path,
    options: &GenerateOptions,
) -> Result<Vec<GeneratedFile>, GenerateError> {
    let registry = options.registry.as_ref()
        .map(|r| std::borrow::Cow::Borrowed(r))
        .unwrap_or_else(|| std::borrow::Cow::Owned(AdapterRegistry::built_in_only()));
    let adapters = resolve_adapters(manifest.adapters.iter(), options, &registry)?;
    if adapters.is_empty() {
        return Err(GenerateError::NoTargets);
    }

    let explicit_output = options.output_dir.is_some();
    let output_root = options.output_dir.as_deref().unwrap_or(base_path);
    let mut generated = Vec::new();

    // Resolve file globs
    let included_files = resolve_file_globs(base_path, &manifest.files)?;

    // Sort skill names for deterministic output
    let mut skill_names: Vec<&String> = manifest.skills.keys().collect();
    skill_names.sort();

    for adapter in &adapters {
        // Look up adapter extra config for this adapter
        let empty_extra = HashMap::new();
        let adapter_extra = manifest.adapters.get(adapter.id())
            .map(|c| &c.extra)
            .unwrap_or(&empty_extra);

        // For script-based adapters, delegate entirely to the script
        if let AdapterDef::Script(script_adapter) = adapter {
            let script_files = generate_v2_via_script(
                manifest, base_path, script_adapter, adapter_extra,
                &included_files, &skill_names, options,
            )?;
            for file in script_files {
                let file_out = resolve_output_path(output_root, &file.relative_path, explicit_output);
                if let Some(parent) = file_out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&file_out, &file.content)?;
                generated.push(file);
            }
            continue;
        }

        let config_adapter = match adapter {
            AdapterDef::Config(c) => c,
            AdapterDef::Script(_) => unreachable!(),
        };

        for skill_name in &skill_names {
            let skill_def = &manifest.skills[*skill_name];

            // Read skill content
            let content_path = base_path.join(&skill_def.entrypoint);
            if !content_path.exists() {
                return Err(GenerateError::ContentNotFound(
                    skill_def.entrypoint.clone(),
                ));
            }
            let skill_content = std::fs::read_to_string(&content_path)?;

            // Build frontmatter + body
            let frontmatter = build_skill_frontmatter_v2(
                manifest, skill_name, skill_def, adapter_extra, adapter.extra_fields(),
            );
            let mut full_content = format!("{}\n\n{}", frontmatter, skill_content);

            // Append tools documentation section
            if let Some(ref tools) = manifest.tools {
                if !tools.is_empty() {
                    full_content.push_str(&generate_tools_section(tools));
                }
            }

            let skill_file = GeneratedFile {
                relative_path: adapter.skill_path(skill_name),
                content: full_content,
            };

            let skill_out = resolve_output_path(output_root, &skill_file.relative_path, explicit_output);
            if let Some(parent) = skill_out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&skill_out, &skill_file.content)?;

            // Write .generated marker
            if let Some(parent) = skill_out.parent() {
                write_generated_marker(parent, &manifest.name)?;
            }

            generated.push(skill_file);

            // Generate command files for this skill
            if let Some(ref commands) = skill_def.commands {
                let namespace = derive_namespace(skill_name);
                for (cmd_name, cmd_path) in commands {
                    if !adapter.supports_commands() {
                        continue;
                    }
                    let cmd_content_path = base_path.join(cmd_path);
                    if !cmd_content_path.exists() {
                        return Err(GenerateError::ContentNotFound(cmd_path.clone()));
                    }
                    let cmd_body = std::fs::read_to_string(&cmd_content_path)?;

                    if let Some(rel_path) = adapter.command_path(&namespace, cmd_name) {
                        let cmd_frontmatter =
                            build_command_frontmatter_v2(manifest, skill_name, cmd_name, config_adapter);
                        let cmd_file = GeneratedFile {
                            relative_path: rel_path,
                            content: format!("{}\n\n{}", cmd_frontmatter, cmd_body),
                        };
                        let cmd_out = resolve_output_path(output_root, &cmd_file.relative_path, explicit_output);
                        if let Some(parent) = cmd_out.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&cmd_out, &cmd_file.content)?;
                        generated.push(cmd_file);
                    }
                }
            }

            // Copy included files into the skill directory
            let skill_dir = skill_out
                .parent()
                .ok_or_else(|| GenerateError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "cannot determine skill output directory",
                )))?;

            for rel_file in &included_files {
                let src = base_path.join(rel_file);
                let dest = skill_dir.join(rel_file);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src, &dest)?;

                generated.push(GeneratedFile {
                    relative_path: format!(
                        "{}/{}",
                        skill_dir.strip_prefix(output_root).unwrap_or(skill_dir.as_ref()).display(),
                        rel_file
                    ),
                    content: String::new(), // file content is binary-safe via copy
                });
            }

            // Generate wrapper scripts for tools
            if let Some(ref tools) = manifest.tools {
                let mut tool_names_sorted: Vec<&String> = tools.keys().collect();
                tool_names_sorted.sort();

                for tool_name in tool_names_sorted {
                    let tool = &tools[tool_name];
                    let wrapper = generate_wrapper_script(tool_name, tool);
                    let wrapper_rel = format!(
                        "{}/tools/{}",
                        skill_dir.strip_prefix(output_root).unwrap_or(skill_dir.as_ref()).display(),
                        tool_name
                    );
                    let wrapper_path = resolve_output_path(output_root, &wrapper_rel, explicit_output);
                    if let Some(parent) = wrapper_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&wrapper_path, &wrapper)?;

                    // Mark executable
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let perms = std::fs::Permissions::from_mode(0o755);
                        std::fs::set_permissions(&wrapper_path, perms)?;
                    }

                    generated.push(GeneratedFile {
                        relative_path: wrapper_rel,
                        content: wrapper,
                    });
                }
            }
        }
    }

    Ok(generated)
}

/// Generate v0.2.0 output via a script adapter.
fn generate_v2_via_script(
    manifest: &ManifestV2,
    base_path: &Path,
    script_adapter: &ScriptAdapter,
    adapter_extra: &HashMap<String, serde_json::Value>,
    included_files: &[String],
    skill_names: &[&String],
    options: &GenerateOptions,
) -> Result<Vec<GeneratedFile>, GenerateError> {
    // Build content map
    let mut skills_content = HashMap::new();
    let mut commands_content: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut files_content = HashMap::new();

    for skill_name in skill_names {
        let skill_def = &manifest.skills[*skill_name];
        let content_path = base_path.join(&skill_def.entrypoint);
        if content_path.exists() {
            skills_content.insert(
                skill_name.to_string(),
                std::fs::read_to_string(&content_path)?,
            );
        }
        if let Some(ref commands) = skill_def.commands {
            let mut cmds = HashMap::new();
            for (cmd_name, cmd_path) in commands {
                let cmd_content_path = base_path.join(cmd_path);
                if cmd_content_path.exists() {
                    cmds.insert(cmd_name.clone(), std::fs::read_to_string(&cmd_content_path)?);
                }
            }
            if !cmds.is_empty() {
                commands_content.insert(skill_name.to_string(), cmds);
            }
        }
    }

    for rel_file in included_files {
        let src = base_path.join(rel_file);
        if src.exists() && src.is_file() {
            if let Ok(content) = std::fs::read_to_string(&src) {
                files_content.insert(rel_file.clone(), content);
            }
        }
    }

    let manifest_json = serde_json::to_value(manifest)
        .map_err(|e| GenerateError::ScriptError(format!("failed to serialize manifest: {}", e)))?;
    let adapter_config_json = serde_json::to_value(adapter_extra)
        .map_err(|e| GenerateError::ScriptError(format!("failed to serialize adapter config: {}", e)))?;

    let input = ScriptInput {
        protocol_version: script_adapter.protocol,
        manifest: manifest_json,
        content: ScriptContent {
            skills: skills_content,
            commands: commands_content,
            files: files_content,
        },
        adapter_config: adapter_config_json,
        options: ScriptOptions {
            output_dir: options.output_dir.as_ref().map(|p| p.display().to_string()),
            base_path: base_path.display().to_string(),
        },
    };

    // Run validation if adapter has a validate script
    if let Some(ref validate_script) = script_adapter.validate {
        let validation = script::execute_validate_script(script_adapter, validate_script, &input)
            .map_err(|e| GenerateError::ScriptError(format!("validation failed: {}", e)))?;
        if !validation.valid {
            let errors: Vec<String> = validation.errors.iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(GenerateError::ScriptError(
                format!("validation errors: {}", errors.join(", "))
            ));
        }
    }

    script::execute_generate_script(script_adapter, &input)
        .map_err(|e| GenerateError::ScriptError(e.to_string()))
}

/// Build command frontmatter for v0.2.0 (uses skill name for display).
fn build_command_frontmatter_v2(
    manifest: &ManifestV2,
    skill_name: &str,
    command_name: &str,
    adapter: &ConfigAdapter,
) -> String {
    let cmd_config = adapter.paths.commands.as_ref().expect("command config required");
    let display_name = cmd_config.display_name
        .replace("{command}", &titlecase(command_name))
        .replace("{skill}", skill_name);
    let skill_def = &manifest.skills[skill_name];
    let mut lines = vec!["---".to_string()];
    lines.push(format!("name: \"{}\"", display_name));
    lines.push(format!("description: \"{}\"", skill_def.description));
    lines.push(format!("category: {}", cmd_config.category));
    let tags: Vec<String> = cmd_config.tags.iter().map(|t| {
        t.replace("{command}", command_name)
            .replace("{skill}", skill_name)
    }).collect();
    lines.push(format!("tags: [{}]", tags.join(", ")));
    lines.push("---".to_string());
    lines.join("\n")
}

/// Entry point that handles any manifest version.
pub fn generate_any(
    manifest: &ManifestAny,
    base_path: &Path,
    options: &GenerateOptions,
) -> Result<Vec<GeneratedFile>, GenerateError> {
    match manifest {
        ManifestAny::V1(m) => generate(m, base_path, options),
        ManifestAny::V2(m) => generate_v2(m, base_path, options),
    }
}

/// Resolve which adapters to generate for.
///
/// If `options.targets` is non-empty, use only those explicit targets.
/// Otherwise, use all enabled adapters from the manifest's `adapters` section.
fn resolve_adapters<'a>(
    manifest_adapters: impl Iterator<Item = (&'a String, &'a aule_schema::manifest::AdapterConfig)>,
    options: &GenerateOptions,
    registry: &AdapterRegistry,
) -> Result<Vec<AdapterDef>, GenerateError> {
    if !options.targets.is_empty() {
        Ok(options
            .targets
            .iter()
            .filter_map(|id| registry.by_id(id).map(|e| e.def.clone()))
            .collect())
    } else {
        Ok(manifest_adapters
            .filter(|(_, config)| config.enabled)
            .filter_map(|(id, _)| registry.by_id(id).map(|e| e.def.clone()))
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
        let target = AdapterDef::claude_code();
        let file = generate_skill_file(&manifest, &target, TEST_SKILL_BODY, &HashMap::new());

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
        let target = AdapterDef::codex();
        let file = generate_skill_file(&manifest, &target, TEST_SKILL_BODY, &HashMap::new());

        assert_eq!(file.relative_path, ".codex/skills/test-skill/SKILL.md");
        assert!(file.content.ends_with(TEST_SKILL_BODY));
    }

    #[test]
    fn codex_skips_commands() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::codex();
        let commands: HashMap<String, String> =
            HashMap::from([("explore".to_string(), "explore body".to_string())]);

        let files = generate_command_files(&manifest, &target, &commands);
        assert!(files.is_empty());
    }

    #[test]
    fn claude_code_generates_commands() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::claude_code();
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
        let target = AdapterDef::claude_code();
        let body = "Exact content with special chars: 日本語 & <tags> \"quotes\"";
        let file = generate_skill_file(&manifest, &target, body, &HashMap::new());

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
            ..Default::default()
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
            ..Default::default()
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

    // --- v0.2.0 tests ---

    fn setup_v2_package(dir: &Path) {
        fs::create_dir_all(dir.join("content")).unwrap();
        fs::create_dir_all(dir.join("logic/tools")).unwrap();
        fs::create_dir_all(dir.join("logic/hooks")).unwrap();

        fs::write(dir.join("content/main.md"), "Main skill body.").unwrap();
        fs::write(dir.join("content/linter.md"), "Linter skill body.").unwrap();
        fs::write(dir.join("logic/tools/generate.ts"), "// generate tool").unwrap();
        fs::write(dir.join("logic/tools/run-tests.ts"), "// run-tests tool").unwrap();
        fs::write(dir.join("logic/hooks/setup.sh"), "#!/bin/sh\nnpm install").unwrap();
        fs::write(dir.join("logic/hooks/verify.sh"), "#!/bin/sh\nnode --version").unwrap();
    }

    const V2_MANIFEST: &str = r#"
schemaVersion: "0.2.0"
name: "test-v2-pkg"
description: "A v0.2.0 test package"
version: "2.0.0"
files:
  - "content/**"
  - "logic/**"
skills:
  main-skill:
    description: "The main skill"
    entrypoint: "content/main.md"
    version: "1.0.0"
    permissions:
      - "filesystem.read"
    determinism: "bounded"
  linter:
    description: "A linter skill"
    entrypoint: "content/linter.md"
    version: "1.0.0"
    permissions:
      - "filesystem.read"
    determinism: "deterministic"
tools:
  generate:
    description: "Generate test harness"
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
  run-tests:
    description: "Execute tests"
    using: "node"
    entrypoint: "logic/tools/run-tests.ts"
    input:
      type: "object"
      properties:
        baseUrl:
          type: "string"
    output:
      type: "object"
      properties:
        passed:
          type: "integer"
hooks:
  onInstall: "logic/hooks/setup.sh"
  onActivate: "logic/hooks/verify.sh"
adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true
metadata:
  author: "test"
  license: "MIT"
"#;

    #[test]
    fn v2_generates_per_skill_files() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        setup_v2_package(&src);
        fs::write(src.join("skill.yaml"), V2_MANIFEST).unwrap();

        let manifest: ManifestV2 = serde_yaml::from_str(V2_MANIFEST).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec!["claude-code".to_string()],
            output_dir: Some(output.clone()),
            ..Default::default()
        };

        let _files = generate_v2(&manifest, &src, &options).unwrap();

        // Should have SKILL.md for both skills
        assert!(output.join(".claude/skills/linter/SKILL.md").exists());
        assert!(output.join(".claude/skills/main-skill/SKILL.md").exists());

        // Check frontmatter uses skill description
        let linter_md = fs::read_to_string(output.join(".claude/skills/linter/SKILL.md")).unwrap();
        assert!(linter_md.contains("name: linter"));
        assert!(linter_md.contains("description: A linter skill"));
    }

    #[test]
    fn v2_wrapper_scripts_generated() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        setup_v2_package(&src);

        let manifest: ManifestV2 = serde_yaml::from_str(V2_MANIFEST).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec!["claude-code".to_string()],
            output_dir: Some(output.clone()),
            ..Default::default()
        };

        generate_v2(&manifest, &src, &options).unwrap();

        // Check wrapper scripts exist for each skill dir
        let gen_wrapper = output.join(".claude/skills/linter/tools/generate");
        assert!(gen_wrapper.exists(), "generate wrapper should exist");

        let wrapper_content = fs::read_to_string(&gen_wrapper).unwrap();
        assert!(wrapper_content.contains("#!/bin/sh"));
        assert!(wrapper_content.contains("exec node"));
        assert!(wrapper_content.contains("logic/tools/generate.ts"));
    }

    #[test]
    fn v2_wrapper_script_content_node() {
        let tool = Tool {
            description: "test".to_string(),
            using: "node".to_string(),
            version: Some(">= 18".to_string()),
            entrypoint: "logic/tools/gen.ts".to_string(),
            input: None,
            output: None,
        };
        let script = generate_wrapper_script("gen", &tool);
        assert!(script.starts_with("#!/bin/sh\n"));
        assert!(script.contains("exec node"));
        assert!(script.contains("logic/tools/gen.ts"));
    }

    #[test]
    fn v2_wrapper_script_content_python() {
        let tool = Tool {
            description: "test".to_string(),
            using: "python".to_string(),
            version: None,
            entrypoint: "logic/tools/analyze.py".to_string(),
            input: None,
            output: None,
        };
        let script = generate_wrapper_script("analyze", &tool);
        assert!(script.contains("exec python3"));
        assert!(script.contains("logic/tools/analyze.py"));
    }

    #[test]
    fn v2_wrapper_script_content_shell() {
        let tool = Tool {
            description: "test".to_string(),
            using: "shell".to_string(),
            version: None,
            entrypoint: "logic/tools/cleanup.sh".to_string(),
            input: None,
            output: None,
        };
        let script = generate_wrapper_script("cleanup", &tool);
        assert!(script.contains("exec \"$(dirname"));
        assert!(script.contains("logic/tools/cleanup.sh"));
        assert!(!script.contains("node"));
        assert!(!script.contains("python"));
    }

    #[test]
    fn v2_tools_section_in_skill_md() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        setup_v2_package(&src);

        let manifest: ManifestV2 = serde_yaml::from_str(V2_MANIFEST).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec!["claude-code".to_string()],
            output_dir: Some(output.clone()),
            ..Default::default()
        };

        generate_v2(&manifest, &src, &options).unwrap();

        let skill_md = fs::read_to_string(output.join(".claude/skills/main-skill/SKILL.md")).unwrap();
        assert!(skill_md.contains("## Tools"), "should contain Tools section");
        assert!(skill_md.contains("### generate"), "should list generate tool");
        assert!(skill_md.contains("### run-tests"), "should list run-tests tool");
        assert!(skill_md.contains("./tools/generate"), "should show invocation example");
    }

    #[test]
    fn v2_included_files_copied() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        setup_v2_package(&src);

        let manifest: ManifestV2 = serde_yaml::from_str(V2_MANIFEST).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec!["claude-code".to_string()],
            output_dir: Some(output.clone()),
            ..Default::default()
        };

        generate_v2(&manifest, &src, &options).unwrap();

        // Logic files should be copied into each skill dir
        assert!(output.join(".claude/skills/main-skill/logic/tools/generate.ts").exists());
        assert!(output.join(".claude/skills/main-skill/logic/hooks/setup.sh").exists());
    }

    #[test]
    fn v2_no_tools_no_tools_section() {
        let yaml = r#"
schemaVersion: "0.2.0"
name: "simple-pkg"
description: "No tools"
version: "1.0.0"
files:
  - "content/**"
skills:
  main:
    description: "A skill"
    entrypoint: "content/main.md"
    version: "1.0.0"
adapters:
  claude-code:
    enabled: true
"#;
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(src.join("content")).unwrap();
        fs::write(src.join("content/main.md"), "Skill body.").unwrap();

        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        let output = tmp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let options = GenerateOptions {
            targets: vec!["claude-code".to_string()],
            output_dir: Some(output.clone()),
            ..Default::default()
        };

        generate_v2(&manifest, &src, &options).unwrap();

        let skill_md = fs::read_to_string(output.join(".claude/skills/main/SKILL.md")).unwrap();
        assert!(!skill_md.contains("## Tools"), "should not have Tools section");
    }

    // --- Pi adapter tests ---

    #[test]
    fn generate_pi_skill() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::pi();
        let file = generate_skill_file(&manifest, &target, TEST_SKILL_BODY, &HashMap::new());

        assert_eq!(file.relative_path, "~/.pi/agent/skills/test-skill/SKILL.md");
        assert!(file.content.contains("name: test-skill"));
        assert!(file.content.contains("description: A test skill"));
        assert!(file.content.ends_with(TEST_SKILL_BODY));
    }

    #[test]
    fn pi_skips_commands() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::pi();
        let commands: HashMap<String, String> =
            HashMap::from([("explore".to_string(), "explore body".to_string())]);

        let files = generate_command_files(&manifest, &target, &commands);
        assert!(files.is_empty());
    }

    #[test]
    fn pi_frontmatter_with_allowed_tools() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::pi();
        let mut extras = HashMap::new();
        extras.insert(
            "allowed-tools".to_string(),
            serde_json::json!(["Read", "Write"]),
        );
        let file = generate_skill_file(&manifest, &target, "body", &extras);

        assert!(file.content.contains("allowed-tools: [\"Read\",\"Write\"]"),
            "should include allowed-tools in frontmatter, got:\n{}", file.content);
    }

    #[test]
    fn pi_frontmatter_with_disable_model_invocation() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::pi();
        let mut extras = HashMap::new();
        extras.insert(
            "disable-model-invocation".to_string(),
            serde_json::json!(true),
        );
        let file = generate_skill_file(&manifest, &target, "body", &extras);

        assert!(file.content.contains("disable-model-invocation: true"),
            "should include disable-model-invocation, got:\n{}", file.content);
    }

    #[test]
    fn pi_frontmatter_no_extras() {
        let manifest = parse_manifest(TEST_MANIFEST).unwrap();
        let target = AdapterDef::pi();
        let file = generate_skill_file(&manifest, &target, "body", &HashMap::new());

        assert!(!file.content.contains("allowed-tools"));
        assert!(!file.content.contains("disable-model-invocation"));
    }
}
