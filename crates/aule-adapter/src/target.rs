/// Defines a runtime target for adapter generation.
///
/// Each target describes where skill files and command files should be placed,
/// what frontmatter fields to include, and whether commands are supported.
#[derive(Debug, Clone)]
pub struct RuntimeTarget {
    pub id: String,
    /// Template path for skill files. Supports `{name}` placeholder.
    pub skill_path_template: String,
    /// Template path for command files. Supports `{namespace}` and `{command_name}`.
    /// None if the target doesn't support commands.
    pub command_path_template: Option<String>,
    pub supports_commands: bool,
}

impl RuntimeTarget {
    pub fn claude_code() -> Self {
        Self {
            id: "claude-code".to_string(),
            skill_path_template: ".claude/skills/{name}/SKILL.md".to_string(),
            command_path_template: Some(".claude/commands/{namespace}/{command_name}.md".to_string()),
            supports_commands: true,
        }
    }

    pub fn codex() -> Self {
        Self {
            id: "codex".to_string(),
            skill_path_template: ".codex/skills/{name}/SKILL.md".to_string(),
            command_path_template: None,
            supports_commands: false,
        }
    }

    pub fn pi() -> Self {
        Self {
            id: "pi".to_string(),
            skill_path_template: "~/.pi/agent/skills/{name}/SKILL.md".to_string(),
            command_path_template: None,
            supports_commands: false,
        }
    }

    pub fn by_id(id: &str) -> Option<Self> {
        match id {
            "claude-code" => Some(Self::claude_code()),
            "codex" => Some(Self::codex()),
            "pi" => Some(Self::pi()),
            _ => None,
        }
    }

    pub fn all_known() -> Vec<Self> {
        vec![Self::claude_code(), Self::codex(), Self::pi()]
    }

    pub fn skill_path(&self, name: &str) -> String {
        self.skill_path_template.replace("{name}", name)
    }

    pub fn command_path(&self, namespace: &str, command_name: &str) -> Option<String> {
        self.command_path_template.as_ref().map(|t| {
            t.replace("{namespace}", namespace)
                .replace("{command_name}", command_name)
        })
    }
}
