//! Adapter registry — discovers and resolves adapters from multiple sources.
//!
//! Sources (in precedence order, highest first):
//! 1. User-installed: `~/.skills/adapters/<id>/adapter.yaml`
//! 2. Skill-bundled: `<package>/adapters/<id>/adapter.yaml`
//! 3. Built-in: compiled into the binary

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::adapter_def::{AdapterDef, AdapterDefError, AdapterSource, parse_adapter_def_from_path};
use crate::paths::expand_home;

/// An entry in the adapter registry.
#[derive(Debug, Clone)]
pub struct AdapterEntry {
    pub def: AdapterDef,
    pub source: AdapterSource,
}

/// Registry that discovers and caches adapter definitions.
#[derive(Debug, Clone)]
pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterEntry>,
}

impl AdapterRegistry {
    /// Build a registry from all sources.
    ///
    /// `skill_base_path` is the path to the current skill package (for
    /// discovering skill-bundled adapters). Pass `None` if not building
    /// from within a skill package.
    pub fn discover(skill_base_path: Option<&Path>) -> Self {
        let mut adapters = HashMap::new();

        // 3. Built-in (lowest precedence — inserted first, overwritten by higher)
        for def in AdapterDef::all_built_in() {
            let id = def.id().to_string();
            adapters.insert(id, AdapterEntry {
                def,
                source: AdapterSource::BuiltIn,
            });
        }

        // 2. Skill-bundled (middle precedence)
        if let Some(base) = skill_base_path {
            let adapters_dir = base.join("adapters");
            if let Ok(entries) = load_adapters_from_dir(&adapters_dir) {
                for (id, def) in entries {
                    adapters.insert(id, AdapterEntry {
                        def,
                        source: AdapterSource::SkillBundled,
                    });
                }
            }
        }

        // 1. User-installed (highest precedence)
        let user_dir = expand_home("~/.skills/adapters");
        if let Ok(entries) = load_adapters_from_dir(&user_dir) {
            for (id, def) in entries {
                adapters.insert(id, AdapterEntry {
                    def,
                    source: AdapterSource::UserInstalled,
                });
            }
        }

        Self { adapters }
    }

    /// Build a registry with only built-in adapters (no disk scanning).
    pub fn built_in_only() -> Self {
        let mut adapters = HashMap::new();
        for def in AdapterDef::all_built_in() {
            let id = def.id().to_string();
            adapters.insert(id, AdapterEntry {
                def,
                source: AdapterSource::BuiltIn,
            });
        }
        Self { adapters }
    }

    /// Look up an adapter by ID.
    pub fn by_id(&self, id: &str) -> Option<&AdapterEntry> {
        self.adapters.get(id)
    }

    /// Return all available adapters, sorted by ID.
    pub fn all(&self) -> Vec<&AdapterEntry> {
        let mut entries: Vec<_> = self.adapters.values().collect();
        entries.sort_by_key(|e| e.def.id());
        entries
    }

    /// Return all adapter IDs, sorted.
    pub fn available_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.adapters.keys().cloned().collect();
        ids.sort();
        ids
    }
}

