use crate::DB;
use std::sync::Arc;

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
    #[error("User not found")]
    UserNotFound,
    #[error("User has no ID")]
    NoUserId,
    #[error("Database error: {0}")]
    DatabaseError(String),
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

    pub(crate) async fn add_participant(
        &mut self,
        discord_id: &str,
        side: &Side,
        db: Arc<DB>,
    ) -> Result<bool, PracticeError> {
        if self.is_locked() {
            return Err(PracticeError::Locked);
        }

        let user = db
            .get_user_by_discord_id(discord_id)
            .await
            .map_err(|e| PracticeError::DatabaseError(e.to_string()))?
            .ok_or(PracticeError::UserNotFound)?;

        let user_id = user.id.ok_or(PracticeError::NoUserId)?;

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

    pub fn transfer_waitlist(&mut self, prev: &Practice) {
        for waitlist_left in prev.left_side_waitlist.iter().flatten() {
            if let Some(empty_spot) = self.left_side.iter_mut().find(|spot| spot.is_none()) {
                *empty_spot = Some(*waitlist_left);
            } else {
                panic!();
            }
        }

        for waitlist_right in prev.right_side_waitlist.iter().flatten() {
            if let Some(empty_spot) = self.right_side.iter_mut().find(|spot| spot.is_none()) {
                *empty_spot = Some(*waitlist_right);
            } else {
                panic!();
            }
        }
    }

    pub fn is_future(&self) -> bool {
        self.start_time > Utc::now()
    }

    pub async fn remove_participant(
        &mut self,
        discord_id: &str,
        db: Arc<DB>,
    ) -> Result<Option<ObjectId>, PracticeError> {
        let user = db
            .get_user_by_discord_id(discord_id)
            .await
            .map_err(|e| PracticeError::DatabaseError(e.to_string()))?
            .ok_or(PracticeError::UserNotFound)?;

        let user_id = user.id.ok_or(PracticeError::NoUserId)?;

        // Check left side main list
        if let Some(pos) = self
            .left_side
            .iter()
            .position(|id| id.as_ref() == Some(&user_id))
        {
            self.left_side[pos] = None;

            // Check left waitlist for replacement
            if let Some(waitlist_pos) = self.left_side_waitlist.iter().position(|id| id.is_some()) {
                let waitlist_user = self.left_side_waitlist[waitlist_pos].take();
                self.left_side[pos] = waitlist_user;
                return Ok(waitlist_user);
            }
            return Ok(None);
        }

        // Check right side main list
        if let Some(pos) = self
            .right_side
            .iter()
            .position(|id| id.as_ref() == Some(&user_id))
        {
            self.right_side[pos] = None;

            // Check right waitlist for replacement
            if let Some(waitlist_pos) = self.right_side_waitlist.iter().position(|id| id.is_some())
            {
                let waitlist_user = self.right_side_waitlist[waitlist_pos].take();
                self.right_side[pos] = waitlist_user;
                return Ok(waitlist_user);
            }
            return Ok(None);
        }

        // Check left waitlist
        if let Some(pos) = self
            .left_side_waitlist
            .iter()
            .position(|id| id.as_ref() == Some(&user_id))
        {
            self.left_side_waitlist[pos] = None;
            return Ok(None);
        }

        // Check right waitlist
        if let Some(pos) = self
            .right_side_waitlist
            .iter()
            .position(|id| id.as_ref() == Some(&user_id))
        {
            self.right_side_waitlist[pos] = None;
            return Ok(None);
        }

        Err(PracticeError::UserNotFound)
    }
}
