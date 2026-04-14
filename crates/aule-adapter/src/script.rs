//! Script adapter protocol — subprocess execution for script-based adapters.
//!
//! Script adapters receive manifest+content as JSON on stdin and return
//! generated files as JSON on stdout.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::adapter_def::ScriptAdapter;
use crate::generate::GeneratedFile;

/// Maximum file size for script output (10MB).
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// JSON input sent to the generate script on stdin.
#[derive(Debug, Serialize)]
pub struct ScriptInput {
    pub protocol_version: u32,
    pub manifest: serde_json::Value,
    pub content: ScriptContent,
    pub adapter_config: serde_json::Value,
    pub options: ScriptOptions,
}

/// Resolved content for the script.
#[derive(Debug, Serialize)]
pub struct ScriptContent {
    /// Skill name → raw markdown content.
    pub skills: HashMap<String, String>,
    /// Skill name → (command name → command body).
    pub commands: HashMap<String, HashMap<String, String>>,
    /// Relative path → file content (for v0.2.0 file includes).
    pub files: HashMap<String, String>,
}

/// Options passed to the script.
#[derive(Debug, Serialize)]
pub struct ScriptOptions {
    pub output_dir: Option<String>,
    pub base_path: String,
}

/// JSON output from the generate script (stdout).
#[derive(Debug, Deserialize)]
pub struct ScriptOutput {
    pub files: Vec<ScriptOutputFile>,
}

/// A file produced by the generate script.
#[derive(Debug, Deserialize)]
pub struct ScriptOutputFile {
    pub relative_path: String,
    pub content: String,
}

/// Structured error from a script (stderr JSON).
#[derive(Debug, Deserialize)]
pub struct ScriptError {
    pub error: String,
    #[serde(default)]
    pub details: Vec<ScriptErrorDetail>,
}

#[derive(Debug, Deserialize)]
pub struct ScriptErrorDetail {
    #[serde(default)]
    pub field: Option<String>,
    pub message: String,
}

/// Validation result from a validation script.
#[derive(Debug, Deserialize)]
pub struct ValidationOutput {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<ValidationIssue>,
    #[serde(default)]
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationIssue {
    #[serde(default)]
    pub field: Option<String>,
    pub message: String,
}

/// Errors from script execution.
#[derive(Debug, thiserror::Error)]
pub enum ScriptExecError {
    #[error("script failed: {0}")]
    Failed(String),
    #[error("script output invalid JSON: {0}")]
    InvalidOutput(String),
    #[error("script output file path contains path traversal: {0}")]
    PathTraversal(String),
    #[error("script output file path is absolute: {0}")]
    AbsolutePath(String),
    #[error("script output file exceeds 10MB size limit: {0}")]
    FileTooLarge(String),
    #[error("validation failed with errors")]
    ValidationFailed(ValidationOutput),
    #[error("validation script crashed: {0}")]
    ValidationCrashed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Execute a generate script and return the generated files.
pub fn execute_generate_script(
    adapter: &ScriptAdapter,
    input: &ScriptInput,
) -> Result<Vec<GeneratedFile>, ScriptExecError> {
    let output_json = run_script(adapter, &adapter.generate, input)?;

    let output: ScriptOutput = serde_json::from_str(&output_json)
        .map_err(|e| ScriptExecError::InvalidOutput(e.to_string()))?;

    // Validate output files
    let mut files = Vec::new();
    for file in output.files {
        validate_output_path(&file.relative_path)?;
        if file.content.len() > MAX_FILE_SIZE {
            return Err(ScriptExecError::FileTooLarge(file.relative_path));
        }
        files.push(GeneratedFile {
            relative_path: file.relative_path,
            content: file.content,
        });
    }

    Ok(files)
}

/// Execute a validation script and return the result.
pub fn execute_validate_script(
    adapter: &ScriptAdapter,
    validate_script: &str,
    input: &ScriptInput,
) -> Result<ValidationOutput, ScriptExecError> {
    let output_json = run_script(adapter, validate_script, input)?;

    let output: ValidationOutput = serde_json::from_str(&output_json)
        .map_err(|e| ScriptExecError::ValidationCrashed(
            format!("invalid JSON from validation script: {}", e)
        ))?;

    Ok(output)
}

/// Run a script with JSON input on stdin, return stdout.
fn run_script(
    adapter: &ScriptAdapter,
    script_path: &str,
    input: &ScriptInput,
) -> Result<String, ScriptExecError> {
    let working_dir = adapter.adapter_dir.as_deref()
        .unwrap_or_else(|| Path::new("."));

    let input_json = serde_json::to_string(input)
        .map_err(|e| ScriptExecError::Failed(format!("failed to serialize input: {}", e)))?;

    let mut child = Command::new(script_path)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ScriptExecError::Failed(
            format!("failed to execute {}: {}", script_path, e)
        ))?;

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input_json.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Try to parse structured error from stderr
        if let Ok(err) = serde_json::from_str::<ScriptError>(&stderr) {
            return Err(ScriptExecError::Failed(err.error));
        }

        // Fallback to raw stderr
        return Err(ScriptExecError::Failed(
            if stderr.is_empty() {
                format!("script exited with code {:?}", output.status.code())
            } else {
                stderr.trim().to_string()
            }
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| ScriptExecError::InvalidOutput(format!("non-UTF-8 stdout: {}", e)))
}

/// Validate that an output file path is safe.
fn validate_output_path(path: &str) -> Result<(), ScriptExecError> {
    // Must be relative
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(ScriptExecError::AbsolutePath(path.to_string()));
    }

    // Windows absolute paths
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err(ScriptExecError::AbsolutePath(path.to_string()));
    }

    // No path traversal
    for component in path.split('/') {
        if component == ".." {
            return Err(ScriptExecError::PathTraversal(path.to_string()));
        }
    }
    for component in path.split('\\') {
        if component == ".." {
            return Err(ScriptExecError::PathTraversal(path.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_rejects_traversal() {
        assert!(validate_output_path("../../etc/passwd").is_err());
        assert!(validate_output_path("foo/../bar").is_err());
        assert!(validate_output_path("..").is_err());
    }

    #[test]
    fn validate_path_rejects_absolute() {
        assert!(validate_output_path("/usr/local/bin/evil").is_err());
        assert!(validate_output_path("C:\\evil").is_err());
    }

    #[test]
    fn validate_path_accepts_relative() {
        assert!(validate_output_path(".cursor/rules/my-skill.mdc").is_ok());
        assert!(validate_output_path("skills/foo/SKILL.md").is_ok());
        assert!(validate_output_path(".hidden/file").is_ok());
    }
}
