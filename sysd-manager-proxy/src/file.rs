use std::path::PathBuf;

use base::enums::UnitDBusLevel;
use log::warn;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::info;

pub async fn create_drop_in(
    dbus: u8,
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> zbus::fdo::Result<()> {
    let level: UnitDBusLevel = dbus.into();
    info!(
        "Creating Drop-in: unit {unit_name:?} runtime {runtime:?}, file_name {file_name:?}, bus {level:?}"
    );

    //Create dir

    let prefix = if runtime { "run" } else { "etc" };
    let path = format!("/{prefix}/systemd/system");

    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(zbus::fdo::Error::Failed(format!(
            "Directory {:?} doesn't exist",
            path.to_string_lossy()
        )));
    }

    let result = create_drop_in_io(path, unit_name, file_name, content).await;
    transform_error(result)
}

fn transform_error(result: Result<(), std::io::Error>) -> Result<(), zbus::fdo::Error> {
    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            warn!("create_drop_in {err:?}");
            match err.kind() {
                std::io::ErrorKind::PermissionDenied => Err(zbus::fdo::Error::AccessDenied(
                    format!("{:?} {err:?}", err.kind()),
                )),

                kind => Err(zbus::fdo::Error::IOError(format!("{:?}", kind))),
            }
        }
    }
}

async fn create_drop_in_io(
    path: PathBuf,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), std::io::Error> {
    let unit_drop_in_dir = path.join(format!("{unit_name}.d"));
    if !unit_drop_in_dir.exists() {
        info!("Creating dir {}", unit_drop_in_dir.display());
        fs::create_dir(&unit_drop_in_dir).await?;
    }

    let file_path = unit_drop_in_dir.join(format!("{file_name}.conf"));

    //Save content
    info!("Creating file {}", file_path.display());
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&file_path)
        .await?;

    let test_bytes = content.as_bytes();
    info!(
        "Writing content ({} bytes) to file {}",
        test_bytes.len(),
        file_path.display()
    );
    file.write_all(test_bytes).await?;
    file.flush().await?;

    let bytes_written = test_bytes.len();
    info!(
        "{bytes_written} bytes writen on File: {}",
        file_path.display()
    );

    Ok(())
}

pub async fn save(dbus: u8, file_path: &str, content: &str) -> zbus::fdo::Result<()> {
    let _level: UnitDBusLevel = dbus.into();
    let result = save_io(file_path, content).await;
    transform_error(result)
}

pub async fn save_io(file_path: &str, content: &str) -> Result<(), std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)
        .await?;

    let test_bytes = content.as_bytes();
    file.write_all(test_bytes).await?;
    file.flush().await?;

    let bytes_written = test_bytes.len();
    info!("{bytes_written} bytes writen on File: {file_path}");
    Ok(())
}
