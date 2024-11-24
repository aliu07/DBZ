use crate::sheets::models::PracticeSheetData;

use super::user::{Side, UserType};
use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PracticeError {
    #[error("Practice is locked")]
    Locked,
    #[error("Practice and waitlist are full")]
    Full,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Practice {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub date: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub left_side: Vec<Option<ObjectId>>,
    pub right_side: Vec<Option<ObjectId>>,
    pub left_side_waitlist: Vec<Option<ObjectId>>,
    pub right_side_waitlist: Vec<Option<ObjectId>>,
}

impl Practice {
    pub fn new(date: DateTime<Utc>, start_time: DateTime<Utc>) -> Self {
        let end_time = start_time + chrono::Duration::hours(1);

        Self {
            id: None,
            date,
            start_time,
            end_time,
            left_side: vec![None; 17],
            right_side: vec![None; 17],
            left_side_waitlist: vec![None; 6],
            right_side_waitlist: vec![None; 6],
        }
    }

    pub fn from_sheet_data(data: &PracticeSheetData) -> Self {
        Self {
            id: None,
            date: data.date,
            start_time: data.date,
            end_time: data.date + chrono::Duration::hours(1),
            left_side: vec![None; 17], // We'll update these after creating users
            right_side: vec![None; 17],
            left_side_waitlist: vec![None; 6],
            right_side_waitlist: vec![None; 6],
        }
    }

    //TO DEPRCEATE
    pub fn is_locked(&self) -> bool {
        let now = Utc::now();
        let unlock_time = self.start_time - chrono::Duration::hours(1);

        now < unlock_time
    }

    fn count_side(&self, side: &Side) -> usize {
        let spots = match side {
            Side::Left => &self.left_side,
            Side::Right => &self.right_side,
            _ => panic!(),
        };
        spots.iter().filter(|spot| spot.is_some()).count()
    }

    pub(crate) fn determine_side(&self, side: &Side) -> Side {
        match side {
            Side::NA => {
                if self.count_side(&Side::Right) >= self.count_side(&Side::Left) {
                    return Side::Left;
                } else {
                    return Side::Right;
                }
            }
            _ => return side.clone(),
        }
    }

    pub(crate) fn add_participant(
        &mut self,
        user_id: ObjectId,
        side: &Side,
    ) -> Result<bool, PracticeError> {
        if self.is_locked() {
            return Err(PracticeError::Locked);
        }

        let (spots, waitlist) = match side {
            Side::Left => (&mut self.left_side, &mut self.left_side_waitlist),
            Side::Right => (&mut self.right_side, &mut self.right_side_waitlist),
            _ => panic!(),
        };

        if let Some(spot) = spots.iter_mut().find(|spot| spot.is_none()) {
            *spot = Some(user_id);
            return Ok(true);
        }

        if let Some(spot) = waitlist.iter_mut().find(|spot| spot.is_none()) {
            *spot = Some(user_id);
            return Ok(false);
        }

        Err(PracticeError::Full)
    }
}
