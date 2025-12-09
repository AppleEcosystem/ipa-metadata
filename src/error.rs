use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Info.plist not found in IPA")]
    InfoPlistNotFound,

    #[error("Invalid IPA file: {0}")]
    InvalidIpa(String),

    #[error("PNG normalization failed: {0}")]
    PngNormalization(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, IpaError>;
