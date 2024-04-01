pub fn validate_account_id(account_id: &str) -> Result<String, String> {
    if account_id.trim().len() == 32 {
        Ok(account_id.to_owned())
    } else {
        Err(String::from("Account id must be 32 characters long"))
    } 
}
