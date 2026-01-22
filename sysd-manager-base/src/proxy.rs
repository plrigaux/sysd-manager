use serde::{Deserialize, Serialize};
use zvariant::Type;

#[derive(Debug, Type, Serialize, Deserialize)]
// #[allow(unused)]
pub struct DisEnAbleUnitFiles {
    pub change_type: String,
    pub file_name: String,
    pub destination: String,
}
