use serde::Serialize;

#[derive(Serialize)]
pub struct SignupResponse {
  pub success: bool,
  pub message: String,
  pub on_waitlist: bool
}
