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