/// Scan a directory for adapter subdirectories containing adapter.yaml.
fn load_adapters_from_dir(dir: &Path) -> Result<Vec<(String, AdapterDef)>, AdapterDefError> {
    let mut results = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(results);
    }

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let adapter_yaml = path.join("adapter.yaml");
            if adapter_yaml.exists() {
                match parse_adapter_def_from_path(&adapter_yaml) {
                    Ok(def) => {
                        results.push((def.id().to_string(), def));
                    }
                    Err(e) => {
                        // Log warning but don't fail — skip invalid adapters
                        eprintln!(
                            "warning: skipping adapter at {}: {}",
                            adapter_yaml.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Return the user-installed adapters directory path.
pub fn user_adapters_dir() -> PathBuf {
    expand_home("~/.skills/adapters")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn built_in_only_has_three() {
        let reg = AdapterRegistry::built_in_only();
        assert_eq!(reg.all().len(), 3);
        assert!(reg.by_id("claude-code").is_some());
        assert!(reg.by_id("codex").is_some());
        assert!(reg.by_id("pi").is_some());
    }

    #[test]
    fn unknown_id_returns_none() {
        let reg = AdapterRegistry::built_in_only();
        assert!(reg.by_id("nonexistent").is_none());
    }

    #[test]
    fn available_ids_sorted() {
        let reg = AdapterRegistry::built_in_only();
        let ids = reg.available_ids();
        assert_eq!(ids, vec!["claude-code", "codex", "pi"]);
    }

    #[test]
    fn user_installed_overrides_built_in() {
        let tmp = TempDir::new().unwrap();

        // Create a user-installed "codex" adapter with a different description
        let codex_dir = tmp.path().join("adapters/codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("adapter.yaml"),
            r#"
id: codex
type: config
description: "Custom Codex adapter"
paths:
  skill: ".custom-codex/skills/{name}/SKILL.md"
"#,
        )
        .unwrap();

        // Build registry manually to test override
        let mut adapters = HashMap::new();

        // Add built-ins
        for def in AdapterDef::all_built_in() {
            let id = def.id().to_string();
            adapters.insert(id, AdapterEntry {
                def,
                source: AdapterSource::BuiltIn,
            });
        }

        // Override with user-installed
        let user_dir = tmp.path().join("adapters");
        if let Ok(entries) = load_adapters_from_dir(&user_dir) {
            for (id, def) in entries {
                adapters.insert(id, AdapterEntry {
                    def,
                    source: AdapterSource::UserInstalled,
                });
            }
        }

        let reg = AdapterRegistry { adapters };

        let codex = reg.by_id("codex").unwrap();
        assert_eq!(codex.source, AdapterSource::UserInstalled);
        assert_eq!(codex.def.description(), "Custom Codex adapter");
    }

    #[test]
    fn skill_bundled_overrides_built_in() {
        let tmp = TempDir::new().unwrap();

        // Create a skill-bundled adapter
        let adapter_dir = tmp.path().join("adapters/codex");
        fs::create_dir_all(&adapter_dir).unwrap();
        fs::write(
            adapter_dir.join("adapter.yaml"),
            r#"
id: codex
type: config
description: "Skill-bundled Codex"
paths:
  skill: ".custom/skills/{name}/SKILL.md"
"#,
        )
        .unwrap();

        let reg = AdapterRegistry::discover(Some(tmp.path()));
        let codex = reg.by_id("codex").unwrap();
        assert_eq!(codex.source, AdapterSource::SkillBundled);
    }

    #[test]
    fn custom_adapter_discovered() {
        let tmp = TempDir::new().unwrap();

        let gemini_dir = tmp.path().join("adapters/gemini");
        fs::create_dir_all(&gemini_dir).unwrap();
        fs::write(
            gemini_dir.join("adapter.yaml"),
            r#"
id: gemini
type: config
description: "Gemini adapter"
paths:
  skill: ".gemini/skills/{name}/SKILL.md"
"#,
        )
        .unwrap();

        let reg = AdapterRegistry::discover(Some(tmp.path()));
        assert_eq!(reg.all().len(), 4); // 3 built-in + 1 custom
        assert!(reg.by_id("gemini").is_some());
    }

    #[test]
    fn invalid_adapter_yaml_skipped() {
        let tmp = TempDir::new().unwrap();

        let bad_dir = tmp.path().join("adapters/broken");
        fs::create_dir_all(&bad_dir).unwrap();
        fs::write(bad_dir.join("adapter.yaml"), "not: valid: yaml: {").unwrap();

        let reg = AdapterRegistry::discover(Some(tmp.path()));
        // Should still have 3 built-ins, broken one skipped
        assert_eq!(reg.all().len(), 3);
    }
}
