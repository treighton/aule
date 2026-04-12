use serde::{Deserialize, Serialize};

use crate::{CacheError, CacheManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationRecord {
    pub skill_name: String,
    pub version: String,
    pub identity_hash: String,
    pub activated_at: String,
    pub output_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivationState {
    pub records: Vec<ActivationRecord>,
}

impl ActivationState {
    fn file_path(mgr: &CacheManager, runtime_id: &str) -> std::path::PathBuf {
        mgr.root().join("activations").join(format!("{runtime_id}.json"))
    }

    /// Loads the activation state for a runtime.
    pub fn load(mgr: &CacheManager, runtime_id: &str) -> Result<Self, CacheError> {
        let path = Self::file_path(mgr, runtime_id);
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let state: ActivationState = serde_json::from_str(&data)?;
        Ok(state)
    }

    fn save(&self, mgr: &CacheManager, runtime_id: &str) -> Result<(), CacheError> {
        let path = Self::file_path(mgr, runtime_id);
        std::fs::create_dir_all(path.parent().unwrap())?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Activates a skill for the given runtime.
    pub fn activate(
        mgr: &CacheManager,
        runtime_id: &str,
        record: ActivationRecord,
    ) -> Result<(), CacheError> {
        let mut state = Self::load(mgr, runtime_id)?;
        // Replace existing activation for same skill
        state
            .records
            .retain(|r| r.skill_name != record.skill_name);
        state.records.push(record);
        state.save(mgr, runtime_id)?;
        Ok(())
    }

    /// Deactivates a skill for the given runtime. Also deletes files in output_paths.
    pub fn deactivate(
        mgr: &CacheManager,
        runtime_id: &str,
        skill_name: &str,
    ) -> Result<(), CacheError> {
        let mut state = Self::load(mgr, runtime_id)?;
        // Find the record and delete output files
        if let Some(record) = state.records.iter().find(|r| r.skill_name == skill_name) {
            for path_str in &record.output_paths {
                let path = std::path::Path::new(path_str);
                if path.exists() {
                    if path.is_dir() {
                        let _ = std::fs::remove_dir_all(path);
                    } else {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
        state.records.retain(|r| r.skill_name != skill_name);
        state.save(mgr, runtime_id)?;
        Ok(())
    }

    /// Lists all active skills for a runtime.
    pub fn list_active(
        mgr: &CacheManager,
        runtime_id: &str,
    ) -> Result<Vec<ActivationRecord>, CacheError> {
        let state = Self::load(mgr, runtime_id)?;
        Ok(state.records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activate_and_list() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let record = ActivationRecord {
            skill_name: "my-skill".into(),
            version: "1.0.0".into(),
            identity_hash: "abc123".into(),
            activated_at: "2026-01-01T00:00:00Z".into(),
            output_paths: vec![],
        };

        ActivationState::activate(&mgr, "runtime-1", record).unwrap();
        let active = ActivationState::list_active(&mgr, "runtime-1").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].skill_name, "my-skill");
    }

    #[test]
    fn test_deactivate_deletes_output_files() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        // Create output files
        let output_file = tmp.path().join("output.txt");
        std::fs::write(&output_file, b"generated content").unwrap();
        assert!(output_file.exists());

        let record = ActivationRecord {
            skill_name: "my-skill".into(),
            version: "1.0.0".into(),
            identity_hash: "abc123".into(),
            activated_at: "2026-01-01T00:00:00Z".into(),
            output_paths: vec![output_file.to_string_lossy().into()],
        };

        ActivationState::activate(&mgr, "rt-1", record).unwrap();
        ActivationState::deactivate(&mgr, "rt-1", "my-skill").unwrap();

        assert!(!output_file.exists());
        let active = ActivationState::list_active(&mgr, "rt-1").unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn test_activate_replaces_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let r1 = ActivationRecord {
            skill_name: "s1".into(),
            version: "1.0.0".into(),
            identity_hash: "h1".into(),
            activated_at: "2026-01-01T00:00:00Z".into(),
            output_paths: vec![],
        };
        let r2 = ActivationRecord {
            skill_name: "s1".into(),
            version: "2.0.0".into(),
            identity_hash: "h2".into(),
            activated_at: "2026-01-02T00:00:00Z".into(),
            output_paths: vec![],
        };

        ActivationState::activate(&mgr, "rt", r1).unwrap();
        ActivationState::activate(&mgr, "rt", r2).unwrap();

        let active = ActivationState::list_active(&mgr, "rt").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].version, "2.0.0");
    }
}
