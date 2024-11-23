use mongodb::bson::oid::ObjectId;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum UserType {
  Regular,
  Exec
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Gender {
  Male,
  Female,
  NA
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Side {
  Left,
  Right,
  NA
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id : Option<ObjectId>,
  pub first_name : String,
  pub last_name : String,
  pub gender : Gender,
  pub discord_id : String,
  pub user_type : UserType,
  pub side: Side
}
