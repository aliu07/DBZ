use axum::{extract::State, routing::post, Json, Router};
use mongodb::bson::oid::ObjectId;

use crate::db::{db::DB, practice::{Practice, PracticeError}, user::{Side, User}};
use std::sync::Arc;

use super::{
    requests::{CreatePracticeRequest, SignupRequest},
    responses::SignupResponse,
};

pub fn create_router(db: Arc<DB>) -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/practice", post(create_practice))
        .route("/practice/signup", post(signup_for_practice))
        .with_state(db)
}

async fn register_user(
    State(db): State<Arc<DB>>,
    Json(req): Json<User>,
) -> Result<Json<User>, String> {
    db.create_user_from_sheet(&req)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(req))
}

async fn create_practice(
    State(db): State<Arc<DB>>,
    Json(req): Json<CreatePracticeRequest>,
) -> Result<Json<Practice>, String> {
    let practice = Practice::new(req.date, req.start_time);
    db.create_practice(&practice)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(practice))
}

async fn signup_for_practice(
    State(db): State<Arc<DB>>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<SignupResponse>, String> {
    let practice_id = ObjectId::parse_str(&req.practice_id).map_err(|e| e.to_string())?;

    let user_id = ObjectId::parse_str(&req.user_id).map_err(|e| e.to_string())?;

    let mut practice = db
        .get_practice(practice_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Practice not found")?;

    let user = db
        .get_user(user_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("User not found")?;

    if practice.is_locked() {
        return Ok(Json(SignupResponse {
            success: false,
            message: "Practice is locked until one hour before start time".to_string(),
            on_waitlist: false,
        }));
    }

    let side = practice.determine_side(&user.side);

    match practice.add_participant(user_id, &side) {
      Ok(main) => {
        db.update_practice(&practice)
          .await
          .map_err(|e| e.to_string())?;

        Ok(Json(SignupResponse {
          success: true,
          message: if main {
            format!("Signed up on main list")
          } else {
            format!("Signed up for waitlist")
          },
          on_waitlist: main
        }))
      },

      Err(PracticeError::Full) => Ok(Json(SignupResponse {
          success: false,
          message: format!("{:?} side main list and waitlist are full", side),
          on_waitlist: false,
      })),
      Err(e) => Err(e.to_string()),
    }
}
