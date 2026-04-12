mod commands;
mod output;
mod registry;

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
    /// Install a skill from a local path, git URL, or registry (@owner/name)
    Install {
        /// Path, git URL, or @owner/name registry identifier
        source: String,
        /// Git branch or tag to check out (only used with git URLs)
        #[arg(long = "ref")]
        git_ref: Option<String>,
        /// Version constraint (only used with registry installs)
        #[arg(long)]
        version: Option<String>,
        /// Target runtime to activate after install (only used with registry installs)
        #[arg(long)]
        target: Option<String>,
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
    /// Authenticate with the skill registry
    Login {
        /// Registry URL (default: https://aule.dev)
        #[arg(long)]
        registry: Option<String>,
    },
    /// Remove authentication token
    Logout,
    /// Publish a skill to the registry
    Publish {
        /// Path to the skill directory (default: current directory)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Git ref to publish (default: current branch)
        #[arg(long = "ref")]
        git_ref: Option<String>,
    },
    /// Search the skill registry
    Search {
        /// Search query
        query: String,
        /// Filter by runtime target
        #[arg(long)]
        runtime: Option<String>,
        /// Maximum number of results
        #[arg(long)]
        limit: Option<u32>,
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
        Commands::Install {
            source,
            git_ref,
            version,
            target,
        } => commands::install::run(source, git_ref, version, target, json_output),
        Commands::Activate { name, target } => commands::activate::run(name, target, json_output),
        Commands::List { installed, active } => {
            commands::list::run(installed, active, json_output)
        }
        Commands::Login { registry } => commands::login::run(registry, json_output),
        Commands::Logout => commands::logout::run(json_output),
        Commands::Publish { path, git_ref } => {
            commands::publish::run(path, git_ref, json_output)
        }
        Commands::Search {
            query,
            runtime,
            limit,
        } => commands::search::run(query, runtime, limit, json_output),
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
