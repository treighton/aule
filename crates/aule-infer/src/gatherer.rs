use std::path::{Path, PathBuf};

use crate::types::{ExecutableInfo, ExecutableKind, InferError, InferredSignals, SignalSource};

/// Gather all signals from a repository for LLM assessment.
/// Runs the generic gatherer first, then enriches with language-specific gatherers.
pub fn gather_signals(repo_root: &Path) -> Result<InferredSignals, InferError> {
    let mut signals = gather_generic(repo_root)?;

    // Detect primary language and enrich
    if repo_root.join("package.json").exists() {
        gather_npm(repo_root, &mut signals)?;
        if signals.signal_source == SignalSource::Generic {
            signals.signal_source = SignalSource::Npm;
        }
    }

    if repo_root.join("pyproject.toml").exists()
        || repo_root.join("setup.py").exists()
        || repo_root.join("setup.cfg").exists()
    {
        gather_python(repo_root, &mut signals)?;
        if signals.signal_source == SignalSource::Generic {
            signals.signal_source = SignalSource::Python;
        }
    }

    if repo_root.join("Cargo.toml").exists() {
        gather_rust(repo_root, &mut signals)?;
        if signals.signal_source == SignalSource::Generic {
            signals.signal_source = SignalSource::Rust;
        }
    }

    if repo_root.join("go.mod").exists() {
        gather_go(repo_root, &mut signals)?;
        if signals.signal_source == SignalSource::Generic {
            signals.signal_source = SignalSource::Go;
        }
    }

    Ok(signals)
}

/// Generic gatherer: README, file tree, license, executables.
fn gather_generic(repo_root: &Path) -> Result<InferredSignals, InferError> {
    let mut signals = InferredSignals::default();

    // README
    let readme_names = ["README.md", "readme.md", "README.rst", "README.txt", "README"];
    for name in &readme_names {
        let path = repo_root.join(name);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                // Truncate to ~32k chars (~8k tokens)
                let truncated = if content.len() > 32_000 {
                    content[..32_000].to_string()
                } else {
                    content
                };
                signals.readme_content = Some(truncated);
                break;
            }
        }
    }

    // License detection
    let license_names = ["LICENSE", "LICENSE.md", "LICENSE.txt", "LICENCE", "LICENCE.md"];
    for name in &license_names {
        let path = repo_root.join(name);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                signals.license = detect_license_type(&content);
                break;
            }
        }
    }

    // File tree (filtered, capped at 500)
    signals.file_tree = build_file_tree(repo_root, 500);

    // Executables
    let exec_dirs = ["bin", "scripts", "cli"];
    for dir_name in &exec_dirs {
        let dir = repo_root.join(dir_name);
        if dir.exists() && dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        let rel = path.strip_prefix(repo_root).unwrap_or(&path).to_path_buf();
                        signals.executables.push(ExecutableInfo {
                            name: path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            path: rel,
                            kind: ExecutableKind::Script,
                        });
                    }
                }
            }
        }
    }

    // Shell scripts at root
    if let Ok(entries) = std::fs::read_dir(repo_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "sh" || ext == "bash" {
                        let rel = path.strip_prefix(repo_root).unwrap_or(&path).to_path_buf();
                        signals.executables.push(ExecutableInfo {
                            name: path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            path: rel,
                            kind: ExecutableKind::Script,
                        });
                    }
                }
            }
        }
    }

    Ok(signals)
}

/// Build a filtered file tree, excluding common non-essential directories.
fn build_file_tree(root: &Path, max_entries: usize) -> Vec<String> {
    let exclude = [
        ".git",
        "node_modules",
        "target",
        "__pycache__",
        ".venv",
        "vendor",
        ".tox",
        "dist",
        "build",
        ".next",
    ];

    let mut entries = Vec::new();
    collect_tree(root, root, &exclude, &mut entries, max_entries, 0, 5);
    entries
}

