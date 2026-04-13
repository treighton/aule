use aule_adapter::{generate, GenerateOptions};
use aule_schema::manifest::parse_manifest;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn generate_and_compare(skill_name: &str) {
    let root = repo_root();
    let skill_src = root.join("examples").join(skill_name);
    let manifest_yaml = fs::read_to_string(skill_src.join("skill.yaml"))
        .unwrap_or_else(|e| panic!("Failed to read skill.yaml for {}: {}", skill_name, e));

    let manifest = parse_manifest(&manifest_yaml)
        .unwrap_or_else(|e| panic!("Failed to parse manifest for {}: {}", skill_name, e));

    let output_dir = TempDir::new().unwrap();
    let options = GenerateOptions {
        targets: vec![],
        output_dir: Some(output_dir.path().to_path_buf()),
    };

    let _files = generate(&manifest, &skill_src, &options)
        .unwrap_or_else(|e| panic!("Failed to generate for {}: {}", skill_name, e));

    // Compare Claude Code SKILL.md
    let generated_claude = fs::read_to_string(
        output_dir.path().join(format!(".claude/skills/{}/SKILL.md", skill_name)),
    )
    .unwrap_or_else(|e| panic!("Generated Claude Code file missing for {}: {}", skill_name, e));

    let expected_claude = fs::read_to_string(
        root.join(format!(".claude/skills/{}/SKILL.md", skill_name)),
    )
    .unwrap_or_else(|e| panic!("Expected Claude Code file missing for {}: {}", skill_name, e));

    // Compare Codex SKILL.md
    let generated_codex = fs::read_to_string(
        output_dir.path().join(format!(".codex/skills/{}/SKILL.md", skill_name)),
    )
    .unwrap_or_else(|e| panic!("Generated Codex file missing for {}: {}", skill_name, e));

    let expected_codex = fs::read_to_string(
        root.join(format!(".codex/skills/{}/SKILL.md", skill_name)),
    )
    .unwrap_or_else(|e| panic!("Expected Codex file missing for {}: {}", skill_name, e));

    // Print diff if they don't match
    if generated_claude != expected_claude {
        eprintln!("\n=== CLAUDE CODE DIFF for {} ===", skill_name);
        let gen_lines: Vec<&str> = generated_claude.lines().collect();
        let exp_lines: Vec<&str> = expected_claude.lines().collect();
        for (i, (g, e)) in gen_lines.iter().zip(exp_lines.iter()).enumerate() {
            if g != e {
                eprintln!("  Line {}: generated: {:?}", i + 1, g);
                eprintln!("  Line {}: expected:  {:?}", i + 1, e);
            }
        }
        if gen_lines.len() != exp_lines.len() {
            eprintln!(
                "  Line count: generated={}, expected={}",
                gen_lines.len(),
                exp_lines.len()
            );
        }
    }

    assert_eq!(
        generated_claude, expected_claude,
        "Claude Code SKILL.md mismatch for {}",
        skill_name
    );
    assert_eq!(
        generated_codex, expected_codex,
        "Codex SKILL.md mismatch for {}",
        skill_name
    );
}

#[test]
fn skill_init_matches() {
    generate_and_compare("skill-init");
}

#[test]
fn skill_validate_matches() {
    generate_and_compare("skill-validate");
}

#[test]
fn skill_build_matches() {
    generate_and_compare("skill-build");
}

#[test]
fn skill_publish_matches() {
    generate_and_compare("skill-publish");
}

#[test]
fn skill_develop_matches() {
    generate_and_compare("skill-develop");
}

#[test]
fn skill_scout_matches() {
    generate_and_compare("skill-scout");
}
