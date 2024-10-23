use db_access::models::JobStatus;
use serde::{Deserialize, Serialize};

// timestamp ranges for each sub-job calculation
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PitchLakeJobRequestParams {
    pub twap: (i64, i64),
    pub volatility: (i64, i64),
    pub reserve_price: (i64, i64),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequest {
    pub identifiers: Vec<String>,
    pub params: PitchLakeJobRequestParams,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobResponse {
    pub job_id: String,
    pub message: Option<String>,
    pub status: Option<JobStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GetJobStatusResponseEnum {
    Success(JobResponse),
    Error(ErrorResponse),
}
