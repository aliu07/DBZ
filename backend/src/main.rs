mod db;
mod logging;
mod router;
mod sheets;

use dotenv::dotenv;
use sheets::sheets::SheetsClient;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

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

    info!("Server starting on port 8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
