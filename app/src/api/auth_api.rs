use shield_models::{AuthenticateRequest, AuthenticationResponse};

use crate::{api::ApiClient, types::ApiError};

pub trait AuthApi {
    async fn authenticate(&self, otp_code: String) -> Result<(), ApiError>;
}

impl AuthApi for ApiClient {
    async fn authenticate(&self, otp_code: String) -> Result<(), ApiError> {
        let response: AuthenticationResponse = self
            .post("/authenticate")
            .json(&AuthenticateRequest { otp_code })
            .send()
            .await?
            .json()
            .await?;

        self.set_tokens(response.token.clone(), response.refresh_token.clone())?;

        Ok(())
    }
}
