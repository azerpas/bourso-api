use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ClientError {
    InvalidCredentials,
    MfaRequired,
    InvalidMfa,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::InvalidCredentials => write!(f, "Invalid credentials"),
            ClientError::MfaRequired => write!(f, "MFA required"),
            ClientError::InvalidMfa => write!(f, "Invalid MFA"),
        }
    }
}
