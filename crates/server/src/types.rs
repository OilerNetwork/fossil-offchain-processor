use db_access::models::JobStatus;
use serde::{Deserialize, Serialize};
use starknet_crypto::Felt;

// timestamp ranges for each sub-job calculation
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PitchLakeJobRequestParams {
    pub twap: (u64, u64),
    pub volatility: (u64, u64),
    pub reserve_price: (u64, u64),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequest {
    pub identifiers: Vec<String>,
    pub params: PitchLakeJobRequestParams,
    pub client_info: ClientInfo, // New field
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientInfo {
    pub client_address: Felt,
    pub vault_address: Felt,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobResponse {
    pub job_id: String,
    pub message: Option<String>,
    pub status: Option<JobStatus>,
}

impl JobResponse {
    pub fn new(job_id: String, message: Option<String>, status: Option<JobStatus>) -> Self {
        Self {
            job_id,
            message,
            status,
        }
    }
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
