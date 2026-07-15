use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use tempfile::NamedTempFile;

use envish::{ConfigFile, list_profiles, profile_env};

#[derive(Parser)]
#[command(
    name = "envish",
    version,
    about = "Load named environment profiles into a nested shell",
    long_about = None,
    arg_required_else_help = true,
    subcommand_required = true
)]
struct Cli {
    /// Path to the TOML config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a shell with the named profile's environment variables
    Load {
        /// Profile name (a top-level table in the config)
        name: String,
    },
    /// Open the config file in $VISUAL / $EDITOR
    Edit,
    /// List available profile names
    List,
    /// Print the path to the config file
    Path,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let config_file = resolve_config(cli.config)?;
    config_file.ensure_exists()?;

    match cli.command {
        Commands::Load { name } => {
            let config = config_file.read()?;
            let vars = profile_env(&config, &name)?;
            print_load_summary(&name, &vars);
            let shell = env::var("SHELL").unwrap_or_else(|_| default_shell());
            let mut cmd = Command::new(&shell);
            for (key, value) in &vars {
                cmd.env(key, value);
            }
            let status = cmd
                .status()
                .with_context(|| format!("failed to start shell `{shell}`"))?;
            let code = status.code().unwrap_or(1) as u8;
            return Ok(ExitCode::from(code));
        }
        Commands::Edit => {
            let original = config_file.read()?;
            let text = run_editor(&original)?;
            toml::from_str::<toml::Value>(&text).context("edited config is not valid TOML")?;
            config_file.write(&text)?;
        }
        Commands::List => {
            let config = config_file.read()?;
            for name in list_profiles(&config)? {
                println!("{name}");
            }
        }
        Commands::Path => {
            println!("{}", config_file.path().display());
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn resolve_config(config: Option<PathBuf>) -> Result<ConfigFile> {
    if let Some(path) = config {
        return Ok(ConfigFile::new(path));
    }

    let project_dirs = ProjectDirs::from("", "Enkidulan", "envish")
        .context("could not determine a config directory for envish")?;
    let config_dir = project_dirs.config_dir();
    fs::create_dir_all(config_dir)
        .with_context(|| format!("failed to create config directory {}", config_dir.display()))?;
    Ok(ConfigFile::new(config_dir.join("config.toml")))
}

fn default_shell() -> String {
    if cfg!(windows) {
        "cmd".into()
    } else {
        "/bin/sh".into()
    }
}

fn print_load_summary(profile: &str, vars: &std::collections::BTreeMap<String, String>) {
    eprintln!("Setting environment variables from profile `{profile}`");
    if vars.is_empty() {
        eprintln!("  all: (none)");
        return;
    }

    let all: Vec<&str> = vars.keys().map(String::as_str).collect();
    let mut overwritten = Vec::new();
    let mut unchanged = Vec::new();

    for (key, value) in vars {
        match env::var(key) {
            Ok(previous) if previous != *value => overwritten.push(key.as_str()),
            Ok(_) => unchanged.push(key.as_str()),
            Err(_) => {}
        }
    }

    eprintln!("  all: {}", all.join(", "));
    if !overwritten.is_empty() {
        eprintln!("  overwritten: {}", overwritten.join(", "));
    }
    if !unchanged.is_empty() {
        eprintln!("  unchanged: {}", unchanged.join(", "));
    }
}

fn run_editor(initial: &str) -> Result<String> {
    let mut file = NamedTempFile::new().context("failed to create temporary file for editor")?;
    file.write_all(initial.as_bytes())
        .context("failed to write temporary file for editor")?;
    file.flush()
        .context("failed to flush temporary file for editor")?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".into()
            } else {
                "vi".into()
            }
        });

    let status = Command::new(&editor)
        .arg(file.path())
        .status()
        .with_context(|| format!("failed to start editor `{editor}`"))?;

    if !status.success() {
        bail!("editor `{editor}` exited with an error");
    }

    fs::read_to_string(file.path()).context("failed to read edited config from temporary file")
}
