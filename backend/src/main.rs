mod db;
mod logging;
mod router;
mod sheets;
mod jobs;

use db::practice::Practice;
use router::responses::PracticeStartInfo;
use dotenv::dotenv;
use sheets::sheets::SheetsClient;
use std::error::Error;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;
use reqwest::Client as HttpClient;

use crate::db::db::DB;
use crate::router::router::create_router;
use crate::sheets::sheets::fetch_and_add_users;

#[tokio::main]
async fn main() {
    dotenv().ok();
    logging::init_logging();

    let db = Arc::new(DB::init().await.expect("Failed to initialize database"));

    let form_client = Arc::new(
        SheetsClient::init_form_client(db.clone())
            .await
            .expect("Failed to initialize form sheets client"),
    );

    let practice_client = Arc::new(
      SheetsClient::init_practice_client(db.clone())
        .await
        .expect("Failed to intialize practice client")
    );

    practice_client.initial_practice_sync()
      .await
      .expect("Failed to init sync to practice sheets");

    let app = create_router(db.clone());

    let scheduler = JobScheduler::new().await.unwrap();
    let db_clone = db.clone();
    let form_client_clone = form_client.clone();

    info!("Creating cron job for form sync");
    scheduler
        .add(
            Job::new_async("0 */1 * * * *", move |_uuid, _l| {
                // Every 5 minutes
                let db = db_clone.clone();
                let sheets = form_client_clone.clone();
                Box::pin(async move {
                    info!("Running cron job to sync users from sheets");
                    if let Err(e) = fetch_and_add_users(db, sheets).await {
                        eprintln!("Error fetching and adding users: {}", e);
                    }
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();

    scheduler.start().await.unwrap();

    let db_clone_notifications = db.clone();
    scheduler
        .add(
            Job::new_async("0 */1 * * * *", move |_uuid, _l| {
                // Every minute
                let db = db_clone_notifications.clone();
                Box::pin(async move {
                    info!("Checking for practices opening soon");
                    match db.get_practices_opening_soon().await {
                        Ok(practices) => {
                          info!("Got practices: {:?}", practices);
                            for practice in practices {
                                if let Err(e) = notify_discord_bot(&practice).await {
                                    eprintln!("Error notifying Discord bot: {}", e);
                                } else {
                                    info!("Successfully notified Discord bot for practice {}", practice.id.unwrap());
                                }
                            }
                        }
                        Err(e) => eprintln!("Error checking for opening practices: {}", e),
                    }
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();
    info!("Server starting on port 8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn notify_discord_bot(practice: &Practice) -> Result<(), Box<dyn Error>> {
    let client = HttpClient::new();
    let practice_info = PracticeStartInfo {
        practice_id: practice.id.unwrap().to_string(),
        start_time: practice.start_time,
        end_time: practice.end_time,
    };

    let response = client
        .post("http://discord-bot:3001/practice")
        .json(&practice_info)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to notify Discord bot: {}", response.status()).into());
    }

    Ok(())
}
