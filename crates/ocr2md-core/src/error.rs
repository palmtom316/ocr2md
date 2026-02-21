use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unsupported input file type: {0}")]
    UnsupportedInputType(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("API call failed with status {status}: {message}")]
    ApiStatus { status: u16, message: String },

    #[error("API response parse error: {0}")]
    ApiResponse(String),
}
