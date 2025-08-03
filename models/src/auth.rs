use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct AuthenticateRequest {
    pub otp_code: String,
}

pub type AuthToken = String;

#[derive(Deserialize, Serialize)]
pub struct AuthenticationResponse {
    pub token: AuthToken,
    pub refresh_token: String,
}

#[derive(Deserialize, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}
