pub mod account;
pub mod client;
pub mod constants;
pub mod types;

use crate::client::BoursoWebClient;

pub fn new_client() -> BoursoWebClient {
    BoursoWebClient::new()
}

// pub use features::{
//     list_accounts::list_accounts,
//     place_order::place_order,
//     session::{init_session, login, request_mfa, submit_mfa},
//     transfer_funds::transfer_funds,
// };
