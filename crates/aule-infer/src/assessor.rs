use crate::types::{InferError, InferredSignals, LlmAssessment};

const MODEL: &str = "claude-sonnet-4-20250514";
const TIMEOUT_SECS: u64 = 30;

/// Assess whether skills can be inferred from the gathered signals.
/// Calls the Claude API with the signals and returns structured suggestions.
pub fn assess(signals: &InferredSignals) -> Result<LlmAssessment, InferError> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| InferError::NoApiKey)?;

    let request_body = build_request(signals);

    // First attempt
    match call_api(&api_key, &request_body) {
        Ok(assessment) => Ok(assessment),
        Err(InferError::LlmResponseParse(msg)) => {
            // Retry once on parse failure
            eprintln!("LLM response parse error, retrying: {}", msg);
            call_api(&api_key, &request_body)
        }
        Err(e) => Err(e),
    }
}

fn build_request(signals: &InferredSignals) -> serde_json::Value {
    let system_prompt = build_system_prompt();
    let user_message = build_user_message(signals);

    serde_json::json!({
        "model": MODEL,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{
            "role": "user",
            "content": user_message
        }]
    })
}

fn build_system_prompt() -> String {
    r#"You are analyzing a software repository to determine if it contains content that could be packaged as an AI coding skill.

A "skill" is a piece of reusable knowledge, documentation, or capability that helps an AI coding assistant do its job better. Skills can be:
- Prose guides (how to use a framework, coding standards, workflow instructions)
- CLI tool wrappers (documentation + tool declarations for executable programs)
- MCP server configurations

Your task:
1. Analyze the repository signals provided
2. Determine if any meaningful skills can be inferred
3. If yes, suggest specific skills with descriptions, permissions, and interface

Respond with ONLY a JSON object matching this schema:
{
  "can_infer": boolean,
  "confidence": number (0.0-1.0),
  "reasoning": "explanation of why or why not",
  "suggested_skills": [
    {
      "name": "kebab-case-name",
      "description": "Clear description of what this skill teaches/enables",
      "entrypoint_suggestion": "path/to/file.md (must exist in the file tree)",
      "permissions": ["filesystem.read", "filesystem.write", "network.external", "process.spawn", "network.internal"],
      "determinism": "deterministic|bounded|probabilistic",
      "inputs": null or JSON Schema object,
      "outputs": null or JSON Schema object
    }
  ],
  "suggested_tools": [
    {
      "name": "tool-name",
      "description": "What the tool does",
      "using": "node|python|shell",
      "entrypoint": "path/to/executable",
      "version": "runtime version constraint or null"
    }
  ]
}

Rules:
- Only suggest skills that genuinely make sense. "can_infer": false is a valid and preferred answer for repos that aren't skill-shaped.
- Repos with only binary assets, data files, or no documentation are NOT skill-shaped.
- Repos with good documentation, CLI tools, or coding guides ARE skill-shaped.
- Permissions must be from: filesystem.read, filesystem.write, network.external, process.spawn, network.internal
- entrypoint_suggestion MUST reference a file that exists in the provided file_tree
- Keep skill names in kebab-case
- Be conservative with permissions — only include what's clearly needed"#.to_string()
}

fn build_user_message(signals: &InferredSignals) -> String {
    let signals_json = serde_json::to_string_pretty(signals).unwrap_or_default();

    format!(
        "Analyze this repository and determine if skills can be inferred.\n\n## Repository Signals\n\n```json\n{}\n```",
        signals_json
    )
}

fn call_api(api_key: &str, request_body: &serde_json::Value) -> Result<LlmAssessment, InferError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| InferError::LlmUnavailable(e.to_string()))?;

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(request_body)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                InferError::LlmUnavailable("request timed out after 30s".to_string())
            } else if e.is_connect() {
                InferError::LlmUnavailable(format!("connection error: {}", e))
            } else {
                InferError::LlmUnavailable(e.to_string())
            }
        })?;

    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(InferError::NoApiKey);
    }

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        return Err(InferError::LlmRateLimit(retry_after));
    }

    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(InferError::LlmUnavailable(format!(
            "API returned {}: {}",
            status, body
        )));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| InferError::LlmResponseParse(e.to_string()))?;

    // Extract text content from the Claude API response
    let text = body
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            InferError::LlmResponseParse("no text content in API response".to_string())
        })?;

    // Strip markdown code fences if present
    let json_str = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(text)
        .trim();

    let assessment: LlmAssessment = serde_json::from_str(json_str).map_err(|e| {
        InferError::LlmResponseParse(format!("failed to parse LLM JSON: {} — raw: {}", e, json_str))
    })?;

    Ok(assessment)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build_system_prompt_contains_schema() {
        let prompt = build_system_prompt();
        assert!(prompt.contains("can_infer"));
        assert!(prompt.contains("suggested_skills"));
        assert!(prompt.contains("filesystem.read"));
    }

    #[test]
    fn test_build_user_message() {
        let signals = InferredSignals {
            name: Some("test-repo".to_string()),
            readme_content: Some("# Test".to_string()),
            ..InferredSignals::default()
        };

        let msg = build_user_message(&signals);
        assert!(msg.contains("test-repo"));
        assert!(msg.contains("Repository Signals"));
    }

    #[test]
    fn test_build_request_structure() {
        let signals = InferredSignals::default();
        let req = build_request(&signals);

        assert_eq!(req["model"], MODEL);
        assert!(req["system"].is_string());
        assert!(req["messages"].is_array());
    }

    #[test]
    fn test_no_api_key() {
        // Temporarily unset the key
        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let signals = InferredSignals::default();
        let result = assess(&signals);
        assert!(matches!(result, Err(InferError::NoApiKey)));

        // Restore
        if let Some(key) = original {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
    }

    #[test]
    fn test_parse_assessment_json() {
        let json = r#"{
            "can_infer": true,
            "confidence": 0.85,
            "reasoning": "Well-documented CLI tool",
            "suggested_skills": [{
                "name": "my-tool",
                "description": "A great tool",
                "entrypoint_suggestion": "README.md",
                "permissions": ["filesystem.read"],
                "determinism": "deterministic",
                "inputs": null,
                "outputs": null
            }],
            "suggested_tools": [{
                "name": "run",
                "description": "Run the tool",
                "using": "node",
                "entrypoint": "bin/run.js",
                "version": ">=18"
            }]
        }"#;

        let assessment: LlmAssessment = serde_json::from_str(json).unwrap();
        assert!(assessment.can_infer);
        assert_eq!(assessment.confidence, 0.85);
        assert_eq!(assessment.suggested_skills.len(), 1);
        assert_eq!(assessment.suggested_tools.len(), 1);
    }

    #[test]
    fn test_parse_assessment_can_infer_false() {
        let json = r#"{
            "can_infer": false,
            "confidence": 0.1,
            "reasoning": "Data-only repo",
            "suggested_skills": [],
            "suggested_tools": []
        }"#;

        let assessment: LlmAssessment = serde_json::from_str(json).unwrap();
        assert!(!assessment.can_infer);
        assert!(assessment.suggested_skills.is_empty());
    }
}
