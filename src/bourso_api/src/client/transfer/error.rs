use std::fmt;

#[derive(Debug)]
pub enum TransferError {
    AmountTooLow,
    TransferInitiationFailed,
    SetDebitAccountFailed,
    SetCreditAccountFailed,
    Step7Failed,
    SetAmountFailed,
    SetReasonFailed,
    ReasonIsTooLong,
    SubmitTransferFailed,
    InvalidTransfer,
}

impl fmt::Display for TransferError {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransferError::AmountTooLow => {
                write!(f, "Amount is below the minimum threshold (10 EUR)")
            }
            TransferError::ReasonIsTooLong => {
                write!(f, "Transfer reason is too long, max 50 characters")
            }
            TransferError::TransferInitiationFailed => write!(f, "Transfer initiation failed"),
            TransferError::SetDebitAccountFailed => write!(f, "Setting debit account failed"),
            TransferError::SetCreditAccountFailed => write!(f, "Setting credit account failed"),
            TransferError::Step7Failed => write!(f, "Transfer step 7 failed"),
            TransferError::SetAmountFailed => write!(f, "Setting transfer amount failed"),
            TransferError::SetReasonFailed => write!(f, "Setting transfer reason failed"),
            TransferError::SubmitTransferFailed => write!(f, "Submitting transfer failed"),
            TransferError::InvalidTransfer => write!(f, "Invalid transfer. Check that the accounts exist and that you have enough balance. Some accounts (e.g. savings) may not allow transfers to certain other accounts, check first on the website that the transfer is possible."),
        }
    }
}
