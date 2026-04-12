use serde::{Deserialize, Serialize};

use crate::commands::CliError;

pub const DEFAULT_REGISTRY_URL: &str = "https://aule.dev";
const USER_AGENT: &str = concat!("skill-cli/", env!("CARGO_PKG_VERSION"));

/// Resolve the registry URL from (in priority order): explicit flag, env var, config, default.
pub fn resolve_registry_url(flag: Option<&str>, config_url: Option<&str>) -> String {
    if let Some(url) = flag {
        return url.trim_end_matches('/').to_string();
    }
    if let Ok(url) = std::env::var("SKILL_REGISTRY_URL") {
        return url.trim_end_matches('/').to_string();
    }
    if let Some(url) = config_url {
        return url.trim_end_matches('/').to_string();
    }
    DEFAULT_REGISTRY_URL.to_string()
}

// ---- API types ----

#[derive(Debug, Deserialize)]
pub struct DeviceStartResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
}

#[derive(Debug, Deserialize)]
pub struct DevicePollResponse {
    pub status: String,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub publisher: Option<DevicePollPublisher>,
}

#[derive(Debug, Deserialize)]
pub struct DevicePollPublisher {
    pub github_username: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterSkillRequest {
    pub repo_url: String,
    pub skill_path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterSkillResponse {
    pub status: String,
    #[serde(default)]
    pub skill_id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    #[serde(default)]
    pub total: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ResolveSkillRequest {
    pub skill: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveSkillResponse {
    pub repo_url: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub skill_path: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    #[allow(dead_code)]
    code: String,
    message: String,
}

// ---- Client ----

pub struct RegistryClient {
    base_url: String,
    auth_token: Option<String>,
}

impl RegistryClient {
    pub fn new(base_url: String, auth_token: Option<String>) -> Self {
        Self {
            base_url,
            auth_token,
        }
    }

    fn agent(&self) -> ureq::Agent {
        ureq::AgentBuilder::new()
            .user_agent(USER_AGENT)
            .build()
    }

    fn handle_error_response(resp: ureq::Response) -> CliError {
        let status = resp.status();
        match resp.into_string() {
            Ok(body) => {
                if let Ok(err_body) = serde_json::from_str::<ApiErrorBody>(&body) {
                    CliError::User(format!(
                        "registry error ({}): {}",
                        status, err_body.error.message
                    ))
                } else {
                    CliError::User(format!("registry error ({}): {}", status, body))
                }
            }
            Err(_) => CliError::User(format!("registry error ({})", status)),
        }
    }

    pub fn device_start(&self) -> Result<DeviceStartResponse, CliError> {
        let url = format!("{}/api/v1/auth/device/start", self.base_url);
        let resp = self
            .agent()
            .post(&url)
            .send_string("")
            .map_err(|e| match e {
                ureq::Error::Status(_code, resp) => Self::handle_error_response(resp),
                ureq::Error::Transport(t) => {
                    CliError::User(format!("could not reach registry at {}: {}", self.base_url, t))
                }
            })?;
        let body: DeviceStartResponse = resp
            .into_json()
            .map_err(|e| CliError::Internal(format!("failed to parse device start response: {}", e)))?;
        Ok(body)
    }

    pub fn device_poll(&self, device_code: &str) -> Result<DevicePollResponse, CliError> {
        let url = format!("{}/api/v1/auth/device/poll", self.base_url);
        let resp = self
            .agent()
            .post(&url)
            .send_json(serde_json::json!({ "device_code": device_code }))
            .map_err(|e| match e {
                ureq::Error::Status(_code, resp) => Self::handle_error_response(resp),
                ureq::Error::Transport(t) => {
                    CliError::User(format!("poll request failed: {}", t))
                }
            })?;
        let body: DevicePollResponse = resp
            .into_json()
            .map_err(|e| CliError::Internal(format!("failed to parse poll response: {}", e)))?;
        Ok(body)
    }

    pub fn register_skill(&self, req: &RegisterSkillRequest) -> Result<RegisterSkillResponse, CliError> {
        let url = format!("{}/api/v1/skills/register", self.base_url);
        let token = self
            .auth_token
            .as_deref()
            .ok_or_else(|| CliError::User("not logged in — run `skill login` first".to_string()))?;

        let resp = self
            .agent()
            .post(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .send_json(serde_json::to_value(req).unwrap())
            .map_err(|e| match e {
                ureq::Error::Status(_code, resp) => Self::handle_error_response(resp),
                ureq::Error::Transport(t) => {
                    CliError::User(format!("publish request failed: {}", t))
                }
            })?;
        let body: RegisterSkillResponse = resp
            .into_json()
            .map_err(|e| CliError::Internal(format!("failed to parse register response: {}", e)))?;
        Ok(body)
    }

    pub fn search(
        &self,
        query: &str,
        runtime: Option<&str>,
        limit: Option<u32>,
    ) -> Result<SearchResponse, CliError> {
        let mut url = format!(
            "{}/api/v1/search?q={}",
            self.base_url,
            urlencod(query)
        );
        if let Some(rt) = runtime {
            url.push_str(&format!("&runtime={}", urlencod(rt)));
        }
        if let Some(lim) = limit {
            url.push_str(&format!("&limit={}", lim));
        }

        let resp = self
            .agent()
            .get(&url)
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(_code, resp) => Self::handle_error_response(resp),
                ureq::Error::Transport(t) => {
                    CliError::User(format!("search request failed: {}", t))
                }
            })?;
        let body: SearchResponse = resp
            .into_json()
            .map_err(|e| CliError::Internal(format!("failed to parse search response: {}", e)))?;
        Ok(body)
    }

    pub fn resolve_skill(&self, req: &ResolveSkillRequest) -> Result<ResolveSkillResponse, CliError> {
        let url = format!("{}/api/v1/resolve", self.base_url);
        let resp = self
            .agent()
            .post(&url)
            .send_json(serde_json::to_value(req).unwrap())
            .map_err(|e| match e {
                ureq::Error::Status(_code, resp) => Self::handle_error_response(resp),
                ureq::Error::Transport(t) => {
                    CliError::User(format!("resolve request failed: {}", t))
                }
            })?;
        let body: ResolveSkillResponse = resp
            .into_json()
            .map_err(|e| CliError::Internal(format!("failed to parse resolve response: {}", e)))?;
        Ok(body)
    }
}

/// Minimal percent-encoding for query parameters.
fn urlencod(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}
