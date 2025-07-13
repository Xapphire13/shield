use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthenticateInput {
    pub otp_code: String,
}

#[derive(Serialize)]
pub struct AuthenticationResult {
    pub token: String,
}
