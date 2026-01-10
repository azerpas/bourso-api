use std::fmt;

#[derive(Debug)]
pub enum ClientError {
    InvalidCredentials,
    MfaRequired,
    QRCodeRequired(String),
    InvalidMfa,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::InvalidCredentials => write!(f, "Invalid credentials"),
            ClientError::MfaRequired => write!(f, "MFA required"),
            ClientError::QRCodeRequired(msg) => write!(f, "{}", msg),
            ClientError::InvalidMfa => write!(f, "Invalid MFA"),
        }
    }
}
