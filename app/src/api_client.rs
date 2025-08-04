use gloo_storage::errors::StorageError;
use reqwest::{RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use shield_models::{
    AuthenticateRequest, AuthenticationResponse, RefreshRequest, SetRecordingModeRequest,
};

use crate::token_store::TokenStore;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub enum ApiError {
    NetworkError,
    Unauthorized,
    BadRequest,
    ServerError,
    TokenMissing,
    StorageError,
}

impl From<reqwest::Error> for ApiError {
    fn from(_: reqwest::Error) -> Self {
        Self::NetworkError
    }
}

impl From<StorageError> for ApiError {
    fn from(_: StorageError) -> Self {
        ApiError::StorageError
    }
}

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    tokens: TokenStore,
    on_unauthorized: Box<dyn Fn() + 'static>,
}

impl ApiClient {
    pub fn new(base_url: String, on_not_authorized: impl Fn() + 'static) -> Result<Self, ApiError> {
        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()?;

        Ok(Self {
            base_url,
            client,
            tokens: TokenStore::new(),
            on_unauthorized: Box::new(on_not_authorized),
        })
    }

    async fn execute_with_auth<T>(&self, request_builder: RequestBuilder) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let token = self
            .tokens
            .get_access_token()
            .ok_or(ApiError::TokenMissing)?;

        let request = request_builder
            .try_clone()
            .unwrap()
            .bearer_auth(&token)
            .build()?;

        let response = self.client.execute(request).await?;

        match response.status() {
            StatusCode::UNAUTHORIZED => self.refresh_token_and_retry(request_builder).await,
            status if status.is_success() => Ok(response.json().await?),
            status => {
                if status.is_client_error() {
                    Err(ApiError::BadRequest)
                } else {
                    Err(ApiError::ServerError)
                }
            }
        }
    }

    async fn refresh_token_and_retry<T>(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        match self.refresh_access_token().await {
            Ok(_) => {
                let new_token = self
                    .tokens
                    .get_access_token()
                    .ok_or(ApiError::TokenMissing)?;
                let retry_request = request_builder.bearer_auth(&new_token).build()?;

                let response = self.client.execute(retry_request).await?;

                if response.status().is_success() {
                    Ok(response.json().await?)
                } else {
                    Err(ApiError::Unauthorized)
                }
            }
            Err(_) => {
                // Refresh failed, clear tokens and notify
                self.tokens.clear_tokens();
                (self.on_unauthorized)();
                Err(ApiError::Unauthorized)
            }
        }
    }

    pub async fn get_cameras(&self) -> Result<Vec<shield_models::Camera>, ApiError> {
        let request = self.client.get(&format!("{}/cameras", self.base_url));
        self.execute_with_auth(request).await
    }

    pub async fn set_recording_mode(
        &self,
        request: SetRecordingModeRequest,
    ) -> Result<(), ApiError> {
        let req = self
            .client
            .post(&format!("{}/set_recording_mode", self.base_url))
            .json(&request);

        self.execute_with_auth::<()>(req).await // Unit type for no response body
    }

    pub async fn authenticate(&self, otp_code: String) -> Result<(), ApiError> {
        let response: AuthenticationResponse = self
            .client
            .post(&format!("{}/authenticate", self.base_url))
            .json(&AuthenticateRequest { otp_code })
            .send()
            .await?
            .json()
            .await?;

        self.tokens
            .set_tokens(response.token.clone(), response.refresh_token.clone())?;

        Ok(())
    }

    async fn refresh_access_token(&self) -> Result<(), ApiError> {
        let response: AuthenticationResponse = self
            .client
            .post(&format!("{}/refresh", self.base_url))
            .json(&RefreshRequest {
                refresh_token: self
                    .tokens
                    .get_refresh_token()
                    .ok_or(ApiError::TokenMissing)?,
            })
            .send()
            .await?
            .json()
            .await?;

        self.tokens
            .set_tokens(response.token.clone(), response.refresh_token.clone())?;

        Ok(())
    }
}
