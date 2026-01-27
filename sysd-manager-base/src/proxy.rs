use serde::{Deserialize, Serialize};
use zvariant::{OwnedObjectPath, Type};

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

#[derive(Debug, Type, Serialize, Deserialize)]
pub struct QueuedJobs {
    ///The numeric job id
    job_id: u32,

    //The primary unit name for this job
    primary_unit_name: String,

    //The job type as string
    job_type: String,

    ///The job state as string
    job_state: String,

    ///The job object path
    job_object: OwnedObjectPath,

    ///The unit object path
    unit_object: OwnedObjectPath,
}
