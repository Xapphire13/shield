use gloo_storage::{LocalStorage, Storage, errors::StorageError};
use std::{cell::RefCell, rc::Rc};

pub const ACCESS_TOKEN_KEY: &str = "api_token";
pub const REFRESH_TOKEN_KEY: &str = "refresh_token";

#[derive(Clone)]
pub struct TokenStore {
    access_token: Rc<RefCell<Option<String>>>,
    refresh_token: Rc<RefCell<Option<String>>>,
}

impl TokenStore {
    pub fn new() -> Self {
        let access_token = LocalStorage::get(ACCESS_TOKEN_KEY).ok();
        let refresh_token = LocalStorage::get(REFRESH_TOKEN_KEY).ok();

        Self {
            access_token: Rc::new(RefCell::new(access_token)),
            refresh_token: Rc::new(RefCell::new(refresh_token)),
        }
    }

    pub fn get_access_token(&self) -> Option<String> {
        self.access_token.borrow().clone()
    }

    pub fn get_refresh_token(&self) -> Option<String> {
        self.refresh_token.borrow().clone()
    }

    pub fn set_tokens(&self, access: String, refresh: String) -> Result<(), StorageError> {
        LocalStorage::set(ACCESS_TOKEN_KEY, access.clone())?;
        LocalStorage::set(REFRESH_TOKEN_KEY, refresh.clone())?;

        self.access_token.replace(Some(access));
        self.refresh_token.replace(Some(refresh));

        Ok(())
    }

    pub fn clear_tokens(&self) {
        LocalStorage::delete(ACCESS_TOKEN_KEY);
        LocalStorage::delete(REFRESH_TOKEN_KEY);

        self.access_token.replace(None);
        self.refresh_token.replace(None);
    }
}
