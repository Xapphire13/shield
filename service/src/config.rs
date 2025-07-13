use std::{fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub credentials: Credentials,
}

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
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
}
