mod db;
mod router;
mod sheets;

use axum::Router;
use dotenv::dotenv;
use std::sync::Arc;

use crate::db::db::DB;
use crate::router::router::create_router;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize the database
    let db = Arc::new(DB::init().await.expect("Failed to initialize database"));

    // Create the router
    let app = create_router(db);

    println!("Server starting on port 8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
