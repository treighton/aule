use serde::{Deserialize, Serialize};

use crate::{CacheError, CacheManager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyConfig {
    pub allow: Option<Vec<String>>,
    pub block: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PublisherInfo {
    pub github_username: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub default_targets: Option<Vec<String>>,
    pub policy: Option<PolicyConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<PublisherInfo>,
}

impl UserConfig {
    /// Loads user configuration from `config.json`. Returns default if missing.
    pub fn load(mgr: &CacheManager) -> Result<Self, CacheError> {
        let path = mgr.root().join("config.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let config: UserConfig = serde_json::from_str(&data)?;
        Ok(config)
    }

    /// Saves user configuration to `config.json`.
    pub fn save(&self, mgr: &CacheManager) -> Result<(), CacheError> {
        let path = mgr.root().join("config.json");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_config_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());

        let config = UserConfig::load(&mgr).unwrap();
        assert!(config.default_targets.is_none());
        assert!(config.policy.is_none());
    }

    #[test]
    fn test_save_and_load_config() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = CacheManager::with_root(tmp.path());

        let config = UserConfig {
            default_targets: Some(vec!["claude-code".into(), "cursor".into()]),
            policy: Some(PolicyConfig {
                allow: Some(vec!["trusted-publisher/*".into()]),
                block: Some(vec!["malicious-pkg".into()]),
            }),
            registry_url: None,
            auth_token: None,
            publisher: None,
        };
        config.save(&mgr).unwrap();

        let loaded = UserConfig::load(&mgr).unwrap();
        assert_eq!(
            loaded.default_targets.as_ref().unwrap().len(),
            2
        );
        assert_eq!(
            loaded.policy.as_ref().unwrap().allow.as_ref().unwrap()[0],
            "trusted-publisher/*"
        );
        assert_eq!(
            loaded.policy.as_ref().unwrap().block.as_ref().unwrap()[0],
            "malicious-pkg"
        );
    }
}
