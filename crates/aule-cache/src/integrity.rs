use crate::{activation::ActivationState, artifact, metadata::MetadataIndex, CacheError, CacheManager};

#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    /// Metadata entries whose artifact directories are missing.
    pub orphaned_artifacts: Vec<String>,
    /// Activations referencing skills not in the metadata index: (runtime_id, skill_name).
    pub broken_activations: Vec<(String, String)>,
}

/// Checks integrity of the cache:
/// - Every metadata entry should have a corresponding artifact directory.
/// - Every activation should reference an installed skill (present in metadata index).
pub fn check_integrity(mgr: &CacheManager) -> Result<IntegrityReport, CacheError> {
    let mut report = IntegrityReport::default();

    // Check metadata entries have artifact dirs
    let index = MetadataIndex::load(mgr)?;
    for entry in &index.entries {
        let path = artifact::artifact_path(mgr, &entry.identity_hash);
        if !path.exists() {
            report.orphaned_artifacts.push(entry.identity_hash.clone());
        }
    }

    // Check activations reference installed skills
    let activations_dir = mgr.root().join("activations");
    if activations_dir.exists() {
        for dir_entry in std::fs::read_dir(&activations_dir)? {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let runtime_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let active = ActivationState::list_active(mgr, &runtime_id)?;
            for record in &active {
                let installed = index
                    .entries
                    .iter()
                    .any(|e| e.identity_hash == record.identity_hash);
                if !installed {
                    report
                        .broken_activations
                        .push((runtime_id.clone(), record.skill_name.clone()));
                }
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::IndexEntry;

    #[test]
    fn test_integrity_all_good() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let report = check_integrity(&mgr).unwrap();
        assert!(report.orphaned_artifacts.is_empty());
        assert!(report.broken_activations.is_empty());
    }

    #[test]
    fn test_integrity_orphaned_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        // Add metadata entry without creating the artifact dir
        let mut index = MetadataIndex::default();
        index.add_entry(IndexEntry {
            name: "ghost-skill".into(),
            version: "1.0.0".into(),
            identity_hash: "deadbeef".into(),
            installed_at: "2026-01-01T00:00:00Z".into(),
            manifest_path: "/nowhere".into(),
            source: "local".into(),
        });
        index.save(&mgr).unwrap();

        let report = check_integrity(&mgr).unwrap();
        assert_eq!(report.orphaned_artifacts, vec!["deadbeef"]);
    }

    #[test]
    fn test_integrity_broken_activation() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        // Create an activation that references a non-installed skill
        let record = crate::ActivationRecord {
            skill_name: "missing-skill".into(),
            version: "1.0.0".into(),
            identity_hash: "not-in-index".into(),
            activated_at: "2026-01-01T00:00:00Z".into(),
            output_paths: vec![],
        };
        ActivationState::activate(&mgr, "test-runtime", record).unwrap();

        let report = check_integrity(&mgr).unwrap();
        assert_eq!(report.broken_activations.len(), 1);
        assert_eq!(report.broken_activations[0].0, "test-runtime");
        assert_eq!(report.broken_activations[0].1, "missing-skill");
    }
}
