use std::{error::Error, sync::Arc};
use tokio_cron_scheduler::JobScheduler;
use tracing::info;

use crate::jobs::practice::schedule_practice_jobs;
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
    practice_client.initial_practice_sync().await?;

    let practices = self.db.get_all_practices().await?;

    for practice in practices {
      if practice.is_future() {
        schedule_practice_jobs(&practice, &self.scheduler, self.db.clone()).await?;
      }
    }
    self.scheduler.start().await?;
    info!("Jobs scheduled successfully");
    Ok(())
  }

  pub fn get_scheduler(&self) -> &JobScheduler {
    &self.scheduler
  }
}
