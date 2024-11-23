use mongodb::bson::oid::ObjectId;
use serde::{Serialize, Deserialize};
use std::error::Error;

use crate::sheets::sheets::FormResponse;

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
  pub discord_id : Option<String>,
  pub mcgill_id: String,
  pub email: String,
  pub user_type : UserType,
  pub side: Side
}

impl User {
    pub fn convert_form_to_user(form: &FormResponse) -> Result<Self, Box<dyn Error>> {
        let mut name_parts = form.full_name.trim().splitn(2, ' ');
        let first_name = name_parts.next().unwrap_or("").to_string();
        let last_name = name_parts.next().unwrap_or("").to_string();

        let gender = Gender::NA; //todo! this is not given by form

        let user_type = UserType::Regular;

        let side = match form.paddle_side.as_str() {
            "Left" => Side::Left,
            "Right" => Side::Right,
            "Ambidextrous (Both Sides)" => Side::NA,
            "Not sure yet!" => Side::NA,
            _ => Side::NA, // Throw an error
        };

        let email = form.preferred_email.clone();

        Ok(Self {
            id: None,
            first_name,
            last_name,
            gender,
            discord_id: None,
            mcgill_id: form.mcgill_id.clone(),
            user_type,
            side,
            email
        })
    }
}
