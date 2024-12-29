use derive_more::{Display, Error};
use miette::Diagnostic;

/// Config errors from [`voidconf`].
#[derive(Clone, Display, Debug, Error, Diagnostic, PartialEq, Eq)]
pub enum ConfError {
    /// Expected key is not defined.
    #[error]
    #[display("expected key not found: {key}")]
    #[diagnostic()]
    KeyNotFound { key: String },

    /// Required value is not defined for a defined key.
    #[error]
    #[display("expected val not found with key: {key}")]
    #[diagnostic()]
    ValNotFound { key: String },

    //TODO extract data from parse error
    /// Type-safe value parsing failed.
    #[error]
    #[display("failed to parse val as given type: {key} = {val}")]
    #[diagnostic()]
    ValParseFailed { key: String, val: String },

    /// Environment variable lookup failed.
    #[error]
    #[display("failed to lookup env var: {key}")]
    #[diagnostic()]
    EnvLookupFailed {
        key: String,
        #[error(source)]
        source: std::env::VarError,
    },
}

impl ConfError {
    pub fn key_not_found(key: impl Into<String>) -> Self {
        Self::KeyNotFound { key: key.into() }
    }

    pub fn val_not_found(key: impl Into<String>) -> Self {
        Self::ValNotFound { key: key.into() }
    }

    pub fn val_parse_failed(key: impl Into<String>, val: impl Into<String>) -> Self {
        Self::ValParseFailed {
            key: key.into(),
            val: val.into(),
        }
    }

    pub fn env_lookup_failed(key: impl Into<String>, source: std::env::VarError) -> Self {
        Self::EnvLookupFailed {
            key: key.into(),
            source,
        }
    }
}
