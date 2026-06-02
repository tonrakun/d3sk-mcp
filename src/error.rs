use thiserror::Error;

#[derive(Debug, Error)]
pub enum D3skError {
    #[error("E400")]
    BadArgs,
    #[error("E404")]
    NotFound,
    #[error("E408")]
    Timeout,
    #[error("E409")]
    SessionConflict,
    #[error("E500")]
    Internal,
}

impl D3skError {
    pub fn code(&self) -> &'static str {
        match self {
            D3skError::BadArgs => "E400",
            D3skError::NotFound => "E404",
            D3skError::Timeout => "E408",
            D3skError::SessionConflict => "E409",
            D3skError::Internal => "E500",
        }
    }
}
