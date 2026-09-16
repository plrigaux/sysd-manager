use std::env;
use tool_lib::{errors::ToolError, extract_changelog, find_change_log_file, write_releases_to_xml};
use tracing::{error, info, level_filters::LevelFilter};

// use crate::lib::{errors::ToolError, find_change_log_file, read_file};

const CHANGELOG: &str = "CHANGELOG.md";

fn main() -> Result<(), ToolError> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    if let Err(e) = write_releases() {
        error!("{:?}", e);
    }
    Ok(())
}

fn write_releases() -> Result<(), ToolError> {
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
