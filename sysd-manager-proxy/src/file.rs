use base::file::SysdBaseError;
use base::file::{create_drop_in_io, create_drop_in_path_file, save_io};
use tracing::info;
use tracing::warn;

pub async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> zbus::fdo::Result<()> {
    info!(
        "Creating Drop-in: unit {unit_name:?} runtime {runtime:?}, file_name {file_name:?} , content {} bytes",
        content.len()
    );

    let file_path = create_drop_in_path_file(unit_name, runtime, false, file_name)
        .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;

    create_drop_in_io(&file_path, content).await.map_err(|err| {
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
            _ => FdoError(zbus::fdo::Error::Failed("Internal Error".to_string())),
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
