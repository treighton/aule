use std::env;

use super::CliError;
use crate::output;

pub fn run(name: Option<String>, json: bool) -> Result<(), CliError> {
    let dir = env::current_dir().map_err(|e| CliError::Internal(e.to_string()))?;

    let skill_name = name.unwrap_or_else(|| {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "my-skill".to_string())
    });

    let created = aule_schema::scaffold::scaffold(&dir, &skill_name)
        .map_err(|e| CliError::User(e.to_string()))?;

    if json {
        let value = serde_json::json!({
            "status": "ok",
            "name": skill_name,
            "created": created,
        });
        output::print_json(&value);
    } else {
        println!("Initialized skill \"{}\"", skill_name);
        for file in &created {
            println!("  created {}", file);
        }
    }

    Ok(())
}
