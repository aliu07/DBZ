use serde::{Deserialize, Serialize};
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use std::error::Error;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct FormResponse {
    pub email_address: String,
    pub full_name: String,
    pub mcgill_id: String,
    pub preferred_email: String,
    pub paddle_side: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PracticeSheetData {
    pub date: DateTime<Utc>,
    pub left_side: Vec<Option<String>>,
    pub right_side: Vec<Option<String>>,
    pub left_waitlist: Vec<Option<String>>,
    pub right_waitlist: Vec<Option<String>>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SheetMetaData {
  #[serde(rename = "_id")]
  pub sheet_id: String,
  pub last_processed_row: usize
}

impl PracticeSheetData {
  pub fn parse_from_rows(rows: Vec<Vec<String>>) -> Result<Self, Box<dyn Error>> {
      if rows.is_empty() {
          return Err("Empty sheet data".into());
      }

      tracing::info!("Parsing sheet with {} rows", rows.len());

      // Parse the date from the first row (which is a single-element vector)
      let date_str = &rows[0][0];
      tracing::info!("{:?}", date_str);
      let date = Self::parse_practice_date(date_str)?;

      let mut left_side = Vec::new();
      let mut right_side = Vec::new();
      let mut left_waitlist = Vec::new();
      let mut right_waitlist = Vec::new();

      let mut in_main_list = false;
      let mut in_waitlist = false;

      // Iterate through rows
      for row in rows.iter() {
          // Skip empty rows
          if row.is_empty() {
              continue;
          }

          // Check for section markers
          if row.get(1).map_or(false, |cell| cell == "LEFTIES") {
              tracing::info!("Found main list section");
              in_main_list = true;
              in_waitlist = false;
              continue;
          }

          if row.get(1).map_or(false, |cell| cell.contains("WAITLIST")) {
              tracing::info!("Found waitlist section");
              in_main_list = false;
              in_waitlist = true;
              continue;
          }

          if row.get(1).map_or(false, |cell| cell.contains("DO NOT SIGN UP")) {
              in_main_list = false;
              in_waitlist = false;
              continue;
          }

          // Process main list
          if in_main_list {
              // Skip header row with "First Name, Last Name"
              if row.get(1).map_or(false, |cell| cell == "First Name") {
                  continue;
              }

              // Only process numbered rows (1-17)
              if let Some(first_cell) = row.get(0) {
                  if first_cell.parse::<u32>().is_ok() {
                      // Get left side entry
                      let left_entry = match (row.get(1), row.get(2)) {
                          (Some(first), Some(last)) if !first.is_empty() && !last.is_empty() => {
                              Some(format!("{} {}", first.trim(), last.trim()))
                          }
                          _ => None
                      };
                      left_side.push(left_entry);

                      // Get right side entry
                      let right_entry = match (row.get(5), row.get(6)) {
                          (Some(first), Some(last)) if !first.is_empty() && !last.is_empty() => {
                              Some(format!("{} {}", first.trim(), last.trim()))
                          }
                          _ => None
                      };
                      right_side.push(right_entry);
                  }
              }
          }

          // Process waitlist
          if in_waitlist {
              // Only process numbered rows (1-6)
              if let Some(first_cell) = row.get(0) {
                  if first_cell.parse::<u32>().is_ok() {
                      // Get left side waitlist entry
                      let left_entry = match (row.get(1), row.get(2)) {
                          (Some(first), Some(last)) if !first.is_empty() && !last.is_empty() => {
                              Some(format!("{} {}", first.trim(), last.trim()))
                          }
                          _ => None
                      };
                      left_waitlist.push(left_entry);

                      // Get right side waitlist entry
                      let right_entry = match (row.get(5), row.get(6)) {
                          (Some(first), Some(last)) if !first.is_empty() && !last.is_empty() => {
                              Some(format!("{} {}", first.trim(), last.trim()))
                          }
                          _ => None
                      };
                      right_waitlist.push(right_entry);
                  }
              }
          }
      }

      // Ensure lists have the correct length
      while left_side.len() < 17 { left_side.push(None); }
      while right_side.len() < 17 { right_side.push(None); }
      while left_waitlist.len() < 6 { left_waitlist.push(None); }
      while right_waitlist.len() < 6 { right_waitlist.push(None); }

      tracing::info!("Successfully parsed sheet data");
      tracing::debug!("Left side entries: {}", left_side.len());
      tracing::debug!("Right side entries: {}", right_side.len());
      tracing::debug!("Left waitlist entries: {}", left_waitlist.len());
      tracing::debug!("Right waitlist entries: {}", right_waitlist.len());

      Ok(Self {
          date,
          left_side,
          right_side,
          left_waitlist,
          right_waitlist,
      })
  }

  fn parse_practice_date(date_str: &str) -> Result<DateTime<Utc>, Box<dyn Error>> {
      // Remove quotes and trim whitespace
      let date_str = date_str.trim_matches('"').trim();

      tracing::info!("Parsing date string: {}", date_str);

      // Expected format: "Thursday, November 28 (7:00 PM)"
      let mut parts = date_str.rsplitn(2, '(');

      // Get time part: "7:00 PM)"
      let time_part = parts.next()
          .ok_or("Missing time part")?
          .trim()
          .trim_end_matches(')')
          .trim();

      // Get date part: "Thursday, November 28"
      let date_part = parts.next()
          .ok_or("Missing date part")?
          .trim()
          .trim_end_matches(',');

      // Split date part into components
      let date_components: Vec<&str> = date_part.split(',')
          .map(|s| s.trim())
          .collect();

      let month_day = date_components.last()
          .ok_or("Missing month and day")?;

      // Combine all parts into a format chrono can parse
      // Add the current year since it's not in the input
      let current_year = Utc::now().year();
      let datetime_str = format!("{} {} {}", month_day, time_part, current_year);

      tracing::info!("Formatted datetime string: {}", datetime_str);

      // Parse using a simpler format string
      let naive_dt = NaiveDateTime::parse_from_str(&datetime_str, "%B %d %I:%M %p %Y")?;

      // Convert to UTC
      Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc))
  }
}
