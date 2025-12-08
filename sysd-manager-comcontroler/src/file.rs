use base::file::{create_drop_in_io, create_drop_in_path_file};

use crate::errors::SystemdErrors;

pub(crate) async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), SystemdErrors> {
    let file_path = create_drop_in_path_file(unit_name, runtime, true, file_name)?;

    create_drop_in_io(&file_path, content).await?;

    Ok(())
}
