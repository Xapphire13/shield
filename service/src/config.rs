use std::{fs, path::PathBuf};

use anyhow::Result;
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use totp_rs::Secret;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub credentials: CredentialsConfig,
    pub otp: Option<OtpConfig>,
    pub jwt: Option<JwtConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct CredentialsConfig {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct OtpConfig {
    pub secret: String,
}

#[derive(Deserialize, Serialize)]
pub struct JwtConfig {
    pub secret: String,
}

impl OtpConfig {
    pub fn new() -> OtpConfig {
        let secret = Secret::generate_secret();
        OtpConfig {
            secret: secret.to_encoded().to_string(),
        }
    }
}

impl JwtConfig {
    pub fn new() -> JwtConfig {
        JwtConfig {
            secret: rand::rng()
                .sample_iter(Alphanumeric)
                .take(20)
                .map(char::from)
                .collect(),
        }
    }
}

#[cfg(debug_assertions)]
fn get_config_path() -> PathBuf {
    use std::env::current_dir;

    let mut current_dir = current_dir().expect("Can't load current directory");
    current_dir.push("shield.config.toml");

    current_dir
}

#[cfg(not(debug_assertions))]
fn get_config_path() -> PathBuf {
    use std::env::home_dir;

    let mut home_dir = home_dir().expect("Can't load home directory");
    home_dir.push("shield.config.toml");

    home_dir
}

impl Config {
    pub fn load() -> Self {
        let config_path = get_config_path();
        let credentials_file = fs::read_to_string(config_path)
            .expect("Couldn't find shield.config.toml in home directory");

        toml::from_str(&credentials_file).expect("Couldn't deserialize shield.config.toml")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path();
        fs::write(config_path, toml::to_string_pretty(self)?)?;

        Ok(())
    }
}
