pub mod common;
pub mod pc;
pub mod web;

use std::sync::{Arc, Mutex};
use rmcp::{ServerHandler, tool_router, tool_handler, tool};

pub struct D3skServer {
    pub last_error: Arc<Mutex<Option<String>>>,
}

impl D3skServer {
    pub fn new() -> Self {
        Self {
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_error(&self, msg: impl Into<String>) {
        *self.last_error.lock().unwrap() = Some(msg.into());
    }

    pub fn take_error(&self) -> String {
        self.last_error.lock().unwrap().take().unwrap_or_default()
    }
}

#[tool_router]
impl D3skServer {
    #[tool(description = "Get detail message of the last error")]
    async fn get_last_error(&self) -> String {
        self.take_error()
    }
}

#[tool_handler]
impl ServerHandler for D3skServer {}
