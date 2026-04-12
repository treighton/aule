use serde::{Deserialize, Serialize};

use crate::{CacheError, CacheManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub name: String,
    pub version: String,
    pub identity_hash: String,
    pub installed_at: String,
    pub manifest_path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataIndex {
    pub entries: Vec<IndexEntry>,
}

impl MetadataIndex {
    /// Loads the metadata index from `metadata/index.json`.
    pub fn load(mgr: &CacheManager) -> Result<Self, CacheError> {
        let path = mgr.root().join("metadata/index.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let index: MetadataIndex = serde_json::from_str(&data)?;
        Ok(index)
    }

    /// Saves the metadata index to `metadata/index.json`.
    pub fn save(&self, mgr: &CacheManager) -> Result<(), CacheError> {
        let path = mgr.root().join("metadata/index.json");
        std::fs::create_dir_all(path.parent().unwrap())?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Adds an entry to the index.
    pub fn add_entry(&mut self, entry: IndexEntry) {
        // Remove existing entry with same name+version if present
        self.entries
            .retain(|e| !(e.name == entry.name && e.version == entry.version));
        self.entries.push(entry);
    }

    /// Removes an entry by name and version.
    pub fn remove_entry(&mut self, name: &str, version: &str) {
        self.entries
            .retain(|e| !(e.name == name && e.version == version));
    }

    /// Lists all installed entries.
    pub fn list_installed(&self) -> Vec<IndexEntry> {
        self.entries.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_index_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let index = MetadataIndex::load(&mgr).unwrap();
        assert!(index.entries.is_empty());
    }

    #[test]
    fn test_save_and_load_index() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let mut index = MetadataIndex::default();
        index.add_entry(IndexEntry {
            name: "test-skill".into(),
            version: "1.0.0".into(),
            identity_hash: "abc123".into(),
            installed_at: "2026-01-01T00:00:00Z".into(),
            manifest_path: "/some/path".into(),
            source: "local".into(),
        });
        index.save(&mgr).unwrap();

        let loaded = MetadataIndex::load(&mgr).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].name, "test-skill");
    }

    #[test]
    fn test_add_and_remove_entry() {
        let mut index = MetadataIndex::default();
        index.add_entry(IndexEntry {
            name: "s1".into(),
            version: "1.0.0".into(),
            identity_hash: "h1".into(),
            installed_at: "2026-01-01T00:00:00Z".into(),
            manifest_path: "/p1".into(),
            source: "local".into(),
        });
        index.add_entry(IndexEntry {
            name: "s2".into(),
            version: "2.0.0".into(),
            identity_hash: "h2".into(),
            installed_at: "2026-01-01T00:00:00Z".into(),
            manifest_path: "/p2".into(),
            source: "remote".into(),
        });
        assert_eq!(index.list_installed().len(), 2);

        index.remove_entry("s1", "1.0.0");
        assert_eq!(index.list_installed().len(), 1);
        assert_eq!(index.entries[0].name, "s2");
    }
}
