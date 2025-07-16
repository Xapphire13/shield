use std::ops::Deref;

use anyhow::{Result, anyhow};
use chrono::{Days, Utc};
use postcard::{from_bytes, to_allocvec};
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::info;

use crate::app_error::AppError;

pub struct RefreshTokenStore {
    db: Db,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshToken {
    pub token: String,
    /// Timestamp of token expiry
    expires: i64,
}

impl RefreshTokenStore {
    pub fn new() -> RefreshTokenStore {
        let db = sled::open("db/refresh-tokens").expect("Failed to open database");

        RefreshTokenStore { db }
    }

    pub fn generate_new_token(&self) -> Result<RefreshToken> {
        let token = RefreshToken {
            token: rand::rng()
                .sample_iter(Alphanumeric)
                .take(32) // 256-bit secret
                .map(char::from)
                .collect(),
            expires: Utc::now()
                .checked_add_days(Days::new(30))
                .ok_or(anyhow!("Couldn't create refresh token"))?
                .timestamp(),
        };

        info!("Created refresh token: {token:?}");
        self.db.insert(&token.token, to_allocvec(&token)?)?;

        Ok(token)
    }

    pub fn validate_token(&self, token: String) -> core::result::Result<(), AppError> {
        let record = self
            .db
            .remove(token)? // We remove because a refresh token is invalid as soon as it's used
            .ok_or(AppError::InvalidRefreshToken)?;
        let token: RefreshToken = from_bytes(record.deref())?;

        if token.expires <= Utc::now().timestamp() {
            Err(AppError::ExpiredRefreshToken)
        } else {
            Ok(())
        }
    }
}
