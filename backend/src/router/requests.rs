use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDiscordUser {
  pub email: String,
  pub discord_id: String
}

#[derive(Deserialize)]
pub struct CreatePracticeRequest {
  pub date: DateTime<Utc>,
  pub start_time : DateTime<Utc>
}

#[derive(Deserialize)]
pub struct SignupRequest {
  pub practice_id: String,
  pub discord_id: String
}
