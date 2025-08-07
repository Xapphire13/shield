use std::{cell::RefCell, rc::Rc};

use gloo_storage::{LocalStorage, Storage, errors::StorageError};

#[derive(Clone)]
pub struct TokenStore {
    access_token: Rc<RefCell<Option<String>>>,
    refresh_token: Rc<RefCell<Option<String>>>,
}

impl TokenStore {
    pub fn new() -> Self {
        let access_token = LocalStorage::get("api_token").ok();
        let refresh_token = LocalStorage::get("refresh_token").ok();

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
        LocalStorage::set("api_token", access.clone())?;
        LocalStorage::set("refresh_token", refresh.clone())?;

        self.access_token.replace(Some(access));
        self.refresh_token.replace(Some(refresh));

        Ok(())
    }

    pub fn clear_tokens(&self) {
        LocalStorage::delete("api_token");
        LocalStorage::delete("refresh_token");

        self.access_token.replace(None);
        self.refresh_token.replace(None);
    }
}
