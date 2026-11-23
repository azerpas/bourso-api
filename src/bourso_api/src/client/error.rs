use std::fmt;

#[derive(Debug)]
pub enum ClientError {
    InvalidCredentials,
    MfaRequired,
    QRCodeRequired(String),
    InvalidMfa,
    /// The account grants access to several identities and none was selected.
    /// Carries the identities offered by the website, as `(id, label)`.
    IdentitySelectionRequired(Vec<(String, String)>),
    /// The requested identity is not among those offered by the website.
    UnknownIdentity(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::InvalidCredentials => write!(f, "Invalid credentials"),
            ClientError::MfaRequired => write!(f, "MFA required"),
            ClientError::QRCodeRequired(msg) => write!(f, "{}", msg),
            ClientError::InvalidMfa => write!(f, "Invalid MFA"),
            ClientError::IdentitySelectionRequired(identities) => {
                writeln!(
                    f,
                    "This account grants access to several identities. \
                     Select one with `--identity <id>`:"
                )?;
                for (id, label) in identities {
                    writeln!(f, "  {id}  {label}")?;
                }
                Ok(())
            }
            ClientError::UnknownIdentity(id) => {
                write!(f, "Unknown identity `{id}` for this account")
            }
        }
    }
}
