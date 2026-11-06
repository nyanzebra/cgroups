use std::num::ParseIntError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("quantity is empty")]
    QuantityEmpty,

    #[error("invalid quantity: '{0}'")]
    InvalidQuantity(String),

    #[error("failed to parse quantity: '{0}'")]
    ParseQuantity(#[from] ParseIntError),

    #[error("io error: '{0}'")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
