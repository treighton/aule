use serde::{Deserialize, Serialize};
use serde::de::Error as _;
use std::path::Path;
use thiserror::Error;

use crate::permissions;
use crate::validation::ValidationResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub version: String,
    pub inputs: InputOutput,
    pub outputs: InputOutput,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default = "default_determinism")]
    pub determinism: Determinism,
    #[serde(default)]
    pub errors: Option<Vec<ContractError>>,
    #[serde(default)]
    pub behavior: Option<BehavioralMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputOutput {
    Prompt(PromptMarker),
    Schema(serde_json::Value),
}

// Serde trick: "prompt" string deserializes into this
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptMarker {
    #[serde(rename = "prompt")]
    Prompt,
}

impl InputOutput {
    pub fn is_prompt(&self) -> bool {
        matches!(self, InputOutput::Prompt(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Determinism {
    Deterministic,
    Bounded,
    Probabilistic,
}

fn default_determinism() -> Determinism {
    Determinism::Probabilistic
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractError {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehavioralMetadata {
    #[serde(default)]
    pub latency_class: Option<LatencyClass>,
    #[serde(default)]
    pub cost_class: Option<CostClass>,
    #[serde(default)]
    pub side_effects: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatencyClass {
    Fast,
    Moderate,
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CostClass {
    Free,
    Low,
    Medium,
    High,
}

// --- Errors ---

#[derive(Debug, Error)]
pub enum ContractParseError {
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("contract file not found: {0}")]
    NotFound(String),
}

// --- Source for parsing ---

pub enum ContractSource<'a> {
    Inline(&'a serde_json::Value),
    File(&'a Path),
}

// --- Parsing ---

pub fn parse_contract_from_yaml(yaml: &str) -> Result<Contract, ContractParseError> {
    let contract: Contract = serde_yaml::from_str(yaml)?;
    Ok(contract)
}

pub fn parse_contract(source: ContractSource) -> Result<Contract, ContractParseError> {
    match source {
        ContractSource::Inline(value) => {
            let contract: Contract = serde_json::from_value(value.clone())
                .map_err(|e| serde_yaml::Error::custom(e.to_string()))?;
            Ok(contract)
        }
        ContractSource::File(path) => {
            let content = std::fs::read_to_string(path)
                .map_err(|_| ContractParseError::NotFound(path.display().to_string()))?;
            parse_contract_from_yaml(&content)
        }
    }
}

// --- Validation ---

pub fn validate_contract(contract: &Contract) -> ValidationResult {
    let mut result = ValidationResult::new();

    // version must be semver
    if contract.version.split('.').count() != 3
        || contract
            .version
            .split('.')
            .any(|p| p.parse::<u64>().is_err())
    {
        result.add_error(format!(
            "contract version must be valid semver, got \"{}\"",
            contract.version
        ));
    }

    // validate permissions against vocabulary
    for perm in &contract.permissions {
        let check = permissions::validate_permission(perm);
        if !check.valid_format {
            result.add_error(format!("permission \"{}\" has invalid format", perm));
        } else if !check.known {
            result.add_warning(format!(
                "permission \"{}\" is not in the v0 vocabulary",
                perm
            ));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_prompt_based_contract() {
        let yaml = r#"
version: "1.0.0"
inputs: "prompt"
outputs: "prompt"
permissions: []
determinism: "probabilistic"
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        assert!(contract.inputs.is_prompt());
        assert!(contract.outputs.is_prompt());
        assert_eq!(contract.determinism, Determinism::Probabilistic);
    }

    #[test]
    fn parse_structured_contract() {
        let yaml = r#"
version: "1.0.0"
inputs:
  type: "object"
  properties:
    query:
      type: "string"
  required: ["query"]
outputs: "prompt"
permissions: ["filesystem.read"]
determinism: "bounded"
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        assert!(!contract.inputs.is_prompt());
        assert!(contract.outputs.is_prompt());
        assert_eq!(contract.determinism, Determinism::Bounded);
    }

    #[test]
    fn default_determinism_is_probabilistic() {
        let yaml = r#"
version: "1.0.0"
inputs: "prompt"
outputs: "prompt"
permissions: []
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        assert_eq!(contract.determinism, Determinism::Probabilistic);
    }

    #[test]
    fn validate_unknown_permission_warns() {
        let yaml = r#"
version: "1.0.0"
inputs: "prompt"
outputs: "prompt"
permissions: ["quantum.entangle"]
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        let result = validate_contract(&contract);
        assert!(result.is_ok()); // warnings only
        assert!(!result.warnings().is_empty());
    }

    #[test]
    fn validate_bad_version() {
        let yaml = r#"
version: "not-semver"
inputs: "prompt"
outputs: "prompt"
permissions: []
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        let result = validate_contract(&contract);
        assert!(!result.is_ok());
    }

    #[test]
    fn parse_with_behavioral_metadata() {
        let yaml = r#"
version: "1.0.0"
inputs: "prompt"
outputs: "prompt"
permissions: []
behavior:
  latencyClass: "slow"
  sideEffects: true
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        let behavior = contract.behavior.unwrap();
        assert!(matches!(behavior.latency_class, Some(LatencyClass::Slow)));
        assert_eq!(behavior.side_effects, Some(true));
    }

    #[test]
    fn parse_with_custom_errors() {
        let yaml = r#"
version: "1.0.0"
inputs: "prompt"
outputs: "prompt"
permissions: []
errors:
  - code: "CONTEXT_TOO_LARGE"
    description: "Input exceeds context window"
"#;
        let contract = parse_contract_from_yaml(yaml).unwrap();
        let errors = contract.errors.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "CONTEXT_TOO_LARGE");
    }
}
