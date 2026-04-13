use aule_adapter::{generate, generate_any, GenerateOptions};
use aule_schema::manifest::{parse_manifest, parse_manifest_any};
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

/// Generate and compare v0.2.0 multi-skill packages.
/// For each skill in the package, compare the generated SKILL.md and verify
/// wrapper scripts and included files are present.
fn generate_and_compare_v2(package_name: &str, skill_names: &[&str]) {
    let root = repo_root();
    let skill_src = root.join("examples").join(package_name);
    let manifest_yaml = fs::read_to_string(skill_src.join("skill.yaml"))
        .unwrap_or_else(|e| panic!("Failed to read skill.yaml for {}: {}", package_name, e));

    let manifest = parse_manifest_any(&manifest_yaml)
        .unwrap_or_else(|e| panic!("Failed to parse manifest for {}: {}", package_name, e));

    let output_dir = TempDir::new().unwrap();
    let options = GenerateOptions {
        targets: vec![],
        output_dir: Some(output_dir.path().to_path_buf()),
    };

    let _files = generate_any(&manifest, &skill_src, &options)
        .unwrap_or_else(|e| panic!("Failed to generate for {}: {}", package_name, e));

    for skill_name in skill_names {
        // Compare Claude Code SKILL.md
        let generated_claude = fs::read_to_string(
            output_dir.path().join(format!(".claude/skills/{}/SKILL.md", skill_name)),
        )
        .unwrap_or_else(|e| panic!("Generated Claude Code SKILL.md missing for {}/{}: {}", package_name, skill_name, e));

        let expected_claude = fs::read_to_string(
            root.join(format!(".claude/skills/{}/SKILL.md", skill_name)),
        )
        .unwrap_or_else(|e| panic!("Expected Claude Code SKILL.md missing for {}/{}: {}", package_name, skill_name, e));

        if generated_claude != expected_claude {
            eprintln!("\n=== CLAUDE CODE DIFF for {}/{} ===", package_name, skill_name);
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
            "Claude Code SKILL.md mismatch for {}/{}",
            package_name, skill_name
        );

        // Compare Codex SKILL.md
        let generated_codex = fs::read_to_string(
            output_dir.path().join(format!(".codex/skills/{}/SKILL.md", skill_name)),
        )
        .unwrap_or_else(|e| panic!("Generated Codex SKILL.md missing for {}/{}: {}", package_name, skill_name, e));

        let expected_codex = fs::read_to_string(
            root.join(format!(".codex/skills/{}/SKILL.md", skill_name)),
        )
        .unwrap_or_else(|e| panic!("Expected Codex SKILL.md missing for {}/{}: {}", package_name, skill_name, e));

        assert_eq!(
            generated_codex, expected_codex,
            "Codex SKILL.md mismatch for {}/{}",
            package_name, skill_name
        );

        // Verify wrapper scripts exist
        let claude_dir = output_dir.path().join(format!(".claude/skills/{}", skill_name));
        assert!(claude_dir.join("tools/generate").exists(),
            "wrapper script tools/generate missing for {}", skill_name);
        assert!(claude_dir.join("tools/run-tests").exists(),
            "wrapper script tools/run-tests missing for {}", skill_name);
        assert!(claude_dir.join("tools/report").exists(),
            "wrapper script tools/report missing for {}", skill_name);

        // Verify ## Tools section in SKILL.md
        assert!(generated_claude.contains("## Tools"),
            "SKILL.md for {} should contain ## Tools section", skill_name);

        // Verify included files are copied
        assert!(claude_dir.join("logic/tools/generate.ts").exists(),
            "logic/tools/generate.ts should be copied for {}", skill_name);
        assert!(claude_dir.join("logic/hooks/setup.sh").exists(),
            "logic/hooks/setup.sh should be copied for {}", skill_name);
    }
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

// v0.2.0 example tests

#[test]
fn api_contract_tester_matches() {
    generate_and_compare_v2(
        "api-contract-tester",
        &["contract-tester", "spec-linter"],
    );
}
