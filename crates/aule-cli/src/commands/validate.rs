use std::path::PathBuf;

use aule_schema::contract::{self, ContractSource};
use aule_schema::manifest::{self, ContractRef, ManifestAny};
use super::CliError;
use crate::output;

pub fn run(path: Option<PathBuf>, json: bool) -> Result<(), CliError> {
    let base_path = path.unwrap_or_else(|| PathBuf::from("."));
    let manifest_path = base_path.join("skill.yaml");

    let manifest_any = manifest::load_manifest_any(&manifest_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    let mut result = manifest::validate_manifest_any(&manifest_any, Some(&base_path));

    // For v0.1.0, also validate the contract separately
    if let ManifestAny::V1(ref m) = manifest_any {
        let contract_result = match &m.contract {
            ContractRef::Inline(c) => contract::validate_contract(c),
            ContractRef::File(file_path) => {
                let contract_path = base_path.join(file_path);
                match contract::parse_contract(ContractSource::File(&contract_path)) {
                    Ok(c) => contract::validate_contract(&c),
                    Err(e) => {
                        result.add_error(format!("contract parse error: {}", e));
                        aule_schema::validation::ValidationResult::new()
                    }
                }
            }
        };
        result.merge(contract_result);
    }

    let errors = result.errors();
    let warnings = result.warnings();

    if json {
        let value = serde_json::json!({
            "valid": result.is_ok(),
            "errors": errors,
            "warnings": warnings,
        });
        output::print_json(&value);
    } else {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
        for e in &errors {
            eprintln!("error: {}", e);
        }
        if result.is_ok() {
            println!("Validation passed.");
            if !warnings.is_empty() {
                println!("  {} warning(s)", warnings.len());
            }
        } else {
            eprintln!("Validation failed with {} error(s).", errors.len());
        }
    }

    if result.is_ok() {
        Ok(())
    } else {
        Err(CliError::User("validation failed".to_string()))
    }
}

/// Validate and return the manifest (any version). Used by build command.
pub fn validate_and_load_any(
    base_path: &std::path::Path,
) -> Result<ManifestAny, CliError> {
    let manifest_path = base_path.join("skill.yaml");

    let manifest_any = manifest::load_manifest_any(&manifest_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    let mut result = manifest::validate_manifest_any(&manifest_any, Some(base_path));

    // For v0.1.0, also validate the contract
    if let ManifestAny::V1(ref m) = manifest_any {
        let contract_result = match &m.contract {
            ContractRef::Inline(c) => contract::validate_contract(c),
            ContractRef::File(file_path) => {
                let contract_path = base_path.join(file_path);
                match contract::parse_contract(ContractSource::File(&contract_path)) {
                    Ok(c) => contract::validate_contract(&c),
                    Err(e) => {
                        result.add_error(format!("contract parse error: {}", e));
                        aule_schema::validation::ValidationResult::new()
                    }
                }
            }
        };
        result.merge(contract_result);
    }

    if !result.is_ok() {
        let errors = result.errors();
        return Err(CliError::User(format!(
            "validation failed: {}",
            errors.join("; ")
        )));
    }

    // Print warnings to stderr even when called from build
    for w in result.warnings() {
        eprintln!("warning: {}", w);
    }

    Ok(manifest_any)
}

/// Validate and return a v0.1.0 manifest. Used by commands that only support v0.1.0.
pub fn validate_and_load(
    base_path: &std::path::Path,
) -> Result<manifest::Manifest, CliError> {
    let manifest_path = base_path.join("skill.yaml");

    let manifest = manifest::load_manifest(&manifest_path)
        .map_err(|e| CliError::User(e.to_string()))?;

    let mut result = manifest::validate_manifest(&manifest, Some(base_path));

    let contract_result = match &manifest.contract {
        ContractRef::Inline(c) => contract::validate_contract(c),
        ContractRef::File(file_path) => {
            let contract_path = base_path.join(file_path);
            match contract::parse_contract(ContractSource::File(&contract_path)) {
                Ok(c) => contract::validate_contract(&c),
                Err(e) => {
                    result.add_error(format!("contract parse error: {}", e));
                    aule_schema::validation::ValidationResult::new()
                }
            }
        }
    };
    result.merge(contract_result);

    if !result.is_ok() {
        let errors = result.errors();
        return Err(CliError::User(format!(
            "validation failed: {}",
            errors.join("; ")
        )));
    }

    for w in result.warnings() {
        eprintln!("warning: {}", w);
    }

    Ok(manifest)
}
