use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    messages: Vec<ValidationMessage>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add(&mut self, msg: ValidationMessage) {
        self.messages.push(msg);
    }

    pub fn add_error(&mut self, message: String) {
        self.add(ValidationMessage {
            severity: Severity::Error,
            message,
        });
    }

    pub fn add_warning(&mut self, message: String) {
        self.add(ValidationMessage {
            severity: Severity::Warning,
            message,
        });
    }

    pub fn is_ok(&self) -> bool {
        !self.messages.iter().any(|m| m.severity == Severity::Error)
    }

    pub fn errors(&self) -> Vec<String> {
        self.messages
            .iter()
            .filter(|m| m.severity == Severity::Error)
            .map(|m| m.message.clone())
            .collect()
    }

    pub fn warnings(&self) -> Vec<String> {
        self.messages
            .iter()
            .filter(|m| m.severity == Severity::Warning)
            .map(|m| m.message.clone())
            .collect()
    }

    pub fn messages(&self) -> &[ValidationMessage] {
        &self.messages
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.messages.extend(other.messages);
    }
}
