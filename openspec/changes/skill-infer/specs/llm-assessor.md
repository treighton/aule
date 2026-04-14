## Capability: llm-assessor

Takes `InferredSignals` and calls the Claude API to determine whether skills can be inferred from the repo, and if so, returns structured suggestions that map to `ManifestV2` fields.

## Requirements

### API Integration

- Call the Claude API using the `ANTHROPIC_API_KEY` environment variable
- Use `claude-sonnet-4-20250514` model
- System prompt defines the task: "You are analyzing a software repository to determine if it contains content that could be packaged as an AI coding skill."
- System prompt includes the `ManifestV2` schema (v0.2.0) as reference
- User message includes serialized `InferredSignals` and README content
- Request structured JSON output matching `LlmAssessment` schema

### LLM Assessment Schema

```rust
pub struct LlmAssessment {
    pub can_infer: bool,
    pub confidence: f32,        // 0.0–1.0
    pub reasoning: String,      // why or why not
    pub suggested_skills: Vec<SuggestedSkill>,
    pub suggested_tools: Vec<SuggestedTool>,
}

pub struct SuggestedSkill {
    pub name: String,
    pub description: String,
    pub entrypoint_suggestion: String,  // path within repo
    pub permissions: Vec<String>,
    pub determinism: String,            // deterministic | bounded | probabilistic
    pub inputs: Option<serde_json::Value>,   // JSON Schema
    pub outputs: Option<serde_json::Value>,  // JSON Schema
}

pub struct SuggestedTool {
    pub name: String,
    pub description: String,
    pub using: String,          // node | python | shell
    pub entrypoint: String,     // path within repo
    pub version: Option<String>, // runtime version constraint
}
```

### Behavior

- If `can_infer` is false, return the assessment with empty suggestions — let the caller decide how to present this
- If `can_infer` is true, `suggested_skills` must have at least one entry
- Confidence thresholds are informational only (for display) — the user decides whether to proceed regardless of confidence
- Timeout: 30 seconds for the API call. On timeout, return an error, not a partial result.

### Error Handling

- Missing `ANTHROPIC_API_KEY` → `InferError::NoApiKey` with message suggesting how to set it
- Network error → `InferError::LlmUnavailable` with underlying error
- Rate limit → `InferError::LlmRateLimit` with retry-after if available
- Malformed LLM response → `InferError::LlmResponseParse` — retry once, then fail
- All errors must be actionable: tell the user what to do, not just what went wrong

### Prompt Design

- System prompt must emphasize: only suggest skills that genuinely make sense, `can_infer: false` is a valid and preferred answer for repos that aren't skill-shaped
- Include examples of what makes a repo skill-shaped vs not
- Constrain permissions to the known set: `filesystem.read`, `filesystem.write`, `network.external`, `process.spawn`, `network.internal`
- Request that `entrypoint_suggestion` references an actual file from the provided file tree

## Acceptance Criteria

- Given signals from a well-documented CLI tool repo, assessor returns `can_infer: true` with at least one skill suggestion including name, description, and permissions
- Given signals from a repo with only binary assets and no documentation, assessor returns `can_infer: false` with reasoning
- Given no API key set, assessor returns `InferError::NoApiKey` immediately without making a network call
- Given a valid API response with malformed JSON, assessor retries once and then returns `InferError::LlmResponseParse`
- LLM-suggested `entrypoint_suggestion` values must reference files that exist in the `file_tree`
