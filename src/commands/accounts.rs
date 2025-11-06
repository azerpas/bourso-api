use anyhow::Result;
use tracing::info;

use crate::cli::AccountsArgs;
use crate::services::AuthService;
use crate::settings::FileSettingsStore;

use bourso_api::account::{Account, AccountKind};

pub async fn handle(args: AccountsArgs) -> Result<()> {
    let settings_store = Box::new(FileSettingsStore::new()?);
    let auth_service = AuthService::with_defaults(settings_store);

    let Some(client) = auth_service.login().await? else {
        return Ok(());
    };

    let kind = if args.banking {
        Some(AccountKind::Banking)
    } else if args.saving {
        Some(AccountKind::Savings)
    } else if args.trading {
        Some(AccountKind::Trading)
    } else if args.loans {
        Some(AccountKind::Loans)
    } else {
        None
    };

    let accounts: Vec<Account> = client.get_accounts(kind).await?;
    info!("Found {} accounts", accounts.len());
    println!("{:#?}", accounts);
    Ok(())
}
