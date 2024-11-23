use crate::sheets::sheets::hyper_util::client::legacy::Client;
use google_sheets4::hyper_rustls::HttpsConnector;
use google_sheets4::{
    hyper_util::{self, client::legacy::connect::HttpConnector, rt::TokioExecutor},
    Sheets,
};
use hyper_rustls;
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;
use std::{env, error::Error};
use tokio::sync::Mutex;
use tracing::info;
use yup_oauth2::{read_service_account_key, ServiceAccountAuthenticator};

use crate::db::user::User;
use super::models::{FormResponse, SheetMetaData};
use crate::DB;

pub struct SheetsClient {
    service: Sheets<HttpsConnector<HttpConnector>>,
    sheet_id: String,
    range: String,
    last_row: Arc<Mutex<usize>>,
    db: Arc<DB>
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

        let sheets_client = Self::new(&credentials_path, &sheet_id, &range, db).await?;
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

        let sheets_client = Self::new(&credentials_path, &sheet_id, &range, db).await?;

        info!("Fitness sheets client initialized successfully");
        Ok(sheets_client)
    }

    pub async fn new(
        credentials_path: &str,
        sheet_id: &str,
        range: &str,
        db: Arc<DB>
    ) -> Result<Self, Box<dyn Error>> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");
        let creds = read_service_account_key(credentials_path)
            .await
            .expect("Can't read creds");

        let service_account = ServiceAccountAuthenticator::builder(creds)
            .build()
            .await
            .expect("failed to build service account");

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
      let parsed_sheet_id = ObjectId::parse_str(sheet_id)?;
      let metadata = db.get_sheet_metadata(parsed_sheet_id).await?;
      Ok(metadata.map_or(1, |m| m.last_processed_row))
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
              return Ok(vec![])
            },
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
        info!("Fetched and added {} new responses", new_responses.len());
        Ok(new_responses)
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
