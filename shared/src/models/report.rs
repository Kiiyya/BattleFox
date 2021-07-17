use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ReportModel {
    pub reporter: String,
    pub reported: String,
    pub reason: String,
    pub server_name: String,
    pub server_guid: Option<String>,
}