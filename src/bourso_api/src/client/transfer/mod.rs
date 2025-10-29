#[cfg(not(tarpaulin_include))]
use crate::account::{Account, AccountKind};
use crate::{client::transfer::error::TransferError, client::BoursoWebClient, constants::BASE_URL};
use anyhow::{bail, Context, Result};
use futures_util::stream::Stream;

mod error;

#[derive(Debug, Clone)]
pub enum TransferProgress {
    Validating,
    InitializingTransfer,
    ExtractingFlowInstance,
    SettingDebitAccount,
    SettingCreditAccount,
    SettingAmount,
    SubmittingStep7,
    SettingReason,
    ConfirmingTransfer,
    Completed,
}

impl TransferProgress {
    #[cfg(not(tarpaulin_include))]
    pub fn step_number(&self) -> u8 {
        match self {
            TransferProgress::Validating => 1,
            TransferProgress::InitializingTransfer => 2,
            TransferProgress::ExtractingFlowInstance => 3,
            TransferProgress::SettingDebitAccount => 4,
            TransferProgress::SettingCreditAccount => 5,
            TransferProgress::SettingAmount => 6,
            TransferProgress::SubmittingStep7 => 7,
            TransferProgress::SettingReason => 8,
            TransferProgress::ConfirmingTransfer => 9,
            TransferProgress::Completed => 10,
        }
    }

    pub fn total_steps() -> u8 {
        10
    }

    #[cfg(not(tarpaulin_include))]
    pub fn description(&self) -> &str {
        match self {
            TransferProgress::Validating => "Validating transfer parameters",
            TransferProgress::InitializingTransfer => "Initializing transfer",
            TransferProgress::ExtractingFlowInstance => "Extracting flow instance",
            TransferProgress::SettingDebitAccount => "Setting debit account",
            TransferProgress::SettingCreditAccount => "Setting credit account",
            TransferProgress::SettingAmount => "Setting transfer amount",
            TransferProgress::SubmittingStep7 => "Submitting intermediate step",
            TransferProgress::SettingReason => "Setting transfer reason",
            TransferProgress::ConfirmingTransfer => "Confirming transfer",
            TransferProgress::Completed => "Transfer completed",
        }
    }
}

impl BoursoWebClient {
    /// Initialize the transfer and extract the transfer ID
    #[cfg(not(tarpaulin_include))]
    async fn init_transfer(&self, from_account: &str) -> Result<String> {
        let init_transfer_url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau",
            BASE_URL, from_account
        );

        let res = self.client.get(&init_transfer_url).send().await?;

        if res.status() != 302 {
            log::debug!("Init transfer response: {:?}", res);
            bail!(TransferError::TransferInitiationFailed);
        }

        let location = res
            .headers()
            .get("location")
            .context("Missing Location header")?
            .to_str()?;

        // /compte/cav/XXXXXXX/virements/immediat/nouveau/YYYYY/1
        // get YYYYY
        let transfer_id = location
            .split('/')
            .nth(7)
            .context("Failed to extract transfer id")?
            .to_string();

