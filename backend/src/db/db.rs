use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Database,
};
use std::error::Error;
use tracing::info;

use crate::sheets::models::SheetMetaData;

use super::{practice::Practice, user::User};

pub struct DB {
    client: Client,
    db: Database,
}

impl DB {
    pub async fn init() -> Result<Self, Box<dyn Error>> {
        info!("Init DB connection");
        let username = std::env::var("MONGO_USERNAME").expect("MONGO_USERNAME must be set");
        let password = std::env::var("MONGO_PASSWORD").expect("MONGO_PASSWORD must be set");
        let host = std::env::var("MONGO_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("MONGO_PORT").unwrap_or_else(|_| "27017".to_string());
        let db_name = std::env::var("MONGO_DB_NAME").expect("MONGO_DB_NAME must be set");

        let mongo_uri = format!(
            "mongodb://{}:{}@{}:{}/{}?authSource=admin",
            username, password, host, port, db_name
        );

        info!("Connecting to mongoDB with {}", mongo_uri);

        let client = Client::with_uri_str(&mongo_uri).await?;

        let db = client.database(&db_name);

        info!("Successfully connected to mongoDB, database: {}", &db_name);
        Ok(Self { client, db })
    }

    pub async fn create_user_from_sheet(&self, user: &User) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<User>("users");
        collection.insert_one(user).await?;
        Ok(())
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, Box<dyn Error>> {
        let collection = self.db.collection::<User>("users");
        Ok(collection.find_one(doc! {"email" : email}).await?)
    }

    pub async fn update_user(&self, user: &User) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<User>("users");
        collection
            .replace_one(doc! { "discord_id" : &user.id.unwrap()}, user)
            .await?;
        Ok(())
    }

    pub async fn create_practice(&self, practice: &Practice) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<Practice>("practices");
        collection.insert_one(practice).await?;
        Ok(())
    }

    pub async fn get_practice(
        &self,
        practice_id: ObjectId,
    ) -> Result<Option<Practice>, Box<dyn Error + Send + Sync>> {
        let collection = self.db.collection::<Practice>("practices");
        Ok(collection.find_one(doc! {"_id": practice_id}).await?)
    }

    pub async fn get_user(&self, user_id: ObjectId) -> Result<Option<User>, Box<dyn Error>> {
        let collection = self.db.collection::<User>("users");
        Ok(collection.find_one(doc! {"_id" : user_id}).await?)
    }

    pub async fn update_practice(&self, practice: &Practice) -> Result<(), Box<dyn Error + Send + Sync>> {
        let collection = self.db.collection::<Practice>("practices");
        collection
            .replace_one(doc! {"_id" : practice.id.unwrap()}, practice)
            .await?;
        Ok(())
    }

    pub async fn get_sheet_metadata(
        &self,
        sheet_id: &str,
    ) -> Result<Option<SheetMetaData>, Box<dyn Error>> {
        let collection = self.db.collection::<SheetMetaData>("sheets_metadata");
        Ok(collection.find_one(doc! {"_id" : sheet_id}).await?)
    }

    pub async fn create_sheet_metadata(
        &self,
        metadata: &SheetMetaData,
    ) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<SheetMetaData>("sheets_metadata");
        info!(
            "Creating new sheet metadata for sheet: {}",
            metadata.sheet_id
        );
        collection.insert_one(metadata).await?;
        info!("Successfully created sheet metadata");
        Ok(())
    }

    pub async fn update_sheet_metadata(
        &self,
        metadata: &SheetMetaData,
    ) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<SheetMetaData>("sheets_metadata");
        info!("Updating sheet metadata for sheet: {}", metadata.sheet_id);
        collection
            .replace_one(doc! {"_id" : &metadata.sheet_id}, metadata)
            .await?;
        info!("Successfully updated sheet metadata");
        Ok(())
    }

    pub async fn get_practices_opening_soon(
        &self,
    ) -> Result<Vec<Practice>, Box<dyn Error + Send + Sync>> {
        let collection = self.db.collection::<Practice>("practices");
        let now = Utc::now();
        let one_hour_from_now = now + chrono::Duration::hours(1);

        info!("Time now: {}", now);
        info!("Time in an hour: {}", one_hour_from_now);

        // Filter for practices that:
        // 1. Start after current time
        // 2. Start before one hour from now
        let filter = doc! {
            "start_time": {
                "$gt": now.to_rfc3339(),
                "$lt": one_hour_from_now.to_rfc3339()
            }
        };

        let mut cursor = collection.find(filter).await?;
        let mut practices = Vec::new();

        while let Some(practice) = cursor.try_next().await? {
            practices.push(practice);
        }

        Ok(practices)
    }

    pub async fn get_practice_by_date(
        &self,
        date: DateTime<Utc>,
    ) -> Result<Option<Practice>, Box<dyn Error>> {
        let collection = self.db.collection::<Practice>("practices");
        Ok(collection
            .find_one(doc! {"date": date.to_rfc3339()})
            .await?)
    }

    pub async fn get_previous_practice(
      &self,
      curr_practice: &Practice
    ) -> Result<Option<Practice>, Box<dyn Error + Send + Sync>> {
      let collection = self.db.collection::<Practice>("practices");
      let previous_practice_time = curr_practice.start_time - chrono::Duration::weeks(1);

      Ok(collection
        .find_one(doc!{"start_time" : previous_practice_time.to_rfc3339()})
        .await?)
    }
    pub async fn get_all_practices(&self) -> Result<Vec<Practice>, Box<dyn Error>> {
        let collection = self.db.collection::<Practice>("practices");
        let mut cursor = collection.find(doc!{}).await?;

        let mut practices = Vec::new();
        while let Some(practice) = cursor.try_next().await? {
            practices.push(practice);
        }

        Ok(practices)
    }
}
