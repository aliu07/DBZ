use std::{error::Error, sync::Arc};
use tokio_cron_scheduler::JobScheduler;
use tracing::info;

use crate::DB;
use crate::sheets::sheets::SheetsClient;

pub struct SchedulerManager {
  scheduler: JobScheduler,
  db: Arc<DB>
}

impl SchedulerManager {
  pub async fn new(db: Arc<DB>) -> Result<Self, Box<dyn Error>> {
    let scheduler = JobScheduler::new().await?;

    Ok(Self{scheduler, db})
  }

  pub async fn init_jobs(&self, practice_client: Arc<SheetsClient>) -> Result<(), Box<dyn Error>> {
    info!("Initing sheets sync and setting up cron jobs");
    practice_client.initial_practice_sync(&self.scheduler).await?;
    self.scheduler.start().await?;
    info!("Jobs scheduled successfully");
    Ok(())
  }

  pub fn get_scheduler(&self) -> &JobScheduler {
    &self.scheduler
  }
}
