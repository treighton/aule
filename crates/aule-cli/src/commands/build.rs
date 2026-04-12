use std::path::PathBuf;

use aule_adapter::{generate, GenerateOptions};

use super::validate;
use super::CliError;
use crate::output;

pub fn run(
    target: Option<String>,
    output_dir: Option<PathBuf>,
    path: Option<PathBuf>,
    json: bool,
) -> Result<(), CliError> {
    let base_path = path.unwrap_or_else(|| PathBuf::from("."));

    // Validate first
    let manifest = validate::validate_and_load(&base_path)?;

    let options = GenerateOptions {
        targets: target.into_iter().collect(),
        output_dir,
    };

    let generated = generate(&manifest, &base_path, &options)
        .map_err(|e| CliError::User(e.to_string()))?;

    if json {
        let files: Vec<&str> = generated.iter().map(|f| f.relative_path.as_str()).collect();
        let value = serde_json::json!({
            "status": "ok",
            "files": files,
        });
        output::print_json(&value);
    } else {
        println!("Build complete. Generated {} file(s):", generated.len());
        for f in &generated {
            println!("  {}", f.relative_path);
        }
    }

    Ok(())
}
