use orion::errors::UnknownCryptoError;
use std::fmt;

#[derive(Debug)]
pub enum VaultError {
    InvalidCommand,
    InvalidPath,
    IncorrectPassword,
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::InvalidCommand => write!(f, "invalid command"),
            VaultError::InvalidPath => write!(f, "invalid path"),
            VaultError::IncorrectPassword => write!(f, "incorrect password"),
        }
    }
}

impl From<UnknownCryptoError> for VaultError {
    fn from(_err: UnknownCryptoError) -> Self {
        VaultError::IncorrectPassword
    }
}
