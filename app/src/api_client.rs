use std::cell::RefCell;

use anyhow::{Result, anyhow};
use dioxus::logger::tracing;
use gloo_storage::{LocalStorage, Storage};
use reqwest::StatusCode;
use serde::{Serialize, de::DeserializeOwned};
use shield_models::{
    AuthenticateRequest, AuthenticationResponse, RefreshRequest, SetRecordingModeRequest,
};

pub struct ApiClient {
    token: RefCell<Option<String>>,
    refresh_token: RefCell<Option<String>>,
    on_not_authorized: Box<dyn Fn() + 'static>,
}

impl ApiClient {
    pub fn new(on_not_authorized: impl Fn() + 'static) -> Result<Self> {
        let token = LocalStorage::get("api_token").map_or(None, Some);
        let refresh_token = LocalStorage::get("refresh_token").map_or(None, Some);

        Ok(Self {
            token: RefCell::new(token),
            refresh_token: RefCell::new(refresh_token),
            on_not_authorized: Box::new(on_not_authorized),
        })
    }

    async fn make_get_request<TResponse: DeserializeOwned>(&self, path: &str) -> Result<TResponse> {
        let res = reqwest::Client::new()
            .get(path)
            .bearer_auth(self.token.borrow().clone().unwrap_or(String::new()))
            .send()
            .await?;

        if res.status() == StatusCode::UNAUTHORIZED {
            tracing::info!("Unauthorized");

            match self.refresh_token().await {
                Ok(_) => return Box::pin(self.make_get_request::<TResponse>(path)).await,
                Err(_) => self.on_not_authorized.as_ref()(),
            }
        }

        Ok(res.json::<TResponse>().await?)
    }

    async fn make_post_request<TRequest: Serialize>(
        &self,
        path: &str,
        request: &TRequest,
    ) -> Result<()> {
        let response = reqwest::Client::new()
            .post(path)
            .bearer_auth(self.token.borrow().clone().unwrap_or(String::new()))
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "API request failed with status {}",
                response.status()
            ));
        }

        Ok(())
    }

    pub async fn get_cameras(&self) -> Result<Vec<shield_models::Camera>> {
        self.make_get_request(&get_api_url("/cameras")).await
    }

    pub async fn set_recording_mode(&self, request: SetRecordingModeRequest) -> Result<()> {
        self.make_post_request(&get_api_url("/set_recording_mode"), &request)
            .await
    }

    pub async fn authenticate(&self, otp_code: String) -> Result<()> {
        let response: AuthenticationResponse = reqwest::Client::new()
            .post(&get_api_url("/authenticate"))
            .json(&AuthenticateRequest { otp_code })
            .send()
            .await?
            .json()
            .await?;

        LocalStorage::set("api_token", response.token.clone())?;
        LocalStorage::set("refresh_token", response.refresh_token.clone())?;

        self.token.replace(Some(response.token));
        self.refresh_token.replace(Some(response.refresh_token));

        Ok(())
    }

    async fn refresh_token(&self) -> Result<()> {
        let response: AuthenticationResponse = reqwest::Client::new()
            .post(&get_api_url("/refresh"))
            .json(&RefreshRequest {
                refresh_token: self
                    .refresh_token
                    .borrow()
                    .clone()
                    .ok_or(anyhow!("Refresh token not found"))?,
            })
            .send()
            .await?
            .json()
            .await?;

        LocalStorage::set("api_token", response.token.clone())?;
        LocalStorage::set("refresh_token", response.refresh_token.clone())?;

        self.token.replace(Some(response.token));
        self.refresh_token.replace(Some(response.refresh_token));

        Ok(())
    }
}

pub fn get_api_url(path: &str) -> String {
    let hostname = web_sys::window().unwrap().location().hostname().unwrap();

    format!("http://{hostname}:3000{path}")
}
