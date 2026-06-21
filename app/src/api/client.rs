use reqwest::{RequestBuilder, Response, StatusCode};
use shield_models::{AuthenticationResponse, RefreshRequest};

use crate::{storage::TokenStore, types::ApiError};

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    storage: TokenStore,
    on_unauthorized: Box<dyn Fn() + 'static>,
}

impl ApiClient {
    pub fn new(base_url: String, on_unauthorized: impl Fn() + 'static) -> Self {
        // NOTE: don't set a custom User-Agent here. It's a forbidden header in
        // the browser Fetch spec, so doing so triggers a CORS preflight that
        // lists `user-agent` in Access-Control-Request-Headers and gets
        // rejected by the server. The browser sets User-Agent itself anyway.
        let client = reqwest::Client::new();

        Self {
            base_url,
            client,
            storage: TokenStore::new(),
            on_unauthorized: Box::new(on_unauthorized),
        }
    }

    pub async fn execute_with_auth(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<Response, ApiError> {
        let token = self.storage.get_access_token().ok_or_else(|| {
            (self.on_unauthorized)();
            ApiError::TokenMissing
        })?;

        let request = request_builder
            .try_clone()
            .expect("Couldn't clone request, is the URL malformed?")
            .bearer_auth(&token)
            .build()?;

        let response = self.client.execute(request).await?;

        match response.status() {
            StatusCode::UNAUTHORIZED => self.refresh_token_and_retry(request_builder).await,
            status if status.is_success() => Ok(response),
            status => Err(ApiError::from(status)),
        }
    }

    async fn refresh_token_and_retry(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<Response, ApiError> {
        match self.refresh_access_token_internal().await {
            Ok(_) => {
                let new_token = self
                    .storage
                    .get_access_token()
                    .ok_or(ApiError::TokenMissing)?;
                let retry_request = request_builder.bearer_auth(&new_token).build()?;

                let response = self.client.execute(retry_request).await?;

                if response.status().is_success() {
                    Ok(response)
                } else {
                    Err(ApiError::Unauthorized)
                }
            }
            Err(_) => {
                // Refresh failed, clear tokens and notify
                self.storage.clear_tokens();
                (self.on_unauthorized)();
                Err(ApiError::Unauthorized)
            }
        }
    }

    async fn refresh_access_token_internal(&self) -> Result<(), ApiError> {
        let response: AuthenticationResponse = self
            .client
            .post(format!("{}/refresh", self.base_url))
            .json(&RefreshRequest {
                refresh_token: self
                    .storage
                    .get_refresh_token()
                    .ok_or(ApiError::TokenMissing)?,
            })
            .send()
            .await?
            .json()
            .await?;

        self.storage
            .set_tokens(response.token.clone(), response.refresh_token.clone())?;

        Ok(())
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(format!("{}{url}", self.base_url))
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(format!("{}{url}", self.base_url))
    }

    pub fn set_tokens(&self, access_token: String, refresh_token: String) -> Result<(), ApiError> {
        self.storage.set_tokens(access_token, refresh_token)?;

        Ok(())
    }
}
