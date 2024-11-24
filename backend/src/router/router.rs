use axum::{
    extract::State,
    middleware,
    routing::{delete, post},
    Json, Router,
};
use mongodb::bson::oid::ObjectId;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{
    db::{
        db::DB,
        practice::{Practice, PracticeError},
    },
    logging::middleware::logging_middleware, router::responses::{PracticeStartInfo, WaitlistTransferNotification},
};
use std::sync::Arc;

use super::{
    requests::{CreateDiscordUser, CreatePracticeRequest, SignupRequest},
    responses::SignupResponse,
};

pub fn create_router(db: Arc<DB>) -> Router {
    Router::new()
        .route("/register", post(register_discord_user))
        .route("/practice", post(create_practice))
        .route("/practice/signup", post(signup_for_practice))
        .route("/practice/unregister", delete(unregister_for_practice))
        .layer(middleware::from_fn(logging_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}

async fn register_discord_user(
    State(db): State<Arc<DB>>,
    Json(req): Json<CreateDiscordUser>,
) -> Result<Json<String>, String> {
    let mut user = db
        .get_user_by_email(&req.email)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("User not found with given email")?;

    return match user.discord_id {
        Some(_) => Err("Discord id already associated to email".to_string()),
        None => {
            user.discord_id = Some(req.discord_id);
            info!("updated mongo object: {:?}", user);
            db.update_user(&user).await;
            Ok(Json(
                "Successfully registerd discord id to user".to_string(),
            ))
        }
    };
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
    info!(
        "Processing signup request for practice_id {}, discord_id: {}",
        req.practice_id, req.discord_id
    );

    let practice_id = ObjectId::parse_str(&req.practice_id).map_err(|e| e.to_string())?;

    let mut practice = db
        .get_practice(practice_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Practice not found")?;

    let user = db
        .get_user_by_discord_id(&req.discord_id)
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

    match practice
        .add_participant(&req.discord_id, &side, db.clone())
        .await
    {
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
                on_waitlist: !main,
            }))
        }

        Err(PracticeError::Full) => Ok(Json(SignupResponse {
            success: false,
            message: format!("{:?} side main list and waitlist are full", side),
            on_waitlist: false,
        })),
        Err(e) => Err(e.to_string()),
    }
}

    async fn unregister_for_practice(
        State(db): State<Arc<DB>>,
        Json(req): Json<SignupRequest>,
    ) -> Result<Json<SignupResponse>, String> {
        info!(
            "Processing unregister request for practice_id {}, discord_id: {}",
            req.practice_id, req.discord_id
        );

        let practice_id = ObjectId::parse_str(&req.practice_id).map_err(|e| e.to_string())?;

        let mut practice = db
            .get_practice(practice_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Practice not found")?;

        // Remove participant and get waitlist user if any
        match practice
            .remove_participant(&req.discord_id, db.clone())
            .await
        {
            Ok(maybe_waitlist_user) => {
                // Update practice in database
                db.update_practice(&practice)
                    .await
                    .map_err(|e| e.to_string())?;

                // If someone from waitlist was moved to main list, notify them
                if let Some(waitlist_user_id) = maybe_waitlist_user {
                    if let Ok(Some(user)) = db.get_user(waitlist_user_id).await {
                        if let Some(discord_id) = user.discord_id {
                            // Send notification to Discord bot
                            let client = reqwest::Client::new();
                            let notification = WaitlistTransferNotification {
                                practice: PracticeStartInfo {
                                    practice_id: practice.id.unwrap().to_string(),
                                    start_time: practice.start_time,
                                    end_time: practice.end_time,
                                },
                                discord_id: discord_id.parse().unwrap_or_default(),
                            };

                            // Ignore errors as this is not critical
                            let _ = client
                                .post("http://discord-bot:3001/waitlisted-msg")
                                .json(&notification)
                                .send()
                                .await;
                        }
                    }
                }

                Ok(Json(SignupResponse {
                    success: true,
                    message: "Successfully unregistered from practice".to_string(),
                    on_waitlist: false,
                }))
            }
            Err(PracticeError::UserNotFound) => Ok(Json(SignupResponse {
                success: false,
                message: "User not registered for this practice".to_string(),
                on_waitlist: false,
            })),
            Err(e) => Err(e.to_string()),
        }
    }
