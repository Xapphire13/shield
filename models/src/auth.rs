use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthenticateRequest {
    pub otp_code: String,
}

pub type AuthToken = String;

#[derive(Serialize)]
pub struct AuthenticationResponse {
    pub token: AuthToken,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}
