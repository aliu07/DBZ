use google_sheets4::{hyper_util::{self, client::legacy::connect::HttpConnector, rt::TokioExecutor}, Sheets};
use google_sheets4::hyper_rustls::HttpsConnector;
use rustls::crypto::CryptoProvider;
use serde::Deserialize;
use std::{env, error::Error};
use std::sync::Arc;
use tokio::sync::Mutex;
use yup_oauth2::{read_service_account_key, ServiceAccountAuthenticator};
use crate::sheets::sheets::hyper_util::client::legacy::Client;
use hyper;
use hyper_rustls;

use crate::db::user::User;
use crate::DB;

#[derive(Debug, Deserialize)]
pub struct FormResponse {
    pub email_address: String,
    pub full_name: String,
    pub mcgill_id: String,
    pub preferred_email: String,
    pub paddle_side: String
}

pub struct SheetsClient {
    service: Sheets<HttpsConnector<HttpConnector>>,
    sheet_id: String,
    range: String,
    last_row: Arc<Mutex<usize>>,
}

impl SheetsClient {
    pub async fn init_form_client() -> Result<Self, Box<dyn Error>> {
        let credentials_path = env::var("GOOGLE_CREDENTIALS_PATH")
            .expect("GOOGLE_CREDENTIALS_PATH must be set");

        let sheet_id = env::var("FORM_ID")
            .expect("FORM_ID must be set");

        let range = env::var("FORM_RANGE")
            .expect("FORM_RANGE must be set");

        let sheets_client = Self::new(&credentials_path, &sheet_id, &range).await?;

        Ok(sheets_client)
    }

    pub async fn new(credentials_path: &str, sheet_id: &str, range: &str) -> Result<Self, Box<dyn Error>> {
        rustls::crypto::ring::default_provider().install_default().expect("Failed to install rustls crypto provider");
        let creds = read_service_account_key(credentials_path).await.expect("Can't read creds");

        let service_account = ServiceAccountAuthenticator::builder(creds)
            .build()
            .await
            .expect("failed to build service account");

        let hub = Sheets::new(
            Client::builder(TokioExecutor::new()).build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots()?.https_or_http().enable_http1().enable_http2().build()),
            service_account
        );

        Ok(Self {
            service: hub,
            sheet_id: sheet_id.to_string(),
            range: range.to_string(),
            last_row: Arc::new(Mutex::new(1)),
        })
    }

    pub async fn fetch_new_responses(&self) -> Result<Vec<FormResponse>, Box<dyn Error>> {
        let result = self
            .service
            .spreadsheets()
            .values_get(&self.sheet_id, &self.range)
            .doit()
            .await?;

        let values = match result.1.values {
            Some(vals) => vals,
            None => return Ok(vec![]),
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
                email_address: row[1].clone().as_str().unwrap_or_default().to_string(),// Column B
                full_name: row[2].clone().as_str().unwrap_or_default().to_string(),     // Column C
                mcgill_id: row[3].clone().as_str().unwrap_or_default().to_string(), // Column D
                preferred_email: row[4].clone().as_str().unwrap_or_default().to_string(), // Column E
                paddle_side: row[7].clone().as_str().unwrap_or_default().to_string(),   // Column H
            };

            new_responses.push(form_response);
            *last_row += 1;
        }

        Ok(new_responses)
    }
}

pub(crate) async fn fetch_and_add_users(db: Arc<DB>, sheets_client: Arc<SheetsClient>) -> Result<(), Box<dyn Error>> {
    let new_responses = sheets_client.fetch_new_responses().await?;

    for response in new_responses {
        let user = User::convert_form_to_user(&response)?;
        db.create_user_from_sheet(&user).await?;
    }

    Ok(())
}
