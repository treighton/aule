use std::path::PathBuf;

/// Expand a `~/` prefix to the user's home directory.
///
/// Returns the path unchanged if it doesn't start with `~/`.
pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Returns true if the path starts with `~/`, indicating a home-directory location.
pub fn is_home_path(path: &str) -> bool {
    path.starts_with("~/")
}

/// Resolve an output path for writing a generated file.
///
/// When `explicit_output_dir` is true (user passed `--output`), home paths are
/// stripped of `~/` and joined with `output_root` like any other path, so that
/// all output lands in the chosen directory.
///
/// When `explicit_output_dir` is false (defaulting to base_path), home paths
/// are expanded to the real home directory.
pub fn resolve_output_path(
    output_root: &std::path::Path,
    relative_path: &str,
    explicit_output_dir: bool,
) -> PathBuf {
    if is_home_path(relative_path) {
        if explicit_output_dir {
            // Strip ~/ and treat as relative under the output root
            let stripped = relative_path.strip_prefix("~/").unwrap_or(relative_path);
            output_root.join(stripped)
        } else {
            expand_home(relative_path)
        }
    } else {
        output_root.join(relative_path)
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_path() {
        let result = expand_home("~/.pi/agent/skills/my-skill/SKILL.md");
        let home = home_dir().unwrap();
        assert_eq!(result, home.join(".pi/agent/skills/my-skill/SKILL.md"));
    }

    #[test]
    fn non_tilde_path_unchanged() {
        let result = expand_home(".claude/skills/my-skill/SKILL.md");
        assert_eq!(result, PathBuf::from(".claude/skills/my-skill/SKILL.md"));
    }

    #[test]
    fn is_home_path_detects_tilde() {
        assert!(is_home_path("~/.pi/agent/skills/foo/SKILL.md"));
        assert!(!is_home_path(".claude/skills/foo/SKILL.md"));
        assert!(!is_home_path("relative/path"));
    }
}
