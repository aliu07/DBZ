mod db;
mod router;
mod sheets;

use axum::Router;
use dotenv::dotenv;
use sheets::sheets::SheetsClient;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::db::db::DB;
use crate::router::router::create_router;
use crate::sheets::sheets::fetch_and_add_users;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db = Arc::new(DB::init().await.expect("Failed to initialize database"));

    let form_client = Arc::new(SheetsClient::init_form_client().await.expect("Failed to initialize form sheets client"));

    let app = create_router(db.clone());

    let scheduler = JobScheduler::new().await.unwrap();
    let db_clone = db.clone();
    let form_client_clone = form_client.clone();

    scheduler
        .add(
            Job::new_async("0 */5 * * * *", move |_uuid, _l| { // Every 5 minutes
                let db = db_clone.clone();
                let sheets = form_client_clone.clone();
                Box::pin(async move {
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

    println!("Server starting on port 8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
