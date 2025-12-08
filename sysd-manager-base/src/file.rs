use std::path::Path;
use std::{error::Error, path::PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::info;

pub fn create_drop_in_path_dir(
    unit_name: &str,
    runtime: bool,
    user: bool,
) -> Result<String, Box<dyn Error + 'static>> {
    let path = match (runtime, user) {
        (true, false) => format!("/run/systemd/system/{}.d", unit_name),
        (false, false) => format!("/etc/systemd/system/{}.d", unit_name),
        (true, true) => {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));

            format!("{runtime_dir}/systemd/user/{}.d", unit_name)
        }
        (false, true) => {
            let home_dir = std::env::home_dir().ok_or(Box::<dyn Error>::from(
                "No HOME found to create drop-in".to_string(),
            ))?;
            format!(
                "{}/.config/systemd/user/{}.d",
                home_dir.display(),
                unit_name
            )
        }
    };
    Ok(path)
}

pub fn create_drop_in_path_file(
    unit_name: &str,
    runtime: bool,
    user: bool,
    file_name: &str,
) -> Result<String, Box<dyn Error + 'static>> {
    let path_dir = create_drop_in_path_dir(unit_name, runtime, user)?;

    let path = format!("{path_dir}/{file_name}.conf");
    Ok(path)
}

pub async fn create_drop_in_io(
    file_path_str: &str,
    content: &str,
    user: bool,
) -> Result<(), std::io::Error> {
    if file_path_str.contains("../") {
        let err = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            r#"The "../" patern is not supported""#,
        );

        return Err(err);
    }

    let file_path = PathBuf::from(file_path_str);

    let Some(unit_drop_in_dir) = file_path.parent() else {
        let err = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Parent dir of file {:?} is invalid", file_path_str),
        );

        return Err(err);
    };

    if !unit_drop_in_dir.exists() {
        info!("Creating dir {}", unit_drop_in_dir.display());
        if user {
            fs::create_dir_all(&unit_drop_in_dir).await?;
        } else {
            fs::create_dir(&unit_drop_in_dir).await?;
        }
    }

    //Save content
    info!("Creating file {}", file_path.display());
    let bytes_written = save_io(&file_path, true, content).await?;

    info!(
        "{bytes_written} bytes writen on File {}",
        file_path.to_string_lossy()
    );
    Ok(())
}

pub async fn save_io(
    file_path: impl AsRef<Path>,
    create: bool,
    content: &str,
) -> Result<u64, std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(create)
        .open(file_path)
        .await?;

    let test_bytes = content.as_bytes();

    file.write_all(test_bytes).await?;
    file.flush().await?;

    let bytes_written = test_bytes.len();

    Ok(bytes_written as u64)
}