        Ok(transfer_id)
    }

    /// Extract the flow instance from the HTML response
    #[cfg(not(tarpaulin_include))]
    async fn extract_flow_instance(&self, url: &str) -> Result<String> {
        let res = self.client.get(url).send().await?;

        if res.status() != 200 {
            log::debug!("First transfer step response: {:?}", res);
            bail!(TransferError::TransferInitiationFailed);
        }

        let res_text = res.text().await?;
        let re = regex::Regex::new(r#"name="flow_ImmediateCashTransfer_instance" value="([^"]+)""#)
            .unwrap();
        let flow_instance = re
            .captures(&res_text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .context("Failed to extract flow instance")?
            .to_string();

        Ok(flow_instance)
    }

    /// Set the debit account (step 2)
    #[cfg(not(tarpaulin_include))]
    async fn set_debit_account(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "1".to_string())
            .text("DebitAccount[debit]", from_account.to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/2",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        if res.status() != 200 {
            log::debug!("Set debit account response: {:?}", res);
            bail!(TransferError::SetDebitAccountFailed);
        }

        Ok(())
    }

    /// Set the credit account (step 3)
    #[cfg(not(tarpaulin_include))]
    async fn set_credit_account(
        &self,
        from_account: &str,
        to_account: &str,
        transfer_id: &str,
        flow_instance: &str,
        transfer_from_banking: bool,
    ) -> Result<()> {
        let form = if transfer_from_banking {
            reqwest::multipart::Form::new().text("CreditAccount[newBeneficiary]", "0".to_string())
        } else {
            reqwest::multipart::Form::new()
        };

        let data = form
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "2".to_string())
            .text("CreditAccount[credit]", to_account.to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/3",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        if res.status() != 200 {
            log::debug!("Set credit account response: {:?}", res);
            bail!(TransferError::SetCreditAccountFailed);
        }

        Ok(())
    }

    /// Set the transfer amount (step 6)
    #[cfg(not(tarpaulin_include))]
    async fn set_transfer_amount(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
        amount: f64,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "5".to_string())
            .text("Amount[amount]", format!("{:.2}", amount).replace('.', ","))
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/6",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        if res.status() != 200 {
            log::debug!("Set amount response: {:?}", res);
            bail!(TransferError::SetAmountFailed);
        }

        Ok(())
    }

    /// Submit step 7
    #[cfg(not(tarpaulin_include))]
    async fn submit_step_7(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "6".to_string())
            .text("submit", "".to_string());

        let res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/7",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        if res.status() != 200 {
            log::debug!("Submit transfer response: {:?}", res);
            bail!(TransferError::Step7Failed);
        }

        Ok(())
    }

    /// Set the transfer reason (step 10)
    #[cfg(not(tarpaulin_include))]
    async fn set_transfer_reason(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
        transfer_reason: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "9".to_string())
            .text("Characteristics[label]", transfer_reason.to_string())
            .text("Characteristics[schedulingType]", "1".to_string()) // 1 = unique
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/10",
            BASE_URL, from_account, transfer_id
        );

        let res = self.client.post(&url).multipart(data).send().await?;

        if res.status() != 200 {
            log::debug!("Set reason response: {:?}", res);
            bail!(TransferError::SetReasonFailed);
        }

        Ok(())
    }

    /// Confirm and finalize the transfer (step 12)
    #[cfg(not(tarpaulin_include))]
    async fn confirm_transfer(
        &self,
        from_account: &str,
        transfer_id: &str,
        flow_instance: &str,
    ) -> Result<()> {
        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "11".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/12",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        if res.status() != 200 {
            log::debug!("Confirm transfer response: {:?}", res);
            bail!(TransferError::SubmitTransferFailed);
        }

        let body = res.text().await?;

        if body.as_str().contains("Confirmation") {
            Ok(())
        } else {
            log::debug!("Cannot find confirmation message in response {:?}", body);
            bail!(TransferError::InvalidTransfer);
        }
    }

    /// Transfer funds from one account to another, yielding progress updates
    ///
    /// ## Arguments
    /// - `amount`: Amount to transfer (must be >= 10.0)
    /// - `from_account`: Source account
    /// - `to_account`: Destination account
    /// - `reason`: Optional reason for the transfer (max 50 characters)
    ///
    /// ## Returns
    /// A stream of progress updates for the transfer.
    #[cfg(not(tarpaulin_include))]
    pub fn transfer_funds(
        &self,
        amount: f64,
        from_account: Account,
        to_account: Account,
        reason: Option<String>,
    ) -> impl Stream<Item = Result<TransferProgress>> + '_ {
        async_stream::stream! {
            // Validation
            yield Ok(TransferProgress::Validating);

            if amount < 10.0 {
                yield Err(TransferError::AmountTooLow.into());
                return;
            }

            log::debug!(
                "Initiating transfer of {:.2} EUR from account {} to account {}",
                amount,
                from_account.id,
                to_account.id
            );

            let transfer_from_banking = from_account.kind == AccountKind::Banking;
            let from_account_id = from_account.id.clone();
            let to_account_id = to_account.id.clone();

            // Default reason if none provided, else use provided reason and
            // warn if the reason is too long (> 50 characters)
            let transfer_reason = if let Some(r) = reason {
                if r.len() > 50 {
                    yield Err(TransferError::ReasonIsTooLong.into());
                    return;
                }
                r
            } else {
                "Virement depuis BoursoBank".to_string()
            };

            // Step 1: Initialize transfer and get transfer ID
            yield Ok(TransferProgress::InitializingTransfer);
            let transfer_id = match self.init_transfer(&from_account_id).await {
                Ok(id) => id,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Extract flow instance
            yield Ok(TransferProgress::ExtractingFlowInstance);
            let flow_instance = match self
                .extract_flow_instance(&format!(
                    "{}/compte/cav/{}/virements/immediat/nouveau/{}/1",
                    BASE_URL, &from_account_id, transfer_id
                ))
                .await {
                Ok(flow) => flow,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Step 2: Set debit account
            yield Ok(TransferProgress::SettingDebitAccount);
            if let Err(e) = self.set_debit_account(&from_account_id, &transfer_id, &flow_instance)
                .await {
                yield Err(e);
                return;
            }

            // Step 3: Set credit account
            yield Ok(TransferProgress::SettingCreditAccount);
            if let Err(e) = self.set_credit_account(
                &from_account_id,
                &to_account_id,
                &transfer_id,
                &flow_instance,
                transfer_from_banking,
            )
            .await {
                yield Err(e);
                return;
            }

            // Step 6: Set amount
            yield Ok(TransferProgress::SettingAmount);
            if let Err(e) = self.set_transfer_amount(&from_account_id, &transfer_id, &flow_instance, amount)
                .await {
                yield Err(e);
                return;
            }

            // Step 7: Submit
            yield Ok(TransferProgress::SubmittingStep7);
            if let Err(e) = self.submit_step_7(&from_account_id, &transfer_id, &flow_instance)
                .await {
                yield Err(e);
                return;
            }

            // Step 10: Set reason
            yield Ok(TransferProgress::SettingReason);
            if let Err(e) = self.set_transfer_reason(
                &from_account_id,
                &transfer_id,
                &flow_instance,
                &transfer_reason,
            )
            .await {
                yield Err(e);
                return;
            }

            // Step 12: Confirm transfer
            yield Ok(TransferProgress::ConfirmingTransfer);
            if let Err(e) = self.confirm_transfer(&from_account_id, &transfer_id, &flow_instance)
                .await {
                yield Err(e);
                return;
            }

            yield Ok(TransferProgress::Completed);
        }
    }
}
