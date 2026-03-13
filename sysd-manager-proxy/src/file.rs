use base::file::SysdBaseError;
use base::file::{create_drop_in_io, save_io};
use tracing::info;
use tracing::warn;

pub async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_path: &str,
    content: &str,
) -> zbus::fdo::Result<()> {
    info!(
        "Creating Drop-in: unit {unit_name:?} runtime {runtime:?}, file_path {file_path:?} , content {} bytes",
        content.len()
    );

    create_drop_in_io(file_path, content, false)
        .await
        .map_err(|err| {
            let fdo: FdoError = err.into();
            fdo.0
        })
}

struct FdoError(zbus::fdo::Error);

impl From<SysdBaseError> for FdoError {
    fn from(value: SysdBaseError) -> Self {
        warn!("{value:?}");
        match value {
            SysdBaseError::Custom(s) => FdoError(zbus::fdo::Error::Failed(s)),
            SysdBaseError::IoError(err) => match err.kind() {
                std::io::ErrorKind::PermissionDenied => FdoError(zbus::fdo::Error::AccessDenied(
                    format!("{:?} {err:?}", err.kind()),
                )),

                std::io::ErrorKind::InvalidData => FdoError(zbus::fdo::Error::InvalidArgs(
                    format!("{:?} {err:?}", err.kind()),
                )),

                kind => FdoError(zbus::fdo::Error::IOError(format!("{:?}", kind))),
            },
            SysdBaseError::NotAuthorizedAuthentificationDismissed => {
                FdoError(zbus::fdo::Error::AuthFailed("Not Authentified".to_string()))
            }
            SysdBaseError::NotAuthorized => {
                FdoError(zbus::fdo::Error::AccessDenied("Not Auhorised".to_string()))
            }
            SysdBaseError::CmdNoFreedesktopFlatpakPermission => FdoError(
                zbus::fdo::Error::NotSupported("Flatpak Permission".to_string()),
            ),
            SysdBaseError::CommandCallError(_, _, _, _) => {
                FdoError(zbus::fdo::Error::Failed("Internal issue".to_string()))
            }
            SysdBaseError::Tokio(_) => {
                FdoError(zbus::fdo::Error::Failed("Internal issue".to_string()))
            }
            SysdBaseError::InvalidPath(msg) => FdoError(zbus::fdo::Error::AccessDenied(msg)),
        }
    }
}

fn transform_error<T>(result: Result<T, std::io::Error>) -> Result<T, zbus::fdo::Error> {
    match result {
        Ok(a) => Ok(a),
        Err(err) => {
            warn!("create_drop_in {err:?}");
            match err.kind() {
                std::io::ErrorKind::PermissionDenied => Err(zbus::fdo::Error::AccessDenied(
                    format!("{:?} {err:?}", err.kind()),
                )),

                std::io::ErrorKind::InvalidData => Err(zbus::fdo::Error::InvalidArgs(format!(
                    "{:?} {err:?}",
                    err.kind()
                ))),

                kind => Err(zbus::fdo::Error::IOError(format!("{:?}", kind))),
            }
        }
    }
}

pub async fn save(file_path: &str, content: &str) -> zbus::fdo::Result<u64> {
    let result = save_io(file_path, false, content).await;
    transform_error(result)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn test_parent_dir() {
        let p = PathBuf::from("/home/plr/../.config");

        println!("p {} {}", p.display(), p.is_absolute());

        println!("p {}", p.canonicalize().unwrap().display())
    }
}
