use serde::Serialize;
#[cfg(test)]
use diesel::prelude::*;

#[derive(Serialize)]
pub struct ErrorMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    pub message: String,
}

#[cfg(test)]
#[derive(QueryableByName)]
pub struct TestResult {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub test: i32,
}
