use axum::{extract::State, routing::post, Json, Router};

use crate::db::{db::DB, user::User};
use std::sync::Arc;

pub fn create_router(db: Arc<DB>) -> Router {
    Router::new().route("/register", post(register_user).with_state(db))
}

async fn register_user(
    State(db): State<Arc<DB>>,
    Json(user): Json<User>,
) -> Result<Json<User>, String> {
  db.create_user_from_sheet(&user)
    .await
    .map_err(|e| e.to_string())?;

  Ok(Json(user))
}
