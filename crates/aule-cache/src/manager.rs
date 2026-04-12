use std::path::{Path, PathBuf};

use crate::CacheError;

#[derive(Debug, Clone)]
pub struct CacheManager {
    root: PathBuf,
}

impl CacheManager {
    /// Creates a new CacheManager using the `SKILL_HOME` env var or `~/.skills/` default.
    pub fn new() -> Result<Self, CacheError> {
        let root = if let Ok(skill_home) = std::env::var("SKILL_HOME") {
            PathBuf::from(skill_home)
        } else {
            let home = dirs::home_dir().ok_or(CacheError::HomeDirNotFound)?;
            home.join(".skills")
        };
        Ok(Self { root })
    }

    /// Creates a CacheManager with a custom root path (useful for testing).
    pub fn with_root(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }

    /// Returns the cache root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Creates required subdirectories if they don't exist.
    pub fn ensure_dirs(&self) -> Result<(), CacheError> {
        std::fs::create_dir_all(self.root.join("cache/artifacts"))?;
        std::fs::create_dir_all(self.root.join("metadata"))?;
        std::fs::create_dir_all(self.root.join("activations"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_root() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        assert_eq!(mgr.root(), tmp.path());
    }

    #[test]
    fn test_ensure_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();
        assert!(tmp.path().join("cache/artifacts").is_dir());
        assert!(tmp.path().join("metadata").is_dir());
        assert!(tmp.path().join("activations").is_dir());
    }

    #[test]
    fn test_new_with_skill_home_env() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        std::env::set_var("SKILL_HOME", &path);
        let mgr = CacheManager::new().unwrap();
        assert_eq!(mgr.root(), tmp.path());
        std::env::remove_var("SKILL_HOME");
    }
}
