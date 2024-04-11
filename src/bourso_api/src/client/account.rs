use crate::{
    account::{AccountKind, Account},
    constants::{BASE_URL, SAVINGS_PATTERN, BANKING_PATTERN, TRADING_PATTERN, LOANS_PATTERN, ACCOUNT_PATTERN}
};

use super::BoursoWebClient;

use anyhow::{Context, Result};
use log::debug;
use regex::Regex;

impl BoursoWebClient {
    /// Get the accounts list.
    /// 
    /// # Arguments
    /// 
    /// * `kind` - The type of accounts to retrieve. If `None`, all accounts are retrieved.
    /// 
    /// # Returns
    /// 
    /// The accounts list as a vector of `Account`.
    pub async fn get_accounts(&self, kind: Option<AccountKind>) -> Result<Vec<Account>> {
        let res = self.client
            .get(format!("{BASE_URL}/dashboard/liste-comptes?rumroute=dashboard.new_accounts&_hinclude=1"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        let accounts = match kind {
            Some(AccountKind::Savings) => extract_accounts(&res, AccountKind::Savings)?,
            Some(AccountKind::Banking) => extract_accounts(&res, AccountKind::Banking)?,
            Some(AccountKind::Trading) => extract_accounts(&res, AccountKind::Trading)?,
            Some(AccountKind::Loans) => extract_accounts(&res, AccountKind::Loans)?,
            // all accounts
            _ => {
                [
                    extract_accounts(&res, AccountKind::Savings).unwrap_or(Vec::new()),
                    extract_accounts(&res, AccountKind::Banking).unwrap_or(Vec::new()),
                    extract_accounts(&res, AccountKind::Trading).unwrap_or(Vec::new()),
                    extract_accounts(&res, AccountKind::Loans).unwrap_or(Vec::new()),
                ].concat()
            },
        };

        Ok(accounts)
    }
}

fn extract_accounts(res: &str, kind: AccountKind) -> Result<Vec<Account>> {
    let regex = Regex::new(
        match kind {
            AccountKind::Savings => SAVINGS_PATTERN,
            AccountKind::Banking => BANKING_PATTERN,
            AccountKind::Trading => TRADING_PATTERN,
            AccountKind::Loans => LOANS_PATTERN,
        }
    )?;
    let accounts_ul = regex
        .captures(&res)
        .with_context(|| {
            debug!("Response: {}", res);
            format!("Failed to extract {:?} accounts from the response", kind)
        })?
        .get(1)
        .context("Failed to extract accounts from regex match")?
        .as_str();

    let account_regex = Regex::new(ACCOUNT_PATTERN)?;

    let accounts = account_regex
        .captures_iter(&accounts_ul)
        .map(|m| {
            Account {
                id: m.name("id")
                    .unwrap()
                    .as_str()
                    .trim()
                    .to_string(),
                name: m.name("name")
                    .unwrap()
                    .as_str()
                    .trim()
                    .to_string(),
                balance: m.name("balance")
                    .unwrap()
                    .as_str()
                    .trim()
                    .replace(" ", "")
                    .replace(",", "")
                    .replace("\u{a0}", "")
                    .replace("−", "-")
                    .parse::<isize>()
                    .unwrap(),
                bank_name: m.name("bank_name")
                    .unwrap()
                    .as_str()
                    .trim()
                    .to_string(),
                kind: kind,
            }
        })
        .collect::<Vec<Account>>();

    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use crate::{client::account::extract_accounts, account::AccountKind};

    #[test]
    fn test_extract_accounts() {
        let accounts = extract_accounts(ACCOUNTS_RES, AccountKind::Savings).unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].name, "LIVRET DEVELOPPEMENT DURABLE SOLIDAIRE");
        assert_eq!(accounts[0].balance, 1101000);
        assert_eq!(accounts[0].bank_name, "BoursoBank");
        assert_eq!(accounts[1].id, "d4e4fd4067b6d4d0b538a15e42238ef9");
        assert_eq!(accounts[1].name, "Livret Jeune");
        assert_eq!(accounts[1].balance, 159972);
        assert_eq!(accounts[1].bank_name, "Crédit Agricole");
        let accounts = extract_accounts(ACCOUNTS_RES, AccountKind::Banking).unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].id, "e2f509c466f5294f15abd873dbbf8a62");
        assert_eq!(accounts[0].name, "BoursoBank");
        assert_eq!(accounts[0].balance, 2081050);
        assert_eq!(accounts[0].bank_name, "BoursoBank");
        assert_eq!(accounts[1].name, "Compte de chèques ****0102");
        assert_eq!(accounts[1].balance, 50040);
        assert_eq!(accounts[1].bank_name, "CIC");
        let accounts = extract_accounts(ACCOUNTS_RES, AccountKind::Trading).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "PEA DOE");
        let accounts = extract_accounts(ACCOUNTS_RES, AccountKind::Loans).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "Prêt personnel");
        assert_eq!(accounts[0].balance, -9495982);
        assert_eq!(accounts[0].bank_name, "Crédit Agricole");
    }



    pub const ACCOUNTS_RES: &str = r#"<hx:include id="hinclude__XXXXXXXX" src="/dashboard/offres?rumroute=dashboard.offers"
    data-cs-override-id="dashboard.offers">
    <div class="c-offers_loading o-vertical-interval-bottom-medium">
        <div class="bourso-spinner">
            <img src=" data:image/png;base64,iVBO"
                alt="">
        </div>
    </div>
</hx:include>

<div class="c-panel c-panel--primary o-vertical-interval-bottom-medium " id="panel-XXXXXXXX">
    <div class="c-panel__header ">
        <span class="c-panel__title" id="panel-XXXXXXXX-title">
            Mon compte bancaire
        </span>
        <span class="c-panel__subtitle">
            21 310,90 €
        </span>
    </div>
    <div class="c-panel__body ">
        <div class="c-panel__no-animation-glitch ">
            <ul class="c-info-box " aria-label="Mon compte bancaire - Total : 21 310,90 €" role="list"
                data-brs-list-header data-summary-bank>
                <li class="c-panel__item c-info-box__item" data-brs-filterable>
                    <a class="c-info-box__link-wrapper" href="/compte/cav/e2f509c466f5294f15abd873dbbf8a62/"
                        data-tag-commander-click='{"label": "application::customer.dashboard::click_accounts_cav", "s2": 1, "type": "N"}'
                        aria-label="Détails du compte BoursoBank - Solde : 20 810,50 €" title="BoursoBank">

                        <span class="c-info-box__account">
                            <span class="c-info-box__account-label"
                                data-account-label="e2f509c466f5294f15abd873dbbf8a62" data-brs-list-item-label>
                                BoursoBank
                            </span>
                            <span class="c-info-box__account-balance c-info-box__account-balance--positive">
                                20 810,50 €
                            </span>
                        </span>

                        <span class="c-info-box__account-sub-label" data-brs-list-item-label>
                            BoursoBank
                        </span>

                        <ul class="c-info-box__account-attached-products">
                            <li class="c-info-box__product">
                                <span class="c-info-box__product-name">
                                    <span class="c-info-box__card ">
                                        <img class="c-info-box__card-image "
                                            src="/bundles/boursoramadesign/img/cbi/25x16/prime_black.png" alt=""
                                            aria-hidden="true">
                                    </span>
                                    JOHN DOE
                                </span>
                            </li>
                        </ul>
                    </a>
                </li>
                <li class="c-panel__item c-info-box__item" data-brs-filterable>
                    <a class="c-info-box__link-wrapper" href="/budget/compte/a22217240487004d13c8a6b5da422bbf/"
                        data-tag-commander-click='{"label": "application::customer.dashboard::click_accounts_pfm_cav", "s2": 1, "type": "N"}'
                        aria-label="Détails du compte Compte de chèques ****0102 - Solde : 500,40 €"
                        title="Compte de chèques ****0102">

                        <span class="c-info-box__account">
                            <span class="c-info-box__account-label"
                                data-account-label="a22217240487004d13c8a6b5da422bbf" data-brs-list-item-label>
                                Compte de chèques ****0102
                            </span>
                            <span class="c-info-box__account-balance c-info-box__account-balance--positive">
                                500,40 €
                            </span>
                        </span>

                        <span class="c-info-box__account-sub-label" data-brs-list-item-label>
                            CIC
                        </span>
                    </a>
                </li>
            </ul>
        </div>
    </div>
</div>


<div class="c-panel c-panel--primary o-vertical-interval-bottom-medium " id="panel-XXXXXXXX">
    <div class="c-panel__header ">
        <span class="c-panel__title" id="panel-XXXXXXXX-title">
            Mon épargne
        </span>
        <span class="c-panel__subtitle">
            12 609,72 €
        </span>
    </div>
    <div class="c-panel__body ">
        <div class="c-panel__no-animation-glitch ">
            <ul class="c-info-box " aria-label="Mon épargne - Total : 12 609,72 €" role="list" data-brs-list-header
                data-summary-savings>
                <li class="c-panel__item c-info-box__item" data-brs-filterable>
                    <a class="c-info-box__link-wrapper" href="/compte/epargne/ldd/a8a23172b7e7c91c538831578242112e/"
                        data-tag-commander-click='{"label": "application::customer.dashboard::click_accounts_saving", "s2": 1, "type": "N"}'
                        aria-label="Détails du compte LIVRET DEVELOPPEMENT DURABLE SOLIDAIRE - Solde : 11 010,00 €"
                        title="LIVRET DEVELOPPEMENT DURABLE SOLIDAIRE">

                        <span class="c-info-box__account">
                            <span class="c-info-box__account-label"
                                data-account-label="a8a23172b7e7c91c538831578242112e" data-brs-list-item-label>
                                LIVRET DEVELOPPEMENT DURABLE SOLIDAIRE
                            </span>
                            <span class="c-info-box__account-balance c-info-box__account-balance--positive">
                                11 010,00 €
                            </span>
                        </span>

                        <span class="c-info-box__account-sub-label" data-brs-list-item-label>
                            BoursoBank
                        </span>
                    </a>
                </li>
                <li class="c-panel__item c-info-box__item" data-brs-filterable>
                    <a class="c-info-box__link-wrapper" href="/budget/compte/d4e4fd4067b6d4d0b538a15e42238ef9/"
                        data-tag-commander-click='{"label": "application::customer.dashboard::click_accounts_pfm_saving", "s2": 1, "type": "N"}'
                        aria-label="Détails du compte Livret Jeune - Solde : 1 599,72 €" title="Livret Jeune">

                        <span class="c-info-box__account">
                            <span class="c-info-box__account-label"
                                data-account-label="d4e4fd4067b6d4d0b538a15e42238ef9" data-brs-list-item-label>
                                Livret Jeune
                            </span>
                            <span class="c-info-box__account-balance c-info-box__account-balance--positive">
                                1 599,72 €
                            </span>
                        </span>

                        <span class="c-info-box__account-sub-label" data-brs-list-item-label>
                            Crédit Agricole
                        </span>
                    </a>
                </li>
            </ul>
        </div>
    </div>
</div>


<div class="c-panel c-panel--primary o-vertical-interval-bottom-medium " id="panel-XXXXXXXX">
    <div class="c-panel__header ">
        <span class="c-panel__title" id="panel-XXXXXXXX-title">
            Mes placements financiers
        </span>
        <span class="c-panel__subtitle">
            143 088,89 €
        </span>
    </div>
    <div class="c-panel__body ">
        <div class="c-panel__no-animation-glitch ">
            <ul class="c-info-box " aria-label="Mes placements financiers - Total : 143 088,89 €" role="list"
                data-brs-list-header data-summary-trading>
                <li class="c-panel__item c-info-box__item" data-brs-filterable>
                    <a class="c-info-box__link-wrapper" href="/compte/pea/9651d8edd5975de1b9eff3865505f15f/"
                        data-tag-commander-click='{"label": "application::customer.dashboard::click_accounts_investement", "s2": 1, "type": "N"}'
                        aria-label="Détails du compte PEA DOE - Solde : 143 088,89 €" title="PEA DOE">

                        <span class="c-info-box__account">
                            <span class="c-info-box__account-label"
                                data-account-label="9651d8edd5975de1b9eff3865505f15f" data-brs-list-item-label>
                                PEA DOE
                            </span>
                            <span class="c-info-box__account-balance c-info-box__account-balance--positive">
                                143 088,89 €
                            </span>
                        </span>

                        <span class="c-info-box__account-sub-label" data-brs-list-item-label>
                            BoursoBank
                        </span>
                    </a>
                </li>
            </ul>
        </div>
    </div>
</div>


<div class="c-panel c-panel--primary o-vertical-interval-bottom-medium " id="panel-XXXXXXXX">
    <div class="c-panel__header ">
        <span class="c-panel__title" id="panel-XXXXXXXX-title">
            Mes crédits
        </span>
        <span class="c-panel__subtitle">
            − 94 959,82 €
        </span>
    </div>
    <div class="c-panel__body ">
        <div class="c-panel__no-animation-glitch ">
            <ul class="c-info-box " aria-label="Mes crédits - Total : − 94 959,82 €" role="list" data-brs-list-header
                data-summary-loan>
                <li class="c-panel__item c-info-box__item" data-brs-filterable>
                    <a class="c-info-box__link-wrapper" href="/budget/compte/7315a57115ae889992ec98a6bb3571cb/"
                        data-tag-commander-click='{"label": "application::customer.dashboard::click_accounts_pfm_loan", "s2": 1, "type": "N"}'
                        aria-label="Détails du compte Prêt personnel - Solde : − 94 959,82 €" title="Prêt personnel">

                        <span class="c-info-box__account">
                            <span class="c-info-box__account-label"
                                data-account-label="7315a57115ae889992ec98a6bb3571cb" data-brs-list-item-label>
                                Prêt personnel
                            </span>
                            <span class="c-info-box__account-balance c-info-box__account-balance--neutral">
                                − 94 959,82 €
                            </span>
                        </span>

                        <span class="c-info-box__account-sub-label" data-brs-list-item-label>
                            Crédit Agricole
                        </span>
                    </a>
                </li>
            </ul>
        </div>
    </div>
</div>

<!-- The Corner -->

<!-- Ajouter un compte externe -->

<!-- script -->
    "#;
}