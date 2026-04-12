use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ENVELOPE_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope {
    pub envelope_version: String,
    pub skill_name: String,
    pub contract_version: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEnvelope {
    pub envelope_version: String,
    pub status: ResponseStatus,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<ErrorDetail>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

// Standard error codes
pub mod error_codes {
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const EXECUTION_ERROR: &str = "EXECUTION_ERROR";
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const CONTRACT_MISMATCH: &str = "CONTRACT_MISMATCH";
    pub const ENVELOPE_VERSION_UNSUPPORTED: &str = "ENVELOPE_VERSION_UNSUPPORTED";
}

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("unsupported envelope version: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },
    #[error("response has status 'success' but no output")]
    MissingOutput,
    #[error("response has status 'error' but no error detail")]
    MissingError,
}

pub fn validate_request(envelope: &RequestEnvelope) -> Result<(), EnvelopeError> {
    if envelope.envelope_version != ENVELOPE_VERSION {
        return Err(EnvelopeError::VersionMismatch {
            expected: ENVELOPE_VERSION.to_string(),
            got: envelope.envelope_version.clone(),
        });
    }
    Ok(())
}

pub fn validate_response(envelope: &ResponseEnvelope) -> Result<(), EnvelopeError> {
    if envelope.envelope_version != ENVELOPE_VERSION {
        return Err(EnvelopeError::VersionMismatch {
            expected: ENVELOPE_VERSION.to_string(),
            got: envelope.envelope_version.clone(),
        });
    }
    match envelope.status {
        ResponseStatus::Success => {
            if envelope.output.is_none() {
                return Err(EnvelopeError::MissingOutput);
            }
        }
        ResponseStatus::Error => {
            if envelope.error.is_none() {
                return Err(EnvelopeError::MissingError);
            }
        }
    }
    Ok(())
}

impl ErrorDetail {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
}

impl ResponseEnvelope {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            envelope_version: ENVELOPE_VERSION.to_string(),
            status: ResponseStatus::Success,
            output: Some(output),
            error: None,
            metadata: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            envelope_version: ENVELOPE_VERSION.to_string(),
            status: ResponseStatus::Error,
            output: None,
            error: Some(ErrorDetail::new(code, message)),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid_request() {
        let req = RequestEnvelope {
            envelope_version: "0.1.0".to_string(),
            skill_name: "test".to_string(),
            contract_version: "1.0.0".to_string(),
            input: serde_json::Value::String("hello".to_string()),
            context: None,
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn validate_bad_version_request() {
        let req = RequestEnvelope {
            envelope_version: "2.0.0".to_string(),
            skill_name: "test".to_string(),
            contract_version: "1.0.0".to_string(),
            input: serde_json::Value::String("hello".to_string()),
            context: None,
        };
        assert!(matches!(
            validate_request(&req),
            Err(EnvelopeError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn validate_success_response() {
        let resp = ResponseEnvelope::success(serde_json::Value::String("output".to_string()));
        assert!(validate_response(&resp).is_ok());
    }

    #[test]
    fn validate_error_response() {
        let resp = ResponseEnvelope::error(error_codes::VALIDATION_ERROR, "bad input");
        assert!(validate_response(&resp).is_ok());
    }

    #[test]
    fn validate_success_missing_output() {
        let resp = ResponseEnvelope {
            envelope_version: "0.1.0".to_string(),
            status: ResponseStatus::Success,
            output: None,
            error: None,
            metadata: None,
        };
        assert!(matches!(
            validate_response(&resp),
            Err(EnvelopeError::MissingOutput)
        ));
    }

    #[test]
    fn validate_error_missing_detail() {
        let resp = ResponseEnvelope {
            envelope_version: "0.1.0".to_string(),
            status: ResponseStatus::Error,
            output: None,
            error: None,
            metadata: None,
        };
        assert!(matches!(
            validate_response(&resp),
            Err(EnvelopeError::MissingError)
        ));
    }

    #[test]
    fn custom_error_code() {
        let resp = ResponseEnvelope::error("CONTEXT_TOO_LARGE", "Input exceeds context window");
        let err = resp.error.unwrap();
        assert_eq!(err.code, "CONTEXT_TOO_LARGE");
    }
}
