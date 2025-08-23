use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serializer};

pub struct DecimalSerde;

impl DecimalSerde {
    pub fn serialize<S: Serializer>(d: &Decimal, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&d.normalize().to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
        let s = String::deserialize(d)?;
        Decimal::from_str_exact(&s).map_err(serde::de::Error::custom)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal error")]
    Internal,
}

impl From<anyhow::Error> for ApiError { fn from(_: anyhow::Error) -> Self { ApiError::Internal } }

pub trait ResultExt<T> {
    fn or_500(self) -> ApiResult<T>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn or_500(self) -> ApiResult<T> { self.map_err(|_| ApiError::Internal) }
}

