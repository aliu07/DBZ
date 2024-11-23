use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct SignupResponse {
  pub success: bool,
  pub message: String,
  pub on_waitlist: bool
}

#[derive(Serialize)]
pub struct PracticeStartInfo{
  pub practice_id: String,
  pub start_time: DateTime<Utc>,
  pub end_time: DateTime<Utc>
}
