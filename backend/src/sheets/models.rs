use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FormResponse {
    pub email_address: String,
    pub full_name: String,
    pub mcgill_id: String,
    pub preferred_email: String,
    pub paddle_side: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SheetMetaData {
  #[serde(rename = "_id")]
  pub sheet_id: String,
  pub last_processed_row: usize
}
