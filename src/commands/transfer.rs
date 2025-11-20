use anyhow::{Context, Result};
use futures_util::{pin_mut, StreamExt};
use tracing::info;

use crate::{cli::TransferArgs, services::AuthService, ux::progress::TextProgressBar, AppCtx};

use bourso_api::client::transfer::TransferProgress;

pub async fn handle(args: TransferArgs, ctx: &AppCtx) -> Result<()> {
    let auth_service = AuthService::with_defaults(ctx.settings_store.as_ref());

    let Some(client) = auth_service.login().await? else {
        return Ok(());
    };

    let accounts = client.get_accounts(None).await?;

    let from_account = accounts
        .iter()
        .find(|a| a.id == args.from_account.as_ref().as_str()) // TODO: compare AccountId instead of String
        .context("From account not found. Are you sure you have access to it? Run `bourso-cli accounts` to list your accounts")?;

    let to_account = accounts
        .iter()
        .find(|a| a.id == args.to_account.as_ref().as_str()) // TODO: compare AccountId instead of String
        .context("To account not found. Are you sure you have access to it? Run `bourso-cli accounts` to list your accounts")?;

    let stream = client.transfer_funds(
        args.amount.get(),
        from_account.clone(),
        to_account.clone(),
        args.reason.map(|r| r.as_ref().to_string()),
    );

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
        args.amount.get(),
        from_account.id,
        to_account.id
    );

    Ok(())
}
