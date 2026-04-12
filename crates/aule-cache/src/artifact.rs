use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{CacheError, CacheManager};

/// Computes the identity hash for a skill name and version.
fn identity_hash(name: &str, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{name}@{version}"));
    format!("{:x}", hasher.finalize())
}

/// Returns the path to an artifact directory.
pub fn artifact_path(mgr: &CacheManager, identity_hash: &str) -> PathBuf {
    mgr.root().join("cache/artifacts").join(identity_hash)
}

/// Installs a skill artifact by copying from source_path to the artifact directory.
/// Returns the identity hash.
pub fn install_artifact(
    mgr: &CacheManager,
    source_path: &Path,
    name: &str,
    version: &str,
) -> Result<String, CacheError> {
    let hash = identity_hash(name, version);
    let dest = artifact_path(mgr, &hash);
    std::fs::create_dir_all(&dest)?;

    if source_path.is_dir() {
        copy_dir_recursive(source_path, &dest)?;
    } else {
        let file_name = source_path
            .file_name()
            .ok_or_else(|| CacheError::NotFound("source file has no name".into()))?;
        std::fs::copy(source_path, dest.join(file_name))?;
    }

    Ok(hash)
}

/// Removes an artifact directory.
pub fn remove_artifact(mgr: &CacheManager, identity_hash: &str) -> Result<(), CacheError> {
    let path = artifact_path(mgr, identity_hash);
    if !path.exists() {
        return Err(CacheError::NotFound(format!(
            "artifact not found: {identity_hash}"
        )));
    }
    std::fs::remove_dir_all(path)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), CacheError> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            std::fs::create_dir_all(&dest_path)?;
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_and_remove_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        // Create a source file
        let src = tmp.path().join("skill.tar.gz");
        std::fs::write(&src, b"fake skill package").unwrap();

        let hash = install_artifact(&mgr, &src, "my-skill", "1.0.0").unwrap();
        assert!(!hash.is_empty());
        assert!(artifact_path(&mgr, &hash).exists());
        assert!(artifact_path(&mgr, &hash).join("skill.tar.gz").exists());

        remove_artifact(&mgr, &hash).unwrap();
        assert!(!artifact_path(&mgr, &hash).exists());
    }

    #[test]
    fn test_remove_nonexistent_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let result = remove_artifact(&mgr, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_install_directory_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());
        mgr.ensure_dirs().unwrap();

        let src_dir = tmp.path().join("skill_src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("manifest.json"), b"{}").unwrap();
        std::fs::write(src_dir.join("main.rs"), b"fn main() {}").unwrap();

        let hash = install_artifact(&mgr, &src_dir, "dir-skill", "0.2.0").unwrap();
        let art_dir = artifact_path(&mgr, &hash);
        assert!(art_dir.join("manifest.json").exists());
        assert!(art_dir.join("main.rs").exists());
    }
}
