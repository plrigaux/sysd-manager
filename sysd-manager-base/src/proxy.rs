use serde::{Deserialize, Serialize};
use zvariant::Type;

#[derive(Debug, Type, Serialize, Deserialize)]
pub struct DisEnAbleUnitFiles {
    pub change_type: String,
    pub file_name: String,
    pub destination: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
pub struct DisEnAbleUnitFilesResponse {
    carries_install_info: bool,
    changes: Vec<DisEnAbleUnitFiles>,
}