fn collect_tree(
    base: &Path,
    current: &Path,
    exclude: &[&str],
    entries: &mut Vec<String>,
    max: usize,
    depth: usize,
    max_depth: usize,
) {
    if entries.len() >= max || depth > max_depth {
        return;
    }

    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut items: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    items.sort_by_key(|e| e.file_name());

    for entry in items {
        if entries.len() >= max {
            return;
        }

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if exclude.iter().any(|&ex| name_str == ex) {
            continue;
        }

        if let Ok(rel) = path.strip_prefix(base) {
            entries.push(rel.to_string_lossy().to_string());
        }

        if path.is_dir() {
            collect_tree(base, &path, exclude, entries, max, depth + 1, max_depth);
        }
    }
}

/// Detect license type from content.
fn detect_license_type(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    if lower.contains("mit license") || lower.contains("permission is hereby granted, free of charge") {
        Some("MIT".to_string())
    } else if lower.contains("apache license") && lower.contains("version 2.0") {
        Some("Apache-2.0".to_string())
    } else if lower.contains("gnu general public license") {
        if lower.contains("version 3") {
            Some("GPL-3.0".to_string())
        } else {
            Some("GPL-2.0".to_string())
        }
    } else if lower.contains("bsd 2-clause") || lower.contains("simplified bsd") {
        Some("BSD-2-Clause".to_string())
    } else if lower.contains("bsd 3-clause") || lower.contains("new bsd") {
        Some("BSD-3-Clause".to_string())
    } else if lower.contains("isc license") {
        Some("ISC".to_string())
    } else {
        None
    }
}

