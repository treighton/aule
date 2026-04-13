use std::path::Path;
use std::process::Command;

use crate::error::CacheError;

/// Result of executing a lifecycle hook.
#[derive(Debug)]
pub struct HookResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Execute a lifecycle hook script.
///
/// The script runs with `working_dir` as the current directory.
/// Returns Ok(HookResult) on completion (even if the script fails);
/// returns Err only on execution errors (e.g., can't spawn the process).
pub fn execute_hook(script_path: &Path, working_dir: &Path) -> Result<HookResult, CacheError> {
    if !script_path.exists() {
        return Err(CacheError::HookNotFound(
            script_path.display().to_string(),
        ));
    }

    let output = Command::new("sh")
        .arg(script_path)
        .current_dir(working_dir)
        .output()
        .map_err(|e| CacheError::HookExecution(format!(
            "failed to execute hook {}: {}",
            script_path.display(),
            e
        )))?;

    let exit_code = output.status.code();
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(HookResult {
        success,
        stdout,
        stderr,
        exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn hook_success() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("hook.sh");
        std::fs::write(&script, "#!/bin/sh\necho success").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let result = execute_hook(&script, tmp.path()).unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("success"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn hook_failure() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("hook.sh");
        std::fs::write(&script, "#!/bin/sh\necho fail >&2\nexit 1").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let result = execute_hook(&script, tmp.path()).unwrap();
        assert!(!result.success);
        assert!(result.stderr.contains("fail"));
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn hook_not_found() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("missing.sh");
        let result = execute_hook(&script, tmp.path());
        assert!(result.is_err());
    }
}
