use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum RiskTier {
    None,
    Low,
    Medium,
    High,
}

pub struct PermissionDef {
    pub permission: &'static str,
    pub category: &'static str,
    pub scope: &'static str,
    pub risk_tier: RiskTier,
}

pub static V0_VOCABULARY: &[PermissionDef] = &[
    PermissionDef {
        permission: "filesystem.read",
        category: "filesystem",
        scope: "read",
        risk_tier: RiskTier::Low,
    },
    PermissionDef {
        permission: "filesystem.write",
        category: "filesystem",
        scope: "write",
        risk_tier: RiskTier::High,
    },
    PermissionDef {
        permission: "filesystem.write.workspace",
        category: "filesystem",
        scope: "write.workspace",
        risk_tier: RiskTier::Medium,
    },
    PermissionDef {
        permission: "network.external",
        category: "network",
        scope: "external",
        risk_tier: RiskTier::Medium,
    },
    PermissionDef {
        permission: "network.external.specific",
        category: "network",
        scope: "external.specific",
        risk_tier: RiskTier::Medium,
    },
    PermissionDef {
        permission: "process.spawn",
        category: "process",
        scope: "spawn",
        risk_tier: RiskTier::High,
    },
    PermissionDef {
        permission: "process.spawn.specific",
        category: "process",
        scope: "spawn.specific",
        risk_tier: RiskTier::Medium,
    },
    PermissionDef {
        permission: "runtime.context",
        category: "runtime",
        scope: "context",
        risk_tier: RiskTier::Low,
    },
];

pub struct PermissionCheck {
    pub valid_format: bool,
    pub known: bool,
    pub risk_tier: Option<RiskTier>,
}

pub fn validate_permission(perm: &str) -> PermissionCheck {
    // Format check: lowercase alphanumeric + dots, no leading/trailing dots
    let valid_format = !perm.is_empty()
        && perm
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.')
        && !perm.starts_with('.')
        && !perm.ends_with('.')
        && !perm.contains("..");

    if !valid_format {
        return PermissionCheck {
            valid_format: false,
            known: false,
            risk_tier: None,
        };
    }

    // Check against vocabulary
    if let Some(def) = V0_VOCABULARY.iter().find(|d| d.permission == perm) {
        PermissionCheck {
            valid_format: true,
            known: true,
            risk_tier: Some(def.risk_tier.clone()),
        }
    } else {
        PermissionCheck {
            valid_format: true,
            known: false,
            risk_tier: None,
        }
    }
}

pub fn max_risk_tier(permissions: &[String]) -> RiskTier {
    permissions
        .iter()
        .filter_map(|p| validate_permission(p).risk_tier)
        .max()
        .unwrap_or(RiskTier::None)
}

/// Checks if `granted` permission implies `required` permission.
/// A broader scope implies all narrower scopes in the same category.
/// e.g., `filesystem.write` implies `filesystem.write.workspace`
pub fn implies_permission(granted: &str, required: &str) -> bool {
    if granted == required {
        return true;
    }
    // granted implies required if required starts with granted + "."
    required.starts_with(granted) && required.as_bytes().get(granted.len()) == Some(&b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_known_permission() {
        let check = validate_permission("filesystem.read");
        assert!(check.valid_format);
        assert!(check.known);
        assert_eq!(check.risk_tier, Some(RiskTier::Low));
    }

    #[test]
    fn valid_unknown_permission() {
        let check = validate_permission("gpu.compute");
        assert!(check.valid_format);
        assert!(!check.known);
        assert_eq!(check.risk_tier, None);
    }

    #[test]
    fn invalid_format() {
        let check = validate_permission("FileSystem Read");
        assert!(!check.valid_format);
    }

    #[test]
    fn risk_tier_computation() {
        let perms = vec![
            "filesystem.read".to_string(),
            "network.external".to_string(),
            "filesystem.write".to_string(),
        ];
        assert_eq!(max_risk_tier(&perms), RiskTier::High);
    }

    #[test]
    fn risk_tier_empty() {
        assert_eq!(max_risk_tier(&[]), RiskTier::None);
    }

    #[test]
    fn hierarchy_implication() {
        assert!(implies_permission("filesystem.write", "filesystem.write.workspace"));
        assert!(!implies_permission("filesystem.write.workspace", "filesystem.write"));
        assert!(implies_permission("filesystem.read", "filesystem.read"));
    }

    #[test]
    fn no_false_implication() {
        // "filesystem.write" should NOT imply "filesystem.writespecial" (no dot separator)
        assert!(!implies_permission("filesystem.write", "filesystem.writespecial"));
    }
}
