use anyhow::{Context, Result};
use futures_util::{pin_mut, StreamExt};
use tracing::info;

use crate::cli::TransferArgs;
use crate::services::AuthService;
use crate::settings::FileSettingsStore;
use crate::ux::progress::TextProgressBar;

use bourso_api::client::transfer::TransferProgress;

pub async fn handle(args: TransferArgs) -> Result<()> {
    let settings_store = Box::new(FileSettingsStore::new()?);
    let auth_service = AuthService::with_defaults(settings_store);

    let Some(client) = auth_service.login().await? else {
        return Ok(());
    };

    let from_account_id = args.from_account;
    let to_account_id = args.to_account;
    let amount: f64 = args.amount.parse()?;
    let reason = args.reason;

    let accounts = client.get_accounts(None).await?;
    let from_account = accounts
        .iter()
        .find(|a| a.id == from_account_id)
        .context("From account not found. Are you sure you have access to it? Run `bourso accounts` to list your accounts")?;
    let to_account = accounts
        .iter()
        .find(|a| a.id == to_account_id)
        .context("To account not found. Are you sure you have access to it? Run `bourso accounts` to list your accounts")?;

    let stream = client.transfer_funds(amount, from_account.clone(), to_account.clone(), reason);

    let bar = TextProgressBar::new(30usize);
    pin_mut!(stream);
    while let Some(progress_result) = stream.next().await {
        let progress = progress_result?;
        let step = progress.step_number() as usize;
        let total = TransferProgress::total_steps() as usize;

        bar.render(step, total, progress.description());
    }
    bar.finish();

    info!(
        "Transfer of {} from account {} to account {} successful ✅",
        amount, from_account.id, to_account.id
    );

    Ok(())
}
