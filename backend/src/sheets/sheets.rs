use crate::db::practice::Practice;
use crate::sheets::sheets::hyper_util::client::legacy::Client;
use google_sheets4::api::ValueRange;
use google_sheets4::hyper_rustls::HttpsConnector;
use google_sheets4::{
    hyper_util::{self, client::legacy::connect::HttpConnector, rt::TokioExecutor},
    Sheets,
};
use hyper_rustls;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::Once;
use std::{env, error::Error};
use tokio::sync::Mutex;
use tracing::info;
use yup_oauth2::{read_service_account_key, ServiceAccountAuthenticator};

use super::models::{FormResponse, PracticeSheetData, SheetMetaData};
use crate::db::user::User;
use crate::DB;

static RUSTLS_INIT: Once = Once::new();

pub struct SheetsClient {
    service: Sheets<HttpsConnector<HttpConnector>>,
    sheet_id: String,
    range: String,
    last_row: Arc<Mutex<usize>>,
    db: Arc<DB>,
}

impl SheetsClient {
    pub async fn init_form_client(db: Arc<DB>) -> Result<Self, Box<dyn Error>> {
        info!("Initing a form sheets client");

        let credentials_path =
            env::var("GOOGLE_CREDENTIALS_PATH").expect("GOOGLE_CREDENTIALS_PATH must be set");

        info!("Read credentials from: {}", &credentials_path);

        let sheet_id = env::var("FORM_ID").expect("FORM_ID must be set");

        let range = env::var("FORM_RANGE").expect("FORM_RANGE must be set");

        info!(
            "Connecting to sheet ID: {} with range {}",
            &sheet_id, &range
        );

        if db.get_sheet_metadata(&sheet_id).await?.is_none() {
            info!(
                "No metadata found for sheet {}, creating initial metadata",
                sheet_id
            );
            let initial_metadata = SheetMetaData {
                sheet_id: sheet_id.clone(),
                last_processed_row: 1,
            };
            db.create_sheet_metadata(&initial_metadata).await?;
        }

        let last_row = Self::get_last_processed_row(&db, &sheet_id).await?;

        info!("Read last processed row as : {}", last_row);

        let sheets_client = Self::new(&credentials_path, &sheet_id, &range, db).await?;

        *sheets_client.last_row.lock().await = last_row;

        info!("Form sheets client initialized successfully");

        Ok(sheets_client)
    }

    pub async fn init_practice_client(db: Arc<DB>) -> Result<Self, Box<dyn Error>> {
        info!("Initing a practice sheets client");

        let credentials_path =
            env::var("GOOGLE_CREDENTIALS_PATH").expect("GOOGLE_CREDENTIALS_PATH must be set");

        info!("Read credentials from: {}", &credentials_path);

        let sheet_id = env::var("PRACTICE_ID").expect("PRACTCICE_ID must be set");

        let range = env::var("PRACTICE_RANGE").expect("PRACTICE_RANGE must be set");

        info!(
            "Connecting to sheet ID: {} with range {}",
            &sheet_id, &range
        );

        let sheets_client = Self::new(&credentials_path, &sheet_id, &range, db).await?;

        info!("Practice sheets client initialized successfully");

        Ok(sheets_client)
    }

    pub async fn init_fitness_client(db: Arc<DB>) -> Result<Self, Box<dyn Error>> {
        info!("Initing a fitness sheets client");
        let credentials_path =
            env::var("GOOGLE_CREDENTIALS_PATH").expect("GOOGLE_CREDENTIALS_PATH must be set");

        info!("Read credentials from: {}", &credentials_path);
        let sheet_id = env::var("FITNESS_ID").expect("FITNESS_ID must be set");

        let range = env::var("FITNESS_RANGE").expect("FITNESS_RANGE must be set");

        info!(
            "Connecting to sheet ID: {} with range {}",
            &sheet_id, &range
        );

        let last_row = Self::get_last_processed_row(&db, &sheet_id).await?;

        info!("Read last processed row as : {}", last_row);
        let sheets_client = Self::new(&credentials_path, &sheet_id, &range, db).await?;
        *sheets_client.last_row.lock().await = last_row;

        info!("Fitness sheets client initialized successfully");
        Ok(sheets_client)
    }

