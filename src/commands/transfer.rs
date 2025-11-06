use anyhow::{Context, Result};
use futures_util::{pin_mut, StreamExt};
use std::sync::Arc;
use tracing::info;

use crate::cli::TransferArgs;
use crate::services::AuthService;
use crate::settings::FileSettingsStore;
use bourso_api::client::transfer::TransferProgress;

pub async fn handle(args: TransferArgs) -> Result<()> {
    let settings_store = Arc::new(FileSettingsStore::new()?);
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

    pin_mut!(stream);
    while let Some(progress_result) = stream.next().await {
        let progress = progress_result?;
        let step = progress.step_number();
        let total = TransferProgress::total_steps();
        let percentage = (step as f32 / total as f32 * 100.0) as u8;

        let bar_length = 30usize;
        let filled = (bar_length as f32 * step as f32 / total as f32) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_length - filled);

        print!(
            "\x1B[2K\r[{}] {:3}% - {}/{} - {}",
            bar,
            percentage,
            step,
            total,
            progress.description()
        );
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }
    println!();

    info!(
        "Transfer of {} from account {} to account {} successful ✅",
        amount, from_account.id, to_account.id
    );

    Ok(())
}