/// Gather signals from `package.json`.
fn gather_npm(repo_root: &Path, signals: &mut InferredSignals) -> Result<(), InferError> {
    let path = repo_root.join("package.json");
    let content = std::fs::read_to_string(&path).map_err(|e| InferError::Gather(e.to_string()))?;
    let pkg: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| InferError::Gather(e.to_string()))?;

    if signals.name.is_none() {
        signals.name = pkg.get("name").and_then(|n| n.as_str()).map(String::from);
    }
    if signals.version.is_none() {
        signals.version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    if signals.description.is_none() {
        signals.description = pkg
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);
    }
    if signals.author.is_none() {
        signals.author = pkg
            .get("author")
            .and_then(|a| {
                a.as_str()
                    .map(String::from)
                    .or_else(|| a.get("name").and_then(|n| n.as_str()).map(String::from))
            });
    }
    if signals.license.is_none() {
        signals.license = pkg
            .get("license")
            .and_then(|l| l.as_str())
            .map(String::from);
    }

    signals.language = Some("javascript".to_string());
    signals.runtime = Some("node".to_string());

    // Engine version
    if let Some(engines) = pkg.get("engines").and_then(|e| e.as_object()) {
        if let Some(node) = engines.get("node").and_then(|n| n.as_str()) {
            signals.runtime_version = Some(node.to_string());
        }
    }

    // Bin entries
    if let Some(bin) = pkg.get("bin") {
        match bin {
            serde_json::Value::String(s) => {
                let name = signals.name.clone().unwrap_or_else(|| "bin".to_string());
                signals.executables.push(ExecutableInfo {
                    name,
                    path: PathBuf::from(s.as_str()),
                    kind: ExecutableKind::EntryPoint,
                });
            }
            serde_json::Value::Object(map) => {
                for (name, path) in map {
                    if let Some(p) = path.as_str() {
                        signals.executables.push(ExecutableInfo {
                            name: name.clone(),
                            path: PathBuf::from(p),
                            kind: ExecutableKind::EntryPoint,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Gather signals from `pyproject.toml` / `setup.py` / `setup.cfg`.
fn gather_python(repo_root: &Path, signals: &mut InferredSignals) -> Result<(), InferError> {
    signals.language = Some("python".to_string());
    signals.runtime = Some("python".to_string());

    // Try pyproject.toml first
    let pyproject = repo_root.join("pyproject.toml");
    if pyproject.exists() {
        let content =
            std::fs::read_to_string(&pyproject).map_err(|e| InferError::Gather(e.to_string()))?;
        let value: toml::Value =
            toml::from_str(&content).map_err(|e| InferError::Gather(e.to_string()))?;

        // [project] table
        if let Some(project) = value.get("project") {
            if signals.name.is_none() {
                signals.name = project
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(String::from);
            }
            if signals.version.is_none() {
                signals.version = project
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }
            if signals.description.is_none() {
                signals.description = project
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from);
            }
            if signals.license.is_none() {
                signals.license = project
                    .get("license")
                    .and_then(|l| {
                        l.as_str()
                            .map(String::from)
                            .or_else(|| l.get("text").and_then(|t| t.as_str()).map(String::from))
                    });
            }

            // requires-python
            if let Some(rp) = project.get("requires-python").and_then(|r| r.as_str()) {
                signals.runtime_version = Some(rp.to_string());
            }

            // console_scripts
            if let Some(scripts) = project.get("scripts").and_then(|s| s.as_table()) {
                for (name, entry) in scripts {
                    if let Some(ep) = entry.as_str() {
                        signals.executables.push(ExecutableInfo {
                            name: name.clone(),
                            path: PathBuf::from(ep),
                            kind: ExecutableKind::EntryPoint,
                        });
                    }
                }
            }
        }

        // Authors
        if signals.author.is_none() {
            if let Some(authors) = value
                .get("project")
                .and_then(|p| p.get("authors"))
                .and_then(|a| a.as_array())
            {
                if let Some(first) = authors.first() {
                    signals.author = first
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(String::from);
                }
            }
        }
    }

    Ok(())
}

/// Gather signals from `Cargo.toml`.
fn gather_rust(repo_root: &Path, signals: &mut InferredSignals) -> Result<(), InferError> {
    signals.language = Some("rust".to_string());
    signals.runtime = Some("shell".to_string());

    let cargo_toml = repo_root.join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml).map_err(|e| InferError::Gather(e.to_string()))?;
    let value: toml::Value =
        toml::from_str(&content).map_err(|e| InferError::Gather(e.to_string()))?;

    if let Some(pkg) = value.get("package") {
        if signals.name.is_none() {
            signals.name = pkg.get("name").and_then(|n| n.as_str()).map(String::from);
        }
        if signals.version.is_none() {
            signals.version = pkg
                .get("version")
                .and_then(|v| v.as_str())
                .map(String::from);
        }
        if signals.description.is_none() {
            signals.description = pkg
                .get("description")
                .and_then(|d| d.as_str())
                .map(String::from);
        }
        if signals.license.is_none() {
            signals.license = pkg
                .get("license")
                .and_then(|l| l.as_str())
                .map(String::from);
        }
        if signals.author.is_none() {
            if let Some(authors) = pkg.get("authors").and_then(|a| a.as_array()) {
                signals.author = authors
                    .first()
                    .and_then(|a| a.as_str())
                    .map(String::from);
            }
        }
    }

    // Detect binary targets
    if let Some(bins) = value.get("bin").and_then(|b| b.as_array()) {
        for bin in bins {
            if let (Some(name), Some(path)) = (
                bin.get("name").and_then(|n| n.as_str()),
                bin.get("path").and_then(|p| p.as_str()),
            ) {
                signals.executables.push(ExecutableInfo {
                    name: name.to_string(),
                    path: PathBuf::from(path),
                    kind: ExecutableKind::Binary,
                });
            }
        }
    }

    // Check for workspace
    if value.get("workspace").is_some() {
        if signals.name.is_none() {
            // Use directory name for workspaces
            signals.name = repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from);
        }
    }

    Ok(())
}

/// Gather signals from `go.mod`.
fn gather_go(repo_root: &Path, signals: &mut InferredSignals) -> Result<(), InferError> {
    signals.language = Some("go".to_string());
    signals.runtime = Some("shell".to_string());

    let go_mod = repo_root.join("go.mod");
    let content =
        std::fs::read_to_string(&go_mod).map_err(|e| InferError::Gather(e.to_string()))?;

    // Parse module name
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("module ") {
            let module = line.trim_start_matches("module ").trim();
            if signals.name.is_none() {
                // Use last path segment as name
                signals.name = module.rsplit('/').next().map(String::from);
            }
            break;
        }
        if line.starts_with("go ") {
            signals.runtime_version = Some(line.trim_start_matches("go ").trim().to_string());
        }
    }

    // Re-read for go version (might be after module line)
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("go ") {
            signals.runtime_version = Some(line.trim_start_matches("go ").trim().to_string());
            break;
        }
    }

    // Detect main.go or cmd/ directory
    if repo_root.join("main.go").exists() {
        signals.executables.push(ExecutableInfo {
            name: signals.name.clone().unwrap_or_else(|| "main".to_string()),
            path: PathBuf::from("main.go"),
            kind: ExecutableKind::EntryPoint,
        });
    }

    let cmd_dir = repo_root.join("cmd");
    if cmd_dir.exists() && cmd_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&cmd_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("cmd")
                        .to_string();
                    let rel = path
                        .strip_prefix(repo_root)
                        .unwrap_or(&path)
                        .to_path_buf();
                    signals.executables.push(ExecutableInfo {
                        name,
                        path: rel,
                        kind: ExecutableKind::Binary,
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_gather_generic_readme() {
        let dir = setup();
        fs::write(dir.path().join("README.md"), "# My Project\nSome content").unwrap();

        let signals = gather_generic(dir.path()).unwrap();
        assert!(signals.readme_content.is_some());
        assert!(signals.readme_content.unwrap().contains("My Project"));
    }

    #[test]
    fn test_gather_generic_readme_truncation() {
        let dir = setup();
        let big_content = "x".repeat(50_000);
        fs::write(dir.path().join("README.md"), &big_content).unwrap();

        let signals = gather_generic(dir.path()).unwrap();
        assert!(signals.readme_content.as_ref().unwrap().len() <= 32_000);
    }

    #[test]
    fn test_gather_generic_license_mit() {
        let dir = setup();
        fs::write(
            dir.path().join("LICENSE"),
            "MIT License\n\nCopyright (c) 2024",
        )
        .unwrap();

        let signals = gather_generic(dir.path()).unwrap();
        assert_eq!(signals.license, Some("MIT".to_string()));
    }

    #[test]
    fn test_gather_generic_executables() {
        let dir = setup();
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("run.sh"), "#!/bin/bash\necho hi").unwrap();

        let signals = gather_generic(dir.path()).unwrap();
        assert_eq!(signals.executables.len(), 1);
        assert_eq!(signals.executables[0].name, "run.sh");
    }

    #[test]
    fn test_gather_generic_file_tree() {
        let dir = setup();
        fs::write(dir.path().join("index.js"), "").unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.js"), "").unwrap();

        let signals = gather_generic(dir.path()).unwrap();
        assert!(!signals.file_tree.is_empty());
    }

    #[test]
    fn test_gather_generic_filters_node_modules() {
        let dir = setup();
        fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        fs::write(dir.path().join("node_modules/pkg/index.js"), "").unwrap();
        fs::write(dir.path().join("index.js"), "").unwrap();

        let signals = gather_generic(dir.path()).unwrap();
        assert!(!signals.file_tree.iter().any(|f| f.contains("node_modules")));
    }

    #[test]
    fn test_gather_npm() {
        let dir = setup();
        let pkg = serde_json::json!({
            "name": "my-tool",
            "version": "2.1.0",
            "description": "A great tool",
            "author": "Jane Doe",
            "license": "MIT",
            "bin": {
                "my-tool": "./bin/cli.js"
            },
            "engines": {
                "node": ">=18"
            }
        });
        fs::write(
            dir.path().join("package.json"),
            serde_json::to_string_pretty(&pkg).unwrap(),
        )
        .unwrap();

        let mut signals = InferredSignals::default();
        gather_npm(dir.path(), &mut signals).unwrap();

        assert_eq!(signals.name, Some("my-tool".to_string()));
        assert_eq!(signals.version, Some("2.1.0".to_string()));
        assert_eq!(signals.description, Some("A great tool".to_string()));
        assert_eq!(signals.runtime, Some("node".to_string()));
        assert_eq!(signals.runtime_version, Some(">=18".to_string()));
        assert_eq!(signals.executables.len(), 1);
        assert_eq!(signals.executables[0].name, "my-tool");
    }

    #[test]
    fn test_gather_python_pyproject() {
        let dir = setup();
        let toml_content = r#"
[project]
name = "my-py-tool"
version = "1.0.0"
description = "Python tool"
requires-python = ">=3.9"

[project.scripts]
my-cli = "my_tool.cli:main"

[[project.authors]]
name = "Author Name"
"#;
        fs::write(dir.path().join("pyproject.toml"), toml_content).unwrap();

        let mut signals = InferredSignals::default();
        gather_python(dir.path(), &mut signals).unwrap();

        assert_eq!(signals.name, Some("my-py-tool".to_string()));
        assert_eq!(signals.version, Some("1.0.0".to_string()));
        assert_eq!(signals.runtime, Some("python".to_string()));
        assert_eq!(signals.runtime_version, Some(">=3.9".to_string()));
        assert_eq!(signals.executables.len(), 1);
    }

    #[test]
    fn test_gather_rust_cargo() {
        let dir = setup();
        let toml_content = r#"
[package]
name = "my-rust-tool"
version = "0.5.0"
description = "Rust tool"
license = "Apache-2.0"
authors = ["Dev <dev@example.com>"]

[[bin]]
name = "my-tool"
path = "src/main.rs"
"#;
        fs::write(dir.path().join("Cargo.toml"), toml_content).unwrap();

        let mut signals = InferredSignals::default();
        gather_rust(dir.path(), &mut signals).unwrap();

        assert_eq!(signals.name, Some("my-rust-tool".to_string()));
        assert_eq!(signals.version, Some("0.5.0".to_string()));
        assert_eq!(signals.language, Some("rust".to_string()));
        assert_eq!(signals.executables.len(), 1);
    }

    #[test]
    fn test_gather_go() {
        let dir = setup();
        fs::write(dir.path().join("go.mod"), "module github.com/user/mytool\n\ngo 1.21\n")
            .unwrap();
        fs::write(dir.path().join("main.go"), "package main\n").unwrap();

        let mut signals = InferredSignals::default();
        gather_go(dir.path(), &mut signals).unwrap();

        assert_eq!(signals.name, Some("mytool".to_string()));
        assert_eq!(signals.language, Some("go".to_string()));
        assert_eq!(signals.runtime_version, Some("1.21".to_string()));
        assert_eq!(signals.executables.len(), 1);
    }

    #[test]
    fn test_gather_signals_multi_language() {
        let dir = setup();
        // Both package.json and Cargo.toml
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "hybrid", "version": "1.0.0"}"#,
        )
        .unwrap();
        let toml = r#"
[package]
name = "hybrid-rust"
version = "0.1.0"
"#;
        fs::write(dir.path().join("Cargo.toml"), toml).unwrap();

        let signals = gather_signals(dir.path()).unwrap();
        // First language detected wins for signal_source
        assert_eq!(signals.signal_source, SignalSource::Npm);
        // But name comes from npm (first detected)
        assert_eq!(signals.name, Some("hybrid".to_string()));
    }

    #[test]
    fn test_gather_signals_bare_repo() {
        let dir = setup();
        let signals = gather_signals(dir.path()).unwrap();
        assert_eq!(signals.signal_source, SignalSource::Generic);
        assert!(signals.name.is_none());
    }
}
