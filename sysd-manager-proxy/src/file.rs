use base::file::{create_drop_in_io, create_drop_in_path_file, save_io};
use log::warn;
use tracing::info;

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

    let result = create_drop_in_io(&file_path, content).await;

    transform_error(result)
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
