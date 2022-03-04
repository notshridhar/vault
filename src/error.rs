use orion::errors::UnknownCryptoError;
use std::fmt;

#[derive(Debug)]
pub enum VaultError {
    InvalidCommand,
    IncorrectPassword,
    CrcMismatch(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::InvalidCommand => write!(f, "invalid command"),
            VaultError::IncorrectPassword => write!(f, "incorrect password"),
            VaultError::CrcMismatch(e) => write!(f, "crc mismatch - {}", &e),
        }
    }
}

impl From<UnknownCryptoError> for VaultError {
    fn from(_err: UnknownCryptoError) -> Self {
        VaultError::IncorrectPassword
    }
}
