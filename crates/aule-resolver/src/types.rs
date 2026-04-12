use std::path::PathBuf;

use aule_schema::permissions::RiskTier;
use serde::{Deserialize, Serialize};

/// A request to resolve a skill for installation.
#[derive(Debug, Clone)]
pub struct ResolveRequest {
    pub skill_name: String,
    pub version_constraint: Option<String>,
    pub runtime_target: Option<String>,
    pub local_path: Option<PathBuf>,
}

/// A fully resolved plan describing what to install and how.
#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub skill_name: String,
    pub resolved_version: String,
    pub contract_version: String,
    pub adapters: Vec<ResolvedAdapter>,
    pub artifact_source: ArtifactSource,
    pub permissions: Vec<String>,
    pub risk_tier: RiskTier,
}

/// An adapter entry in a resolved install plan.
#[derive(Debug, Clone)]
pub struct ResolvedAdapter {
    pub runtime_id: String,
    pub enabled: bool,
}

/// Where to obtain the skill artifact from.
#[derive(Debug, Clone)]
pub enum ArtifactSource {
    LocalPath(PathBuf),
    Cache(String),
    /// Cloned from a git repository into a temporary directory.
    Git { url: String, temp_dir: PathBuf },
}

/// An entry in the local cache metadata index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndexEntry {
    pub name: String,
    pub version: String,
    pub contract_version: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub adapters: Vec<CacheAdapterEntry>,
    pub identity_hash: String,
}

/// Adapter info stored in the cache index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheAdapterEntry {
    pub runtime_id: String,
    pub enabled: bool,
}
