use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthenticateRequest {
    pub otp_code: String,
}

#[derive(Serialize)]
pub struct AuthenticationResponse {
    pub token: String,
}
