use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::validation::ValidationResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataDocument {
    pub identity: String,
    pub name: String,
    pub repository: String,
    pub manifest: String,
    pub versions: Vec<VersionDescriptor>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDescriptor {
    pub version: String,
    pub contract_version: String,
    #[serde(default)]
    pub manifest: Option<String>,
    #[serde(default)]
    pub checksums: Option<HashMap<String, String>>,
}

pub fn validate_metadata_document(doc: &MetadataDocument) -> ValidationResult {
    let mut result = ValidationResult::new();

    if doc.identity.is_empty() {
        result.add_error("identity is required".to_string());
    }
    if doc.name.is_empty() {
        result.add_error("name is required".to_string());
    }
    if doc.repository.is_empty() {
        result.add_error("repository is required".to_string());
    }
    if doc.manifest.is_empty() {
        result.add_error("manifest is required".to_string());
    }
    if doc.versions.is_empty() {
        result.add_error("versions must have at least one entry".to_string());
    }

    for (i, v) in doc.versions.iter().enumerate() {
        if v.version.split('.').count() != 3
            || v.version.split('.').any(|p| p.parse::<u64>().is_err())
        {
            result.add_error(format!("versions[{}].version is not valid semver", i));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_doc() -> MetadataDocument {
        MetadataDocument {
            identity: "skills.acme.dev/workflow/explore".to_string(),
            name: "explore".to_string(),
            repository: "https://github.com/acme/skills".to_string(),
            manifest: "skill.yaml".to_string(),
            versions: vec![VersionDescriptor {
                version: "1.0.0".to_string(),
                contract_version: "1.0.0".to_string(),
                manifest: None,
                checksums: None,
            }],
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn validate_complete_document() {
        let result = validate_metadata_document(&valid_doc());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_missing_field() {
        let mut doc = valid_doc();
        doc.manifest = String::new();
        let result = validate_metadata_document(&doc);
        assert!(!result.is_ok());
    }

    #[test]
    fn validate_multiple_versions() {
        let mut doc = valid_doc();
        doc.versions.push(VersionDescriptor {
            version: "1.1.0".to_string(),
            contract_version: "1.0.0".to_string(),
            manifest: None,
            checksums: Some(HashMap::from([("sha256".to_string(), "abc123".to_string())])),
        });
        let result = validate_metadata_document(&doc);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_bad_version_format() {
        let mut doc = valid_doc();
        doc.versions[0].version = "not-semver".to_string();
        let result = validate_metadata_document(&doc);
        assert!(!result.is_ok());
    }
}
