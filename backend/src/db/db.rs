use mongodb::{Client, Database};
use std::error::Error;

use super::user::User;

pub struct DB {
    client: Client,
    db: Database,
}

impl DB {
    pub async fn init() -> Result<Self, Box<dyn Error>> {
        let username = std::env::var("MONGO_USERNAME").expect("MONGO_USERNAME must be set");
        let password = std::env::var("MONGO_PASSWORD").expect("MONGO_PASSWORD must be set");
        let host = std::env::var("MONGO_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("MONGO_PORT").unwrap_or_else(|_| "27017".to_string());
        let db_name = std::env::var("MONGO_DB_NAME").expect("MONGO_DB_NAME must be set");

        let mongo_uri = format!(
            "mongodb://{}:{}@{}:{}/{}",
            username, password, host, port, db_name
        );

        let client = Client::with_uri_str(&mongo_uri).await?;

        let db = client.database(&db_name);

        Ok(Self { client, db })
    }

    pub async fn create_user_from_sheet(&self, user: &User) -> Result<(), Box<dyn Error>> {
        let collection = self.db.collection::<User>("users");
        collection.insert_one(user).await?;
        Ok(())
    }
}