    pub async fn new(
        credentials_path: &str,
        sheet_id: &str,
        range: &str,
        db: Arc<DB>,
    ) -> Result<Self, Box<dyn Error>> {
        RUSTLS_INIT.call_once(|| {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Failed to install rustls crypto provider")
        });

        let creds = read_service_account_key(credentials_path)
            .await
            .expect("Can't read creds");

        let service_account = ServiceAccountAuthenticator::builder(creds)
            .build()
            .await
            .expect("Failed to build service account");

        let hub = Sheets::new(
            Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()?
                    .https_or_http()
                    .enable_http1()
                    .enable_http2()
                    .build(),
            ),
            service_account,
        );

        Ok(Self {
            service: hub,
            sheet_id: sheet_id.to_string(),
            range: range.to_string(),
            last_row: Arc::new(Mutex::new(1)),
            db,
        })
    }

    async fn get_last_processed_row(db: &DB, sheet_id: &str) -> Result<usize, Box<dyn Error>> {
        info!(
            "Attempting to get last processed row for sheet: {}",
            sheet_id
        );

        match db.get_sheet_metadata(sheet_id).await? {
            Some(metadata) => {
                info!(
                    "Found existing metadata: last_processed_row = {}",
                    metadata.last_processed_row
                );
                Ok(metadata.last_processed_row)
            }
            None => {
                info!("No existing metadata found, creating initial metadata");

                let metadata = SheetMetaData {
                    sheet_id: sheet_id.to_string(),
                    last_processed_row: 1,
                };

                db.update_sheet_metadata(&metadata).await?;
                info!("Created initial metadata for sheet {}", sheet_id);
                Ok(1)
            }
        }
    }

    async fn update_last_processed_row(&self, row: usize) -> Result<(), Box<dyn Error>> {
        let metadata = SheetMetaData {
            sheet_id: self.sheet_id.clone(),
            last_processed_row: row,
        };

        self.db.update_sheet_metadata(&metadata).await?;
        Ok(())
    }

    pub async fn fetch_new_form_responses(&self) -> Result<Vec<FormResponse>, Box<dyn Error>> {
        info!("Fetching new form responses");
        let result = self
            .service
            .spreadsheets()
            .values_get(&self.sheet_id, &self.range)
            .doit()
            .await?;

        let values = match result.1.values {
            Some(vals) => vals,
            None => {
                info!("No new responses found");
                return Ok(vec![]);
            }
        };

        let mut last_row = self.last_row.lock().await;
        let start = *last_row;
        let mut new_responses = Vec::new();

        for row in values.iter().skip(start) {
            // Ensure the row has at least 8 columns (0-based index 7 for column H)
            if row.len() < 8 {
                continue; // Skip incomplete rows
            }

            let form_response = FormResponse {
                email_address: row[1].clone().as_str().unwrap_or_default().to_string(), // Column B
                full_name: row[2].clone().as_str().unwrap_or_default().to_string(),     // Column C
                mcgill_id: row[3].clone().as_str().unwrap_or_default().to_string(),     // Column D
                preferred_email: row[4].clone().as_str().unwrap_or_default().to_string(), // Column E
                paddle_side: row[7].clone().as_str().unwrap_or_default().to_string(), // Column H
            };

            new_responses.push(form_response);
            *last_row += 1;
        }

        if *last_row > start {
            info!(
                "Updating last processed row from {} to {}",
                start, *last_row
            );
            self.update_last_processed_row(*last_row).await?;
        }

        info!("Fetched and added {} new responses", new_responses.len());

        Ok(new_responses)
    }

    pub async fn initial_practice_sync(&self) -> Result<(), Box<dyn Error>> {
        let practice_data = self.fetch_practice_data().await?;

        for data in practice_data {
            tracing::info!("Creating initial practice for date: {}", data.date);

            // Only create if practice doesn't exist
            if self.db.get_practice_by_date(data.date).await?.is_none() {
                let practice = Practice::from_sheet_data(&data);
                self.db.create_practice(&practice).await?;
                tracing::info!("Successfully created new practice");
            }
        }

        Ok(())
    }

    pub async fn fetch_practice_data(&self) -> Result<Vec<PracticeSheetData>, Box<dyn Error>> {
        // First, get all sheet names/ids
        let spreadsheet = self
            .service
            .spreadsheets()
            .get(&self.sheet_id)
            .doit()
            .await?
            .1;

        let mut all_practice_data = Vec::new();

        // Iterate through each sheet
        for sheet in spreadsheet.sheets.unwrap_or_default() {
            let sheet_title = sheet.properties.unwrap().title.unwrap();
            tracing::info!("Processing sheet: {}", sheet_title);

            // Create range with sheet name
            let range = format!("{}!{}", sheet_title, self.range);

            // Fetch data from this specific sheet
            let result = self
                .service
                .spreadsheets()
                .values_get(&self.sheet_id, &range)
                .doit()
                .await?;

            if let Some(values) = result.1.values {
                // Convert Vec<Vec<Value>> to Vec<Vec<String>>
                let string_values: Vec<Vec<String>> = values
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|cell| cell.as_str().unwrap_or("").to_string())
                            .collect()
                    })
                    .collect();

                match PracticeSheetData::parse_from_rows(string_values) {
                    Ok(practice_data) => all_practice_data.push(practice_data),
                    Err(e) => tracing::error!("Failed to parse sheet {}: {}", sheet_title, e),
                }
            }
        }

        tracing::info!("Processed {} sheets", all_practice_data.len());
        Ok(all_practice_data)
    }

    pub async fn update_sheet_from_practice(
        &self,
        practice: &Practice,
    ) -> Result<(), Box<dyn Error>> {
        let values = self.format_practice_for_sheet(practice).await?;

        // Update the sheet with the new values
        let range = format!("A1:H{}", values.len()); // Adjust range based on your sheet structure
        let req = ValueRange {
            range: Some(range),
            values: Some(values),
            major_dimension: None,
        };

        self.service
            .spreadsheets()
            .values_update(req, &self.sheet_id, &self.range)
            .value_input_option("RAW")
            .doit()
            .await?;

        Ok(())
    }

    async fn format_practice_for_sheet(
        &self,
        practice: &Practice,
    ) -> Result<Vec<Vec<JsonValue>>, Box<dyn Error>> {
        let mut values = Vec::new();

        // Add header row
        values.push(vec![JsonValue::String(format!(
            "{}",
            practice.date.format("%A, %B %d (%I:%M %p)")
        ))]);
        values.push(vec![]); // Empty row
        values.push(vec![]); // Empty row

        // Add column headers
        values.push(vec![
            JsonValue::String("".to_string()),
            JsonValue::String("LEFTIES".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("RIGHTIES".to_string()),
        ]);

        // Add participant rows
        for i in 0..17 {
            let mut row = vec![JsonValue::String("".to_string()); 6];

            // Left side
            if let Some(user_id) = &practice.left_side[i] {
                if let Some(user) = self.db.get_user(*user_id).await? {
                    row[1] = JsonValue::String(user.first_name);
                    row[2] = JsonValue::String(user.last_name);
                }
            }

            // Right side
            if let Some(user_id) = &practice.right_side[i] {
                if let Some(user) = self.db.get_user(*user_id).await? {
                    row[5] = JsonValue::String(user.first_name);
                    row[6] = JsonValue::String(user.last_name);
                }
            }

            values.push(row);
        }

        // Add waitlist header
        values.push(vec![
            JsonValue::String("WAITLIST".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("".to_string()),
            JsonValue::String("".to_string()),
        ]);

        // Add waitlist rows
        for i in 0..6 {
            let mut row = vec![JsonValue::String("".to_string()); 6];

            // Left side waitlist
            if let Some(user_id) = &practice.left_side_waitlist[i] {
                if let Some(user) = self.db.get_user(*user_id).await? {
                    row[1] = JsonValue::String(user.first_name);
                    row[2] = JsonValue::String(user.last_name);
                }
            }

            // Right side waitlist
            if let Some(user_id) = &practice.right_side_waitlist[i] {
                if let Some(user) = self.db.get_user(*user_id).await? {
                    row[5] = JsonValue::String(user.first_name);
                    row[6] = JsonValue::String(user.last_name);
                }
            }

            values.push(row);
        }

        Ok(values)
    }
}

pub(crate) async fn fetch_and_add_users(
    db: Arc<DB>,
    sheets_client: Arc<SheetsClient>,
) -> Result<(), Box<dyn Error>> {
    let new_responses = sheets_client.fetch_new_form_responses().await?;

    for response in new_responses {
        let user = User::convert_form_to_user(&response)?;
        db.create_user_from_sheet(&user).await?;
    }

    Ok(())
}
