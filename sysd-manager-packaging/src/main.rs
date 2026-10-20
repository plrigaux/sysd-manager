mod build_rpm;
mod errors;
use clap::{Parser, Subcommand};
use std::env;
use sysd_build_tool::{
    errors::ToolError, extract_changelog, find_change_log_file, write_releases_to_xml,
};
use tracing::{error, info, level_filters::LevelFilter, warn};

use crate::errors::PackageError;

const CHANGELOG: &str = "CHANGELOG.md";

/// Build a simple RPM package from scratch.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build a simple RPM package from scratch.
    Rpm {},
    WriteRealese {},
}

fn main() -> Result<(), ToolError> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let args = Args::parse();

    if let Err(e) = execute_command(args) {
        error!("{:?}", e);
    }
    Ok(())
}

fn execute_command(args: Args) -> Result<(), PackageError> {
    match args.command {
        Some(Commands::Rpm {}) => build_rpm::build_rpm(),
        Some(Commands::WriteRealese {}) => write_releases(),
        _ => {
            warn!("No option selected");
            Err(PackageError::NoOption)
        }
    }
}

fn write_releases() -> Result<(), PackageError> {
    let dir = env::current_dir()?;

    let changelog_path = find_change_log_file(&dir, CHANGELOG)?;

    info!("File path {:?}", changelog_path);

    //Read Changelog
    let releases = extract_changelog(&changelog_path)?;

    let file_path = find_change_log_file(
        &dir,
        "data/metainfo/io.github.plrigaux.sysd-manager.releases.xml",
    )?;

    write_releases_to_xml(&file_path, &releases)?;
    Ok(())
}
