use anyhow::Error;
use hidapi::HidError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, GamepadError>;

#[derive(Error, Debug)]
pub enum GamepadError {
    #[error("communication error")]
    Hidapi(#[from] HidError),
    #[error("other error")]
    Anyhow(#[from] Error),
}
