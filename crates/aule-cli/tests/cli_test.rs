use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn skill_cmd() -> Command {
    Command::cargo_bin("skill").unwrap()
}

#[test]
fn test_no_args_shows_help() {
    skill_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_init_creates_files() {
    let tmp = TempDir::new().unwrap();
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("test-skill")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized skill \"test-skill\""))
        .stdout(predicate::str::contains("skill.yaml"));

    assert!(tmp.path().join("skill.yaml").exists());
    assert!(tmp.path().join("content/skill.md").exists());
}

#[test]
fn test_init_json_output() {
    let tmp = TempDir::new().unwrap();
    skill_cmd()
        .arg("--json")
        .arg("init")
        .arg("--name")
        .arg("json-skill")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"ok\""))
        .stdout(predicate::str::contains("\"name\": \"json-skill\""));
}

#[test]
fn test_validate_valid_skill() {
    let tmp = TempDir::new().unwrap();

    // Init first
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("valid-skill")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Then validate
    skill_cmd()
        .arg("validate")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Validation passed"));
}

#[test]
fn test_validate_missing_manifest() {
    let tmp = TempDir::new().unwrap();
    skill_cmd()
        .arg("validate")
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn test_validate_json_output() {
    let tmp = TempDir::new().unwrap();

    // Init first
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("json-valid")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Validate with --json
    skill_cmd()
        .arg("--json")
        .arg("validate")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\": true"));
}

#[test]
fn test_build_produces_output() {
    let tmp = TempDir::new().unwrap();

    // Init
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("build-test")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Build
    skill_cmd()
        .arg("build")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Build complete"));

    // Verify output directories exist
    assert!(tmp.path().join(".claude/skills/build-test/SKILL.md").exists());
    assert!(tmp.path().join(".codex/skills/build-test/SKILL.md").exists());
}

#[test]
fn test_build_single_target() {
    let tmp = TempDir::new().unwrap();

    // Init
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("single-target")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Build for claude-code only
    skill_cmd()
        .arg("build")
        .arg("--target")
        .arg("claude-code")
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".claude/skills/single-target/SKILL.md").exists());
    assert!(!tmp.path().join(".codex/skills/single-target/SKILL.md").exists());
}

#[test]
fn test_build_with_output_dir() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("output");
    std::fs::create_dir_all(&out_dir).unwrap();

    // Init
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("out-test")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Build with custom output
    skill_cmd()
        .arg("build")
        .arg("--output")
        .arg(out_dir.to_str().unwrap())
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(out_dir.join(".claude/skills/out-test/SKILL.md").exists());
}

#[test]
fn test_build_json_output() {
    let tmp = TempDir::new().unwrap();

    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("json-build")
        .current_dir(tmp.path())
        .assert()
        .success();

    skill_cmd()
        .arg("--json")
        .arg("build")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"ok\""))
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_list_empty() {
    let tmp = TempDir::new().unwrap();

    // Point SKILL_HOME to empty temp dir so we don't read real cache
    skill_cmd()
        .arg("list")
        .env("SKILL_HOME", tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("No skills installed"));
}

#[test]
fn test_list_json_empty() {
    let tmp = TempDir::new().unwrap();

    skill_cmd()
        .arg("--json")
        .arg("list")
        .env("SKILL_HOME", tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skills\""));
}

#[test]
fn test_init_twice_fails() {
    let tmp = TempDir::new().unwrap();

    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("dup-skill")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second init should fail
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("dup-skill")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_install_nonexistent_git_url_gives_clear_error() {
    skill_cmd()
        .arg("install")
        .arg("https://github.com/not-a-real-org-xyz/not-a-real-repo-xyz.git")
        .assert()
        .failure()
        .stderr(predicate::str::contains("git clone failed"));
}

// --- Infer command tests ---

#[test]
fn test_infer_with_claude_skills() {
    let tmp = TempDir::new().unwrap();

    // Create a repo with .claude/skills/
    let skill_dir = tmp.path().join(".claude/skills/my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: A test skill\n---\n# My Skill\nThis is a test skill.",
    )
    .unwrap();

    skill_cmd()
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Scanning known skill locations"))
        .stdout(predicate::str::contains("my-skill"))
        .stdout(predicate::str::contains("schemaVersion"));
}

#[test]
fn test_infer_json_output() {
    let tmp = TempDir::new().unwrap();

    // Create a repo with .claude/skills/
    let skill_dir = tmp.path().join(".claude/skills/json-test");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: json-test\ndescription: JSON test\n---\n# JSON Test",
    )
    .unwrap();

    skill_cmd()
        .arg("--json")
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"stage\": \"discovery\""))
        .stdout(predicate::str::contains("\"skills_found\": 1"));
}

