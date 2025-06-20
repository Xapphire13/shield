use std::fs;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn load() -> Self {
        let credentials_file =
            fs::read_to_string(".credentials.toml").expect("Couldn't find .credentials.toml");

        toml::from_str(&credentials_file).expect("Couldn't deserialize .credentials.toml")
    }
}
