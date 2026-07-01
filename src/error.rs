use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("api request {0}")]
    ApiRequest(#[from] reqwest::Error),
    #[error("serde json {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("response {0}")]
    Response(String),
    #[error("http {0}")]
    Http(#[from] http::Error),
    #[error("http invalidheadername {0}")]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),
    #[error("http invalidheadervalue {0}")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error("general {0}")]
    General(String),
}