#[test]
fn test_infer_existing_skill_yaml() {
    let tmp = TempDir::new().unwrap();

    // Init a skill first (creates skill.yaml)
    skill_cmd()
        .arg("init")
        .arg("--name")
        .arg("existing")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Infer should detect existing skill.yaml
    skill_cmd()
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("already has a skill.yaml"));
}

#[test]
fn test_infer_existing_skill_yaml_with_force() {
    let tmp = TempDir::new().unwrap();

    // Create skill.yaml AND a .claude/skills/ directory
    let skill_dir = tmp.path().join(".claude/skills/forced");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: forced\ndescription: Forced re-infer\n---\n# Forced",
    )
    .unwrap();
    std::fs::write(tmp.path().join("skill.yaml"), "schemaVersion: '0.1.0'\nname: old\ndescription: Old\nversion: '1.0.0'\ncontent:\n  skill: content/skill.md\ncontract:\n  version: '1.0.0'\n  inputs: prompt\n  outputs: prompt\n  permissions: []\n").unwrap();

    // Infer with --force should re-infer
    skill_cmd()
        .arg("infer")
        .arg("--force")
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Scanning known skill locations"))
        .stdout(predicate::str::contains("forced"));
}

#[test]
fn test_infer_with_plugin_json() {
    let tmp = TempDir::new().unwrap();

    let plugin = serde_json::json!({
        "name": "plugin-test",
        "description": "A test plugin",
        "skills": [{
            "name": "plugin-skill",
            "description": "Plugin skill desc",
            "entrypoint": "skills/main.md"
        }]
    });
    std::fs::write(
        tmp.path().join("plugin.json"),
        serde_json::to_string_pretty(&plugin).unwrap(),
    )
    .unwrap();

    skill_cmd()
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("plugin-skill"));
}

#[test]
fn test_infer_with_skill_md() {
    let tmp = TempDir::new().unwrap();

    std::fs::write(
        tmp.path().join("SKILL.md"),
        "---\nname: standalone\ndescription: Standalone skill\n---\n# Standalone\nContent here.",
    )
    .unwrap();

    skill_cmd()
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("standalone"));
}

#[test]
fn test_infer_empty_repo_no_api_key() {
    let tmp = TempDir::new().unwrap();

    // Empty repo, no skill artifacts, no API key → should fail gracefully
    skill_cmd()
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .env_remove("ANTHROPIC_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ANTHROPIC_API_KEY"));
}

#[test]
fn test_infer_output_flag() {
    let tmp = TempDir::new().unwrap();
    let out_path = tmp.path().join("my-manifest.yaml");

    let skill_dir = tmp.path().join(".claude/skills/out-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: out-skill\ndescription: Output test\n---\n# Out",
    )
    .unwrap();

    skill_cmd()
        .arg("infer")
        .arg(tmp.path().to_str().unwrap())
        .arg("--output")
        .arg(out_path.to_str().unwrap())
        .assert()
        .success();

    assert!(out_path.exists());
    let content = std::fs::read_to_string(&out_path).unwrap();
    assert!(content.contains("schemaVersion"));
    assert!(content.contains("out-skill"));
}

#[test]
fn test_install_infer_with_skills() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();

    // Create a repo with .claude/skills/ but no skill.yaml
    let skill_dir = tmp.path().join(".claude/skills/install-test");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: install-test\ndescription: Install test\n---\n# Install Test",
    )
    .unwrap();

    skill_cmd()
        .arg("install")
        .arg(tmp.path().to_str().unwrap())
        .arg("--infer")
        .env("SKILL_HOME", cache_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("No skill.yaml found. Running inference"))
        .stdout(predicate::str::contains("Installed"));
}
