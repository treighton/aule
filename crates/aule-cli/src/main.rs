mod commands;
mod output;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skill", about = "Aule skill ecosystem CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output JSON instead of human-readable text
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new skill package
    Init {
        /// Skill name (defaults to current directory name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Validate a skill package
    Validate {
        /// Path to the skill directory (default: current directory)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Build adapter output for a skill
    Build {
        /// Target runtime (e.g. claude-code, codex)
        #[arg(long)]
        target: Option<String>,
        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,
        /// Path to the skill directory (default: current directory)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Install a skill from a local path or git URL
    Install {
        /// Path to the skill package, or a git URL
        source: String,
        /// Git branch or tag to check out (only used with git URLs)
        #[arg(long = "ref")]
        git_ref: Option<String>,
    },
    /// Activate an installed skill for a runtime target
    Activate {
        /// Skill name
        name: String,
        /// Target runtime
        #[arg(long)]
        target: Option<String>,
    },
    /// List installed or active skills
    List {
        /// Show only installed skills
        #[arg(long)]
        installed: bool,
        /// Show only active skills
        #[arg(long)]
        active: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let json_output = cli.json;

    let result = match cli.command {
        Commands::Init { name } => commands::init::run(name, json_output),
        Commands::Validate { path } => commands::validate::run(path, json_output),
        Commands::Build {
            target,
            output,
            path,
        } => commands::build::run(target, output, path, json_output),
        Commands::Install { source, git_ref } => commands::install::run(source, git_ref, json_output),
        Commands::Activate { name, target } => commands::activate::run(name, target, json_output),
        Commands::List { installed, active } => {
            commands::list::run(installed, active, json_output)
        }
    };

    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if json_output {
                let err_json = serde_json::json!({
                    "error": e.to_string(),
                    "code": code,
                });
                println!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            } else {
                eprintln!("error: {}", e);
            }
            process::exit(code);
        }
    }
}
