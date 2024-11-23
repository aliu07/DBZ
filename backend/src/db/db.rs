use mongodb::{bson::{oid::ObjectId, doc}, Client, Database};
use std::{collections, error::Error};
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

    pub async fn create_practice(&self, practice: &Practice) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<Practice>("practices");
        collection.insert_one(practice).await?;
        Ok(())
    }

    pub async fn get_practice(&self, practice_id: ObjectId) -> Result<Option<Practice>, Box<dyn Error>> {
      let collection = self.db.collection::<Practice>("practices");
      Ok(collection.find_one(doc! {"_id": practice_id}).await?)
    }

    pub async fn get_user(&self, user_id: ObjectId) -> Result<Option<User>, Box<dyn Error>> {
      let collection = self.db.collection::<User>("users");
      Ok(collection.find_one(doc! {"_id" : user_id}).await?)
    }

    pub async fn update_practice(&self, practice: &Practice) -> Result<(), Box<dyn Error>> {
      let collection = self.db.collection::<Practice>("practices");
      collection.replace_one(doc! {"_id" : practice.id.unwrap()}, practice).await?;
      Ok(())
    }

    pub async fn get_sheet_metadata(&self, sheet_id: ObjectId) -> Result<Option<SheetMetaData>, Box<dyn Error>> {
      let collection = self.db.collection::<SheetMetaData>("sheets_metadata");
      Ok(collection.find_one(doc! {"_id" : sheet_id}).await?)
    }

    pub async fn update_sheet_metadata(&self, metadata: &SheetMetaData) -> Result<(), Box<dyn Error>> {
      let collection = self.db.collection::<SheetMetaData>("sheets_metadata");
      collection.replace_one(doc! {"_id" : &metadata.sheet_id}, metadata).await?;
      Ok(())
    }
}
