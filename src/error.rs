use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkyTabError {
    #[error("missing SKYTAB_USERNAME or SKYTAB_PASSWORD")]
    MissingCredentials,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("toml deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("api error {status}: {body}")]
    Api { status: u16, body: String },

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, SkyTabError>;
