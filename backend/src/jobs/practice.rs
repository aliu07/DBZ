use chrono::{DateTime, Duration, Utc};
use mongodb::bson::oid::ObjectId;
use reqwest::Client as HttpClient;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;
use std::{error::Error, sync::Arc, time::Duration as StdDuration};

use crate::db::db::DB;
use crate::db::practice::Practice;
use crate::router::responses::PracticeStartInfo;

pub async fn schedule_practice_jobs(
    practice: &Practice,
    scheduler: &JobScheduler,
    db: Arc<DB>,
) -> Result<(), Box<dyn Error>> {
    let practice_id = practice.id.unwrap();
    let waitlist_transfer_time =
        practice.start_time - chrono::Duration::hours(1) - chrono::Duration::seconds(30);

    let unlock_time = practice.start_time - chrono::Duration::hours(1);
    let now = Utc::now();

    // Only schedule if times are in the future
    if waitlist_transfer_time > now {
        info!("Creating job for waitlist transfer @ {}", waitlist_transfer_time);
        schedule_waitlist_transfer(scheduler, db.clone(), practice_id, waitlist_transfer_time).await?;
    } else {
        info!("Skipping waitlist transfer job as time {} has passed", waitlist_transfer_time);
    }

    if unlock_time > now {
        info!("Creating job for unlock time transfer @ {}", unlock_time);
        schedule_practice_unlock(scheduler, db.clone(), practice_id, unlock_time).await?;
    } else {
        info!("Skipping unlock time job as time {} has passed", unlock_time);
    }

    Ok(())
}

async fn schedule_waitlist_transfer(
    scheduler: &JobScheduler,
    db: Arc<DB>,
    practice_id: ObjectId,
    execution_time: DateTime<Utc>,
) -> Result<(), Box<dyn Error>> {
    scheduler
        .add(
            Job::new_one_shot_async(execution_time
              .signed_duration_since(Utc::now())
              .to_std()
              .map_err(|_| format!("Target time is in the past {}", execution_time))?,
            move |_uuid, _l| {
                let db = db.clone();
                let practice_id = practice_id.clone();
                Box::pin(async move {
                    info!("Executing waitlist transfer for practice {}", practice_id);
                    if let Err(e) = handle_waitlist_transfer(db, practice_id).await {
                        tracing::error!("Waitlist transfer failed: {}", e);
                    }
                })
            })
            .unwrap(),
        )
        .await?;
    Ok(())
}

async fn schedule_practice_unlock(
    scheduler: &JobScheduler,
    db: Arc<DB>,
    practice_id: ObjectId,
    execution_time: DateTime<Utc>,
) -> Result<(), Box<dyn Error>> {
    scheduler
        .add(
            Job::new_one_shot_async(
                execution_time
                  .signed_duration_since(Utc::now())
                  .to_std()
                  .map_err(|_| format!("Target time is in the past {}", execution_time))?,
                move |_uuid, _l| {
                    let db = db.clone();
                    let practice_id = practice_id.clone();
                    Box::pin(async move {
                        info!("Notifying practice unlock for practice {}", practice_id);
                        if let Err(e) = notify_practice_unlock(db, practice_id).await {
                            eprintln!("Error notifying practice unlock: {}", e);
                        }
                    })
                },
            )
            .unwrap(),
        )
        .await?;

    Ok(())
}

async fn handle_waitlist_transfer(
    db: Arc<DB>,
    practice_id: ObjectId,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(mut practice) = db.get_practice(practice_id).await? {
        info!(
            "Processing waitlist transfer for practice on {}",
            practice.date
        );

        if let Some(previous_practice) = db.get_previous_practice(&practice).await? {
            practice.transfer_waitlist(&previous_practice);
            db.update_practice(&practice).await?;

            info!(
                "Successfully transferred waitlist for practice {}",
                practice.id.unwrap()
            );
        }
    }

    Ok(())
}

async fn notify_practice_unlock(db: Arc<DB>, practice_id: ObjectId) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(practice) = db.get_practice(practice_id).await? {
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
    }

    Ok(())
}
