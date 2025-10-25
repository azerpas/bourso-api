#[cfg(not(tarpaulin_include))]
use crate::account::{Account, AccountKind};
use crate::{client::transfer::error::TransferError, client::BoursoWebClient, constants::BASE_URL};
use anyhow::{bail, Context, Result};

mod error;

impl BoursoWebClient {
    #[cfg(not(tarpaulin_include))]
    pub async fn transfer_funds(
        &self,
        amount: f64,
        from_account: Account,
        to_account: Account,
        reason: Option<&str>,
    ) -> Result<()> {
        // Minimum amount is 10 EUR

        if amount < 10.0 {
            bail!(TransferError::AmountTooLow);
        }

        log::debug!(
            "Initiating transfer of {:.2} EUR from account {} to account {}",
            amount,
            from_account.id,
            to_account.id
        );

        let transfer_from_banking = from_account.kind == AccountKind::Banking;

        let from_account = &from_account.id;
        let to_account = &to_account.id;

        // Default reason if none provided, else use provided reason and
        // warn if the reason is too long (> 50 characters)
        let transfer_reason = if let Some(r) = reason {
            if r.len() > 50 {
                bail!(TransferError::ReasonIsTooLong);
            }
            r.to_string()
        } else {
            "Virement depuis BoursoBank".to_string()
        };

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
            .context("Failed to extract transfer id")?;

        let first_res = self
            .client
            .get(format!("{}{}", BASE_URL, location))
            .send()
            .await?;

        if first_res.status() != 200 {
            log::debug!("First transfer step response: {:?}", first_res);
            bail!(TransferError::TransferInitiationFailed);
        }

        let first_res_text = first_res.text().await?;
        let re = regex::Regex::new(r#"name="flow_ImmediateCashTransfer_instance" value="([^"]+)""#)
            .unwrap();
        let flow_instance = re
            .captures(&first_res_text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .context("Failed to extract flow instance")?;

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

        let second_res = self.client.post(&url).multipart(data).send().await?;

        if second_res.status() != 200 {
            log::debug!("Set debit account response: {:?}", second_res);
            bail!(TransferError::SetDebitAccountFailed);
        }

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

        let third_res = self.client.post(&url).multipart(data).send().await?;

        if third_res.status() != 200 {
            log::debug!("Set credit account response: {:?}", third_res);
            bail!(TransferError::SetCreditAccountFailed);
        }

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

        let set_amount_res = self.client.post(&url).multipart(data).send().await?;

        if set_amount_res.status() != 200 {
            log::debug!("Set amount response: {:?}", set_amount_res);
            bail!(TransferError::SetAmountFailed);
        }

        let data = reqwest::multipart::Form::new()
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "6".to_string())
            .text("submit", "".to_string());

        let submit_res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/7",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        if submit_res.status() != 200 {
            log::debug!("Submit transfer response: {:?}", submit_res);
            bail!(TransferError::Step7Failed);
        }

        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "9".to_string())
            .text("Characteristics[label]", transfer_reason) // Reason for transfer
            .text("Characteristics[schedulingType]", "1".to_string()) // 1 = unique
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let url = format!(
            "{}/compte/cav/{}/virements/immediat/nouveau/{}/10",
            BASE_URL, from_account, transfer_id
        );

        let set_reason_res = self.client.post(&url).multipart(data).send().await?;

        if set_reason_res.status() != 200 {
            log::debug!("Set reason response: {:?}", set_reason_res);
            bail!(TransferError::SetReasonFailed);
        }

        let data = reqwest::multipart::Form::new()
            .text(
                "flow_ImmediateCashTransfer_instance",
                flow_instance.to_string(),
            )
            .text("flow_ImmediateCashTransfer_step", "11".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("flow_ImmediateCashTransfer_transition", "".to_string())
            .text("submit", "".to_string());

        let confirm_res = self
            .client
            .post(format!(
                "{}/compte/cav/{}/virements/immediat/nouveau/{}/12",
                BASE_URL, from_account, transfer_id
            ))
            .multipart(data)
            .send()
            .await?;

        if confirm_res.status() != 200 {
            log::debug!("Confirm transfer response: {:?}", confirm_res);
            bail!(TransferError::SubmitTransferFailed);
        }

        let body = confirm_res.text().await?;

        if body.as_str().contains("Confirmation") {
            Ok(())
        } else {
            log::debug!("Cannot find confirmation message in response {:?}", body);
            bail!(TransferError::InvalidTransfer);
        }
    }
}
