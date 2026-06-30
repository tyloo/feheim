//! Domain errors with user-facing messages.

use std::fmt;

#[derive(Debug)]
pub enum FeheimError {
    /// Formula name absent from the index.
    NotFound(String),
    /// Formula not present in the Cellar.
    NotInstalled(String),
}

impl fmt::Display for FeheimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeheimError::NotFound(name) => {
                write!(f, "no formula named \"{name}\" (try `feheim update`)")
            }
            FeheimError::NotInstalled(name) => {
                write!(f, "\"{name}\" is not installed")
            }
        }
    }
}

impl std::error::Error for FeheimError {}
