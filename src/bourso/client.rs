use std::{io, sync::Arc};

use super::{
    config::{extract_brs_config, Config},
    constants::{
        ACCOUNT_PATTERN, BANKING_PATTERN, BASE_URL, LOANS_PATTERN, SAVINGS_PATTERN, TRADING_PATTERN,
    },
    virtual_pad, Account, AccountKind,
    utils::{log_with_timestamp}
};
use anyhow::{bail, Context, Result};
use colored::*;
use cookie_store::Cookie;
use regex::Regex;
use reqwest::Method;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
pub struct BoursoWebClient {
    /// The client used to make requests to the Bourso website.
    client: reqwest::Client,
    /// __brs_mit cookie is a cookie that is necessary to access login page.
    /// Bourso website sets it when you access the login page for the first time before refreshing the page.
    brs_mit_cookie: String,
    /// Virtual pad IDs are the IDs of the virtual pad keys. They are used to translate the password
    virtual_pad_ids: Vec<String>,
    /// Challenge ID is a token retrieved from the virtual pad page. It represents a random string
    /// that corresponds to the used virtual pad keys layout.
    challenge_id: String,
    /// Customer ID used to login.
    customer_id: String,
    /// Form token used to login.
    token: String,
    /// Password used to login.
    password: String,
    /// Cookie store used to store cookies between each request made by the client to the Bourso website.
    cookie_store: Arc<CookieStoreMutex>,
    /// Bourso Web current configuration
    config: Config,
}

impl BoursoWebClient {
    pub fn new() -> BoursoWebClient {
        // create a new client
        let cookie_store = CookieStore::new(None);
        let cookie_store = CookieStoreMutex::new(cookie_store);
        let cookie_store = Arc::new(cookie_store);
        BoursoWebClient {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .cookie_provider(Arc::clone(&cookie_store))
                .build()
                .unwrap(),
            cookie_store: cookie_store,
            brs_mit_cookie: String::new(),
            virtual_pad_ids: Default::default(),
            challenge_id: String::new(),
            customer_id: String::new(),
            token: String::new(),
            password: String::new(),
            config: Config::default(),
        }
    }

    /// Get the headers needed to make requests to the Bourso website.
    ///
    /// # Returns
    ///
    /// The headers as a `reqwest::header::HeaderMap`.
    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "user-agent", 
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36".parse().unwrap(),
        );

        headers
    }

    /// Get the login page content as a string.
    ///
    /// We're forced to call this page at least two times to retrieve the `__brs_mit` cookie and the form token.
    ///
    /// # Returns
    ///
    /// The login page content as a string.
    async fn get_login_page(&self) -> Result<String> {
        Ok(self
            .client
            .get(format!("{BASE_URL}/connexion/"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?)
    }

    /// Initialize the session by retrieving the `__brs_mit` cookie, the form token, the challenge ID and the virtual pad keys.
    ///
    /// # Returns
    ///
    /// Nothing if the session was initialized successfully, an error otherwise.
    pub async fn init_session(&mut self) -> Result<()> {
        // This first call is necessary to get the __brs_mit cookie
        let init_res = self.get_login_page().await?;

        self.brs_mit_cookie = extract_brs_mit_cookie(&init_res)?;

        // Use a scope to drop the lock on the cookie store
        // once we've inserted the necessary cookies
        {
            let mut store = self.cookie_store.lock().unwrap();
            store.insert(
                Cookie::parse(
                    // Necessary cookie to remove the domain migration error
                    "brsDomainMigration=migrated;",
                    &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap(),
                )
                .unwrap(),
                &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap(),
            )?;
            store.insert(
                Cookie::parse(
                    // Necessary cookie to access the virtual pad
                    format!("__brs_mit={};", self.brs_mit_cookie),
                    &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap(),
                )
                .unwrap(),
                &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap(),
            )?;
        }

        // We call the login page again to a form token
        let res = self.get_login_page().await?;

        self.token = extract_token(&res)?;
        self.config = extract_brs_config(&res)?;
        log_with_timestamp(format!("Using version from {}", self.config.app_release_date).blue());

        let res = self
            .client
            .get(format!("{BASE_URL}/connexion/clavier-virtuel?_hinclude=1"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        self.challenge_id = extract_challenge_token(&res)?;

        self.virtual_pad_ids = extract_data_matrix_keys(&res)?
            .map(|key| key.to_string())
            .to_vec();

        Ok(())
    }

    /// Translate a password to virtual pad keys.
    ///
    /// It matches each character of the password to a virtual pad key.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to translate.
    ///
    /// # Returns
    ///
    /// The virtual pad keys as an array of strings.
    fn password_to_virtual_pad_keys(&self, password: &str) -> Result<Vec<String>> {
        let mut keys: Vec<String> = Vec::new();
        for c in password.chars() {
            let number = c
                .to_digit(10)
                .with_context(|| format!("Invalid character in password: {}", c))?;
            keys.push(self.virtual_pad_ids[number as usize].clone());
        }

        Ok(keys)
    }
    
    /// Performs strong authentication verification.
    ///
    /// This method handles the verification process by communicating with the server,
    /// awaiting user input, and validating the authentication status. If the user is successfully
    /// verified via the app, it returns `true`. If not, it returns `false`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the strong authentication was successful.
    pub async fn handle_strong_auth_verification(&mut self) -> Result<bool> {
        log_with_timestamp(format!("Initiating strong authentication verification.").blue());
        let res = self
            .client
            .get(format!("{BASE_URL}/securisation"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        self.config = extract_brs_config(&res)?;

        if let Some(config) = &self.config.user_hash {
            log_with_timestamp(format!("Retrieved user hash: `{}`", config).blue());
        } else {
            log_with_timestamp(format!("User hash not found in config during strong authentication.").red());
            bail!("Strong authentication failed: User hash not found.");
        }

        let res = self
            .client
            .get(format!("{BASE_URL}/securisation/validation"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        let token_form = extract_strong_auth_form_token(&res)?;
        let form_state = extract_strong_auth_formstate(&res)?;

        self.client
            .request(Method::OPTIONS, format!(
                "https://api.boursobank.com/services/api/v1.7/_user_/_{}_/session/challenge/checkwebtoapp/10305",
                self.config.user_hash.as_mut().unwrap()
            ))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        // Implement Bearer authorization to headers
        let mut headers = self.get_headers();
        headers.insert(
            "authorization",
            format!("Bearer {}", self.config.default_api_bearer)
                .parse()
                .unwrap(),
        );


        let strong_auth_url = format!(
            "https://api.boursobank.com/services/api/v1.7/_user_/_{}_/session/challenge/startwebtoapp/10305", 
            self.config.user_hash.as_mut().unwrap()
        );

        // Send strong auth verification to app
        self.client.post(strong_auth_url)
            .headers(headers.clone())
            .body(serde_json::json!({ "form_state": form_state }).to_string())
            .send()
            .await?
            .text()
            .await?;

        log_with_timestamp(format!("Strong authentication: Sending request to app").blue());

        // Await for user to press enter
        log_with_timestamp(format!("Click 'Enter' after the app verification is complete.").yellow());

        io::stdin()
            .read_line(&mut String::new())
            .expect("Failed to read line");

        // Validate verification
        self.client
            .post("https://clients.boursobank.com/securisation/validation")
            .headers(self.get_headers())
            .form(&[("form[_token]", token_form)])
            .send()
            .await?
            .text()
            .await?;

        // Check if verificaiton is done
        let res = self
            .client
            .get(format!("{BASE_URL}/"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        if res.contains(r#"href="/se-deconnecter""#) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Login to the Bourso website.
    ///
    /// # Arguments
    ///
    /// * `customer_id` - The customer ID used to login.
    /// * `password` - The password used to login in plaintext.
    ///
    /// # Returns
    ///
    /// Nothing if the login was successful, an error otherwise.
    pub async fn login(&mut self, customer_id: &str, password: &str) -> Result<()> {
        log_with_timestamp(format!("Attempting to login user: `{}`", customer_id).blue());
        self.customer_id = customer_id.to_string();
        self.password = password.to_string();
        let data = reqwest::multipart::Form::new()
            .text("form[fakePassword]", "••••••••")
            .text("form[ajx]", "1")
            .text(
                "form[password]",
                self.password_to_virtual_pad_keys(password)?.join("|"),
            )
            // passwordAck is a JSON object that indicates the different times the user pressed on the virtual pad keys,
            // the click coordinates and the screen size. It seems like it's not necessary to fill the values to login.
            .text("form[passwordAck]", r#"{"ry":[],"pt":[],"js":true}"#)
            .text("form[platformAuthenticatorAvailable]", "1")
            .text("form[matrixRandomChallenge]", self.challenge_id.to_string())
            .text("form[_token]", self.token.to_string())
            .text("form[clientNumber]", self.customer_id.to_string());

        let res = self
            .client
            .post(format!("{BASE_URL}/connexion/saisie-mot-de-passe"))
            .multipart(data)
            .headers(self.get_headers())
            .send()
            .await?;

        if res.status() != 302 {
            log_with_timestamp(
                format!(
                    "Login failed for user `{}`, status code: {}",
                    customer_id,
                    res.status()
                ).red()
            );
            bail!(
                "Could not login to Bourso website, status code: {}",
                res.status()
            );
        }

        let res = self
            .client
            .get(format!("{BASE_URL}/"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        if res.contains(r#"href="/se-deconnecter""#) {
            // Update the config with user hash
            self.config = extract_brs_config(&res)?;
            log_with_timestamp(format!("User `{}` logged in successfully.", customer_id).green());
        } else if res.contains(r#"href="/securisation""#) {
            // Strong authentication required
            log_with_timestamp(format!("User `{}` requires strong authentication.", customer_id).yellow());
            match self.handle_strong_auth_verification().await {
                Ok(true) => {
                    log_with_timestamp(format!("Strong authentication verified successfully.").green());
                    log_with_timestamp(format!("User `{}` requires strong authentication.", customer_id).green());
                }
                Ok(false) => {
                    log_with_timestamp(format!("Strong authentication verification failed or incomplete.").red());
                    bail!("Strong authentication verification failed or incomplete.");
                }
                Err(e) => {
                    log_with_timestamp(format!("An error occurred during strong authentication verification: {}",
                    e).red());
                }
            }
        } else {
            log_with_timestamp(
                format!(
                    "Login failed for user {}, could not confirm login on Bourso website",customer_id
                ).red()
            );
            bail!("Could not login to Bourso website");
        }

        Ok(())
    }

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
        let res = self
            .client
            .get(format!(
                "{BASE_URL}/dashboard/liste-comptes?rumroute=dashboard.new_accounts&_hinclude=1"
            ))
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
            _ => [
                extract_accounts(&res, AccountKind::Savings)?,
                extract_accounts(&res, AccountKind::Banking)?,
                extract_accounts(&res, AccountKind::Trading)?,
                extract_accounts(&res, AccountKind::Loans)?,
            ]
            .concat(),
        };

        Ok(accounts)
    }
}

/// Extract the __brs_mit cookie from a string, usually the response of the `/connexion/` page.
///
/// # Arguments
///
/// * `res` - The response of the `/connexion/` page as a string.
///
/// # Returns
///
/// The __brs_mit cookie as a string.
///
/// # Example
///
/// ```
/// let res = r#"<!DOCTYPE html> \n<html>\n<head>\n    <script type="text/javascript">\n    document.cookie="__brs_mit=8e6912eb6a0268f0a2411668b8bf289f; domain=." + window.location.hostname + "; path=/; ";\n    window.location.reload();\n    </script>\n</head>\n<body>\n</body>\n</html>\n\n"#;
/// let brs_mit_cookie = extract_brs_mit_cookie(&res).unwrap();
/// assert_eq!(brs_mit_cookie, "8e6912eb6a0268f0a2411668b8bf289f");
/// ```
fn extract_brs_mit_cookie(res: &str) -> Result<String> {
    let regex = Regex::new(r"(?m)__brs_mit=(?P<brs_mit_cookie>.*?);").unwrap();
    let brs_mit_cookie = regex
        .captures(&res)
        .unwrap()
        .name("brs_mit_cookie")
        .unwrap();

    Ok(brs_mit_cookie.as_str().to_string())
}

/// Extract the challenge token from a string, usually the response of the `/connexion/clavier-virtuel?_hinclude=1` page.
///
/// # Arguments
///
/// * `res` - The response of the `/connexion/clavier-virtuel?_hinclude=1` page as a string.
///
/// # Returns
///
/// The challenge token as a string.
fn extract_challenge_token(res: &str) -> Result<String> {
    let regex =
        Regex::new(r#"(?m)data-matrix-random-challenge\]"\)\.val\("(?P<challenge_id>.*?)"\)"#)
            .unwrap();
    let challenge_id = regex.captures(&res).unwrap().name("challenge_id").unwrap();

    Ok(challenge_id.as_str().trim().to_string())
}

/// Extract the data matrix keys from a string, usually the response of the `/connexion/clavier-virtuel?_hinclude=1` page.
///
/// # Arguments
///
/// * `res` - The response of the `/connexion/clavier-virtuel?_hinclude=1` page as a string.
///
/// # Returns
///
/// The data matrix keys as an array of 10 strings.
fn extract_data_matrix_keys(res: &str) -> Result<[&str; 10]> {
    let regex = Regex::new(r#"(?ms)<button.*?data-matrix-key="(?P<matrix_key>[A-Z]{3})".*?src="(?P<svg>data:image.*?)">.*?</button>"#).unwrap();
    let mut keys: [&str; 10] = Default::default();
    //let mut keys = [String::new(); 10];
    // get_number_for_svg(&svg);
    for cap in regex.captures_iter(&res) {
        let matrix_key = cap.name("matrix_key").unwrap();
        let svg = cap.name("svg").unwrap();
        let number = virtual_pad::get_number_for_svg(&svg.as_str())
            .with_context(|| format!("Could not find number for svg: {}.\nIt seems like the Bourso login page has changed, please contact an admin.", svg.as_str()))?;
        keys[number as usize] = matrix_key.as_str();
    }

    Ok(keys)
}

fn extract_token(res: &str) -> Result<String> {
    let regex = Regex::new(r#"(?ms)form\[_token\]"(.*?)value="(?P<token>.*?)"\s*>"#).unwrap();
    let token = regex.captures(&res).unwrap().name("token").unwrap();

    Ok(token.as_str().trim().to_string())
}

fn extract_accounts(res: &str, kind: AccountKind) -> Result<Vec<Account>> {
    let regex = Regex::new(match kind {
        AccountKind::Savings => SAVINGS_PATTERN,
        AccountKind::Banking => BANKING_PATTERN,
        AccountKind::Trading => TRADING_PATTERN,
        AccountKind::Loans => LOANS_PATTERN,
    })?;
    let accounts_ul = regex.captures(&res).unwrap().get(1).unwrap().as_str();

    let account_regex = Regex::new(ACCOUNT_PATTERN)?;

    let accounts = account_regex
        .captures_iter(&accounts_ul)
        .map(|m| {
            Account {
                id: m.name("id").unwrap().as_str().trim().to_string(),
                name: m.name("name").unwrap().as_str().trim().to_string(),
                balance: m
                    .name("balance")
                    .unwrap()
                    .as_str()
                    .trim()
                    .replace(" ", "")
                    .replace(",", "")
                    .replace("\u{a0}", "")
                    .replace("−", "-")
                    .parse::<isize>()
                    .unwrap(),
                bank_name: m.name("bank_name").unwrap().as_str().trim().to_string(),
                kind: kind,
            }
        })
        .collect::<Vec<Account>>();

    Ok(accounts)
}

/// Extract the form state from a string, usually the response of the `/securisation/validation` page.
/// 
/// # Arguments
/// 
/// * `res` - The response of the `/securisation/validation` page as a string.
/// 
/// # Returns
/// 
/// The form state as a string.
fn extract_strong_auth_formstate(res: &str) -> Result<String> {
    let regex = Regex::new(r#"(?ms)formState&quot;\s?:\s?&quot;(?P<formState>.*?)&quot;"#).unwrap();
    let form_state_as_str = regex
        .captures(res)
        .unwrap()
        .name("formState")
        .unwrap()
        .as_str()
        .to_string();

    Ok(form_state_as_str)
}

/// Extract the form strong auth token from a string, usually the response of the `/securisation/validation` page.
/// 
/// # Arguments
/// 
/// * `res` - The response of the `/securisation/validation` page as a string.
/// 
/// # Returns
/// 
/// The form state as a string.
pub fn extract_strong_auth_form_token(res: &str) -> Result<String> {
    let regex = Regex::new(r#"(?ms)form__token(.*?)value="(?P<token>.*?)""#).unwrap();

    let form_token_as_str = regex
        .captures(res)
        .unwrap()
        .name("token")
        .unwrap()
        .as_str()
        .to_string();

    Ok(form_token_as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_brs_mit_cookie() {
        let res = r#"<!DOCTYPE html> \n<html>\n<head>\n    <script type="text/javascript">\n    document.cookie="__brs_mit=8e6912eb6a0268f0a2411668b8bf289f; domain=." + window.location.hostname + "; path=/; ";\n    window.location.reload();\n    </script>\n</head>\n<body>\n</body>\n</html>\n\n"#;
        let brs_mit_cookie = extract_brs_mit_cookie(&res).unwrap();
        assert_eq!(brs_mit_cookie, "8e6912eb6a0268f0a2411668b8bf289f");
    }

    #[test]
    fn test_extract_challenge_token() {
        let token = extract_challenge_token(VIRTUAL_PAD_RES).unwrap();
        assert_eq!(
            token,
            "THIS-STRING_represents0the1random__ElXSl-qJoXCKnqTBiew"
        );
    }

    #[test]
    fn test_extract_data_matrix_keys() {
        let keys = extract_data_matrix_keys(VIRTUAL_PAD_RES).unwrap();
        assert_eq!(
            keys,
            ["WZE", "UVQ", "LGK", "TLT", "ISV", "RNI", "ANP", "UCA", "FIG", "YCL"]
        );
    }

    #[test]
    fn test_extract_token() {
        let res = r#"data-backspace><i class="form-row-circles-password__backspace-icon / c-icon c-icon--backspace u-block"></i></button></div></div></div><input  id="form_ajx" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[ajx]" value="1" ><input  autocomplete="off" aria-label="Renseignez votre mot de passe en sélectionnant les 8 chiffres sur le clavier virtuel accessible ci-après par votre liseuse." data-matrix-password="1" id="form_password" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[password]" value="" ><input  data-password-ack="1" id="form_passwordAck" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[passwordAck]" value="{&quot;js&quot;:false}" ><input  data-authentication-factor-webauthn-detection="data-authentication-factor-webauthn-detection" id="form_platformAuthenticatorAvailable" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[platformAuthenticatorAvailable]" value="" ><input  data-matrix-random-challenge="1" id="form_matrixRandomChallenge" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[matrixRandomChallenge]" value="" ><input  id="form__token" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[_token]" value="45ed28b1-76ff-46a2-9202-0ee01928e6bb" ><hx:include id="hinclude__36d8139868f4bef54611a886784a3cbb"  src="/connexion/clavier-virtuel"><div data-matrix-placeholder class="sasmap sasmap--placeholder"><div class="bouncy-loader "><div class="bouncy-loader__balls"><div class="bouncy-loader__ball bouncy-loader__ball--left"></div><div class="bouncy-loader__ball bouncy-loader__ball--center"></div><div class="bouncy-loader__ball bouncy-loader__ball--right"></div></div></div></div></hx:include><div class="narrow-modal-window__input-container"><div class="u-text-center  o-vertical-interval-bottom "><div class="o-grid"><div class="o-grid__item"><button class="c-button--fancy c-button c-button--fancy u-1/1 c-button--primary"        type="submit"        data-login-submit       ><span class="c-button__text">Je me connecte</span></button></div><div class="o-grid__item  u-hidden" data-login-go-to-webauthn-wrapper><button class="c-button--fancy c-button c-button--fancy u-1/1 c-button--secondary"        type="button"        data-login-go-to-webauthn       ><span class="c-button__text">Clé de sécurité</span></button></div></div></div><div class="u-text-center"><a class="c-button--fancy c-button c-button--fancy u-1/1 c-button--tertiary c-button--link"        href="/connexion/mot-de-passe/retrouver"        data-pjax       ><span class="c-button__text">Mot de passe oublié ?</span></a></div></div><div class="narrow-modal-window__back-link"><button class="c-button--nav-back c-button u-1/1@xs-max c-button--text"        type="button"        data-login-back-to-login data-login-change-user-action="/connexion/oublier-identifiant"       ><span class="c-button__text"><div class="o-flex o-flex--align-center"><div class="c-button__icon"><svg xmlns="http://www.w3.org/2000/svg" width="7.8" height="14" viewBox="0 0 2.064 3.704"><path d="M1.712 3.644L.082 2.018a.212.212 0 0 1-.022-.02.206.206 0 0 1-.06-.146.206.206 0 0 1 .06-.147.212.212 0 0 1 .022-.019L1.712.06a.206.206 0 0 1 .291 0 .206.206 0 0 1 0 .291L.5 1.852l1.504 1.501a.206.206 0 0 1 0 .291.205.205 0 0 1-.146.06.205.205 0 0 1-.145-.06z"/></svg></div><div class="c-button__content">Mon identifiant</div></div></span></button></div></div><footer class="narrow-modal-footer narrow-modal-footer--mobile" data-transition-view-footer><div class="narrow-modal-footer__item narrow-modal-footer__item--mobile"><a href="" class="c-link c-link--icon c-link--pull-up c-link--subtle""#;
        let token = extract_token(&res).unwrap();
        assert_eq!(token, "45ed28b1-76ff-46a2-9202-0ee01928e6bb");
    }

    #[test]
    fn test_password_to_virtual_pad_keys() {
        let mut client = BoursoWebClient::new();
        let keys = extract_data_matrix_keys(VIRTUAL_PAD_RES)
            .unwrap()
            .map(|key| key.to_string())
            .to_vec();
        client = BoursoWebClient {
            virtual_pad_ids: keys,
            ..client
        };
        let password_translated_to_keys = client.password_to_virtual_pad_keys("123654").unwrap();
        assert_eq!(
            vec!["UVQ", "LGK", "TLT", "ANP", "RNI", "ISV",],
            password_translated_to_keys
        );
    }

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

    #[test]
    fn test_extract_strong_auth_formstate() {
        let formstate_as_str = extract_strong_auth_formstate(VALIDATION_RES).unwrap();
        assert_eq!(formstate_as_str, "eyJhbGciOiJIUzM4NCIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0.bQTnz6AuMJvmXXQsVPrxeQNvzDkimo7VNXxHeSBfClLufmCVZRUuyTwJF311JHuh");
    }

    #[test]
    fn test_extract_strong_auth_form_token() {
        let form_token = extract_strong_auth_form_token(VALIDATION_RES).unwrap();
        assert_eq!(form_token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c");
    }

    const VIRTUAL_PAD_RES: &str = r#"<div class="login-matrix">
    <div class="sr-only">
        Le bouton suivant permet d&#039;activer la vocalisation du clavier virtuel de saisie du mot de passe situé juste après.
          En activant la vocalisation, vous pouvez entendre les chiffres présents sur le clavier virtuel.
          Le clavier virtuel est composé de 2 lignes de 5 boutons, chacun correspondant à un chiffre de 0 à 9.
          Naviguez au clavier avec tabs ou les flèches pour entendre le chiffre correspondant.
          Si vous utilisez une interface tactile, vous pouvez maintenir appuyé chaque bouton pour entendre le chiffre.
    </div>

    <div class="login-a11y">
        <div class="login-a11y__switch">
            

    

<div class="c-switch c-switch--outline c-field c-field--error" data-id="switch-341374934" data-name="" data-brs-field><span id="aria-l-switch-341374934" class="u-sr-only">Activer la vocalisation</span><div class="c-switch__wrapper c-field__wrapper" data-brs-field-wrapper><input
     id="switch-341374934" type="checkbox" class="c-switch__checkbox" name="switch-341374934"    data-switch-id="switch-341374934"
    data-matrix-toggle-sound ><button
     role="checkbox" type="button" class="c-switch__button-wrapper" aria-checked="false"    aria-labelledby="aria-l-switch-341374934"
    data-switch="switch-341374934"
        ><span class="c-switch__inner"></span><span class="c-switch__button"></span></button><label  class="c-switch__label c-field__label" for="switch-341374934"><span class="c-field__label-text data-label-container" >Activer la vocalisation</span></label></div></div>        </div>
        <a href="javascript://;" class="brs-tooltip" data-selector="true" data-toggle="popover" data-placement="top"
           data-trigger="hover focus" data-content="Clavier sonore accessible
          aux clients non et malvoyants. Naviguez au clavier grâce à la touche tabulation ou, sur une interface
          tactile, en maintenant la touche appuyée. Validez la saisie de chaque chiffre avec la touche espace ou la
          touche entrée.">
            <span class="c-icon c-icon--help-helpbar"></span>
        </a>
    </div>

    <div class="sasmap"
        data-matrix data-matrix-harmony         data-matrix-random-challenge-selector="[data-matrix-random-challenge]"
                >

        <ul class="password-input">
                            <li data-matrix-list-item data-matrix-list-item-index="0">
                    <button type="button"
                            data-matrix-key="WZE"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Im0yMS41IDZjNC42IDAgNi40IDQuOCA2LjQgOC45cy0xLjggOC45LTYuNCA4LjljLTQuNyAwLTYuNC00LjgtNi40LTguOXMxLjgtOC45IDYuNC04Ljl6bTAgMS40Yy0zLjYgMC00LjggNC00LjggNy42IDAgMy41IDEuMiA3LjYgNC44IDcuNnM0LjgtNCA0LjgtNy42LTEuMi03LjYtNC44LTcuNnoiIGZpbGw9IiMwMDM4ODMiLz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="1">
                    <button type="button"
                            data-matrix-key="YCL"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im03LjYgMzEuNy0xLjYgNS44aC0xbC0yLTcuMmgxbDEuNiA2IDEuNi02aC44bDEuNiA2IDEuNi02aDFsLTIgNy4yaC0xeiIvPjxwYXRoIGQ9Im0xOCAzNC40LTIuMyAzLjFoLTEuMWwyLjgtMy43LTIuNi0zLjVoMS4xbDIuMSAyLjkgMi4xLTIuOWgxLjFsLTIuNiAzLjUgMi44IDMuN2gtMS4xeiIvPjxwYXRoIGQ9Im0yNi42IDM0LjUtMi44LTQuMWgxbDIuMiAzLjMgMi4yLTMuM2gxbC0yLjggNC4xdjNoLS45di0zeiIvPjxwYXRoIGQ9Im0zMy4xIDM2LjggNC01LjZoLTR2LS44aDUuMnYuN2wtNCA1LjZoNC4xdi44aC01LjJ2LS43eiIvPjwvZz48cGF0aCBkPSJtMTcuNyAyMC42Yy44IDEuMSAxLjkgMS45IDMuOCAxLjkgMy44IDAgNS4xLTQgNS4xLTcuNnYtLjhjLS44IDEuMi0yLjcgMi45LTUuMSAyLjktMy4xIDAtNS42LTEuOC01LjYtNS41LjEtMi44IDIuMi01LjUgNS45LTUuNSA0LjcgMCA2LjMgNC4zIDYuMyA4LjkgMCA0LjQtMS44IDguOS02LjYgOC45LTIuMyAwLTMuNi0uOS00LjYtMi4yem00LjEtMTMuMmMtMyAwLTQuMyAyLjMtNC4zIDQuMSAwIDIuOCAxLjkgNC4yIDQuMyA0LjIgMS45IDAgMy43LTEuMiA0LjctMy0uMi0yLjMtMS40LTUuMy00LjctNS4zeiIvPjwvZz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="2">
                    <button type="button"
                            data-matrix-key="ANP"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMy45IDMxLjYtMi40IDUuOWgtLjRsLTIuNC01Ljl2NS45aC0uOXYtNy4yaDEuM2wyLjIgNS40IDIuMi01LjRoMS4zdjcuMmgtLjl6Ii8+PHBhdGggZD0ibTE5LjUgMzEuOHY1LjdoLS45di03LjJoLjlsNC4xIDUuNnYtNS42aC45djcuMmgtLjl6Ii8+PHBhdGggZD0ibTMxLjcgMzAuMmMyLjEgMCAzLjYgMS42IDMuNiAzLjdzLTEuNCAzLjctMy42IDMuN2MtMi4xIDAtMy42LTEuNi0zLjYtMy43czEuNC0zLjcgMy42LTMuN3ptMCAuOGMtMS43IDAtMi43IDEuMi0yLjcgMi45czEgMi45IDIuNiAyLjkgMi42LTEuMiAyLjYtMi45Yy4xLTEuNy0uOS0yLjktMi41LTIuOXoiLz48L2c+PHBhdGggZD0ibTIyLjYgNmMyLjMgMCAzLjYuOSA0LjcgMi4ybC0uOSAxLjFjLS44LTEuMS0xLjktMS45LTMuOC0xLjktMy43IDAtNS4xIDMuOS01LjEgNy42di44Yy43LTEuMiAyLjctMi45IDUtMi45IDMuMSAwIDUuNiAxLjggNS42IDUuNSAwIDIuOC0yLjEgNS41LTUuOCA1LjUtNC43IDAtNi4zLTQuMy02LjMtOC45IDAtNC41IDEuOC05IDYuNi05em0tLjMgOC4yYy0xLjkgMC0zLjcgMS4yLTQuNyAzIC4yIDIuNCAxLjQgNS40IDQuNyA1LjQgMyAwIDQuMy0yLjMgNC4zLTQuMSAwLTIuOS0xLjgtNC4zLTQuMy00LjN6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="3">
                    <button type="button"
                            data-matrix-key="LGK"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMy45IDM1LjloLTMuNmwtLjYgMS42aC0xbDIuOS03LjJoMS4xbDIuOSA3LjJoLTF6bS0zLjMtLjhoM2wtMS41LTMuOXoiLz48cGF0aCBkPSJtMTguNyAzMC4zaDMuMmMxLjIgMCAyIC44IDIgMS44IDAgLjktLjYgMS41LTEuMyAxLjYuOC4xIDEuNC45IDEuNCAxLjggMCAxLjItLjggMS45LTIuMSAxLjloLTMuM3YtNy4xem0zIDMuMWMuOCAwIDEuMi0uNSAxLjItMS4yIDAtLjYtLjQtMS4yLTEuMi0xLjJoLTIuMnYyLjNoMi4yem0wIDMuM2MuOCAwIDEuMy0uNSAxLjMtMS4ycy0uNS0xLjItMS4zLTEuMmgtMi4ydjIuNWgyLjJ6Ii8+PHBhdGggZD0ibTI3LjMgMzMuOWMwLTIuMiAxLjYtMy43IDMuNy0zLjcgMS4zIDAgMi4yLjYgMi43IDEuNGwtLjguNGMtLjQtLjYtMS4yLTEtMi0xLTEuNiAwLTIuOCAxLjItMi44IDIuOXMxLjIgMi45IDIuOCAyLjljLjggMCAxLjYtLjQgMi0xbC44LjRjLS42LjgtMS41IDEuNC0yLjcgMS40LTIuMSAwLTMuNy0xLjUtMy43LTMuN3oiLz48L2c+PHBhdGggZD0ibTE1LjkgMjIuM2M1LjktNC43IDkuOC04LjEgOS44LTExLjQgMC0yLjUtMi0zLjUtMy45LTMuNS0yLjEgMC0zLjguOS00LjcgMi4zbC0xLS45YzEuMi0xLjggMy4zLTIuOCA1LjctMi44IDIuNSAwIDUuNCAxLjQgNS40IDQuOSAwIDMuOC00IDcuMy05IDExLjNoOS4xdjEuM2gtMTEuNHoiLz48L2c+PC9zdmc+">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="4">
                    <button type="button"
                            data-matrix-key="TLT"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMC4yIDMwLjNoMi41YzIuMiAwIDMuNyAxLjYgMy43IDMuNnMtMS41IDMuNi0zLjcgMy42aC0yLjV6bTIuNSA2LjRjMS43IDAgMi44LTEuMiAyLjgtMi44IDAtMS41LTEtMi44LTIuOC0yLjhoLTEuNnY1LjZ6Ii8+PHBhdGggZD0ibTE5LjkgMzAuM2g0Ljd2LjhoLTMuOHYyLjNoMy43di44aC0zLjd2Mi41aDMuOHYuOGgtNC43eiIvPjxwYXRoIGQ9Im0yOC4xIDMwLjNoNC43di44aC0zLjh2Mi4zaDMuN3YuOGgtMy43djMuM2gtLjl6Ii8+PC9nPjxwYXRoIGQ9Im0xNi4zIDIwLjFjMSAxLjQgMi42IDIuNCA0LjggMi40IDIuNyAwIDQuMy0xLjQgNC4zLTMuNyAwLTIuNS0yLTMuNS00LjYtMy41LS43IDAtMS4zIDAtMS42IDB2LTEuM2gxLjZjMi4zIDAgNC40LTEgNC40LTMuMyAwLTIuMS0xLjktMy4zLTQuMS0zLjMtMiAwLTMuNC44LTQuNiAyLjJsLS45LS45YzEuMi0xLjUgMy4xLTIuNyA1LjYtMi43IDMgMCA1LjYgMS42IDUuNiA0LjYgMCAyLjYtMi4yIDMuOC0zLjcgNCAxLjUuMiA0IDEuNCA0IDQuM3MtMi4xIDQuOS01LjggNC45Yy0yLjggMC00LjktMS4zLTUuOS0yLjl6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="5">
                    <button type="button"
                            data-matrix-key="FIG"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMS44IDMxLjFoLTIuM3YtLjhoNS40di44aC0yLjN2Ni40aC0uOXYtNi40eiIvPjxwYXRoIGQ9Im0xOC4zIDMwLjNoLjl2NC40YzAgMS4zLjcgMi4xIDIgMi4xczItLjggMi0yLjF2LTQuNGguOXY0LjRjMCAxLjgtMSAyLjktMi45IDIuOXMtMi45LTEuMi0yLjktMi45eiIvPjxwYXRoIGQ9Im0yNy4yIDMwLjNoMWwyLjQgNi4yIDIuNC02LjJoMWwtMi45IDcuMmgtMS4xeiIvPjwvZz48cGF0aCBkPSJtMjAuMyAxNC43Yy0yLS41LTQtMS45LTQtNC4yIDAtMy4xIDIuOC00LjUgNS42LTQuNSAyLjcgMCA1LjYgMS40IDUuNiA0LjUgMCAyLjMtMiAzLjYtNCA0LjIgMi4yLjYgNC4zIDIuMiA0LjMgNC42IDAgMi44LTIuNSA0LjYtNS44IDQuNnMtNS45LTEuOC01LjktNC42Yy0uMS0yLjUgMi00LjEgNC4yLTQuNnptMS42LjZjLTEuMS4xLTQuNCAxLjItNC40IDMuOCAwIDIuMSAyLjEgMy40IDQuNCAzLjRzNC40LTEuMyA0LjQtMy40YzAtMi42LTMuNC0zLjYtNC40LTMuOHptMC03LjljLTIuMyAwLTQuMSAxLjItNC4xIDMuMyAwIDIuNCAzLjEgMy4yIDQuMSAzLjQgMS4xLS4yIDQuMS0xIDQuMS0zLjQgMC0yLjEtMS44LTMuMy00LjEtMy4zeiIvPjwvZz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="6">
                    <button type="button"
                            data-matrix-key="ISV"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMy42IDMwLjJjMS4zIDAgMi4yLjYgMi44IDEuM2wtLjcuNWMtLjUtLjYtMS4yLTEtMi4xLTEtMS42IDAtMi44IDEuMi0yLjggMi45czEuMiAyLjkgMi44IDIuOWMuOSAwIDEuNi0uNCAxLjktLjh2LTEuNWgtMi41di0uOGgzLjR2Mi42Yy0uNy43LTEuNiAxLjItMi44IDEuMi0yIDAtMy43LTEuNS0zLjctMy43czEuNy0zLjYgMy43LTMuNnoiLz48cGF0aCBkPSJtMjUuMSAzNC4yaC00LjJ2My4zaC0uOXYtNy4yaC45djMuMWg0LjJ2LTMuMWguOXY3LjJoLS45eiIvPjxwYXRoIGQ9Im0yOS44IDMwLjNoLjl2Ny4yaC0uOXoiLz48L2c+PHBhdGggZD0ibTIzLjYgMTguOGgtOC4ydi0xLjNsNy43LTExLjJoMnYxMS4yaDIuNXYxLjNoLTIuNXY0LjdoLTEuNXptLTYuNy0xLjNoNi43di05Ljd6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="7">
                    <button type="button"
                            data-matrix-key="UCA"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im01IDMwLjRoMi45YzEuNCAwIDIuMiAxIDIuMiAyLjJzLS44IDIuMi0yLjIgMi4yaC0ydjIuOWgtLjl6bTIuOC44aC0xLjl2Mi44aDEuOWMuOSAwIDEuNC0uNiAxLjQtMS40cy0uNS0xLjQtMS40LTEuNHoiLz48cGF0aCBkPSJtMTkuMyAzNi43LjcuNy0uNi41LS43LS43Yy0uNS4zLTEuMi41LTEuOS41LTIuMSAwLTMuNi0xLjYtMy42LTMuN3MxLjQtMy43IDMuNi0zLjdjMi4xIDAgMy42IDEuNiAzLjYgMy43LS4xIDEuMS0uNCAyLTEuMSAyLjd6bS0xLjItLjEtMS0xLjEuNi0uNSAxIDEuMWMuNC0uNS43LTEuMi43LTIgMC0xLjctMS0yLjktMi42LTIuOXMtMi42IDEuMi0yLjYgMi45IDEgMi45IDIuNiAyLjljLjUtLjEuOS0uMiAxLjMtLjR6Ii8+PHBhdGggZD0ibTI2LjIgMzQuOGgtMS40djIuOWgtLjl2LTcuMmgyLjljMS4zIDAgMi4yLjggMi4yIDIuMiAwIDEuMy0uOSAyLTEuOSAyLjFsMS45IDIuOWgtMXptLjQtMy42aC0xLjl2Mi44aDEuOWMuOCAwIDEuNC0uNiAxLjQtMS40LjEtLjgtLjUtMS40LTEuNC0xLjR6Ii8+PHBhdGggZD0ibTMyLjcgMzUuOWMuNS41IDEuMiAxIDIuMyAxIDEuMyAwIDEuNy0uNyAxLjctMS4yIDAtLjktLjktMS4xLTEuOC0xLjQtMS4yLS4zLTIuNC0uNi0yLjQtMiAwLTEuMiAxLjEtMiAyLjUtMiAxLjEgMCAxLjkuNCAyLjUgMWwtLjcuN2MtLjUtLjYtMS4zLS45LTIuMS0uOS0uOSAwLTEuNS41LTEuNSAxLjEgMCAuNy44LjkgMS43IDEuMiAxLjIuMyAyLjUuNyAyLjUgMi4yIDAgMS0uNyAyLjEtMi42IDIuMS0xLjIgMC0yLjItLjUtMi44LTEuMXoiLz48L2c+PHBhdGggZD0ibTI0LjkgNy42aC05LjV2LTEuM2gxMS4zdjFsLTcuNCAxNi4yaC0xLjZ6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="8">
                    <button type="button"
                            data-matrix-key="RNI"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMS42IDM2LjFjLjMuNC43LjcgMS40LjcuOSAwIDEuNC0uNiAxLjQtMS41di01aC45djVjMCAxLjYtMSAyLjMtMi4zIDIuMy0uOCAwLTEuNC0uMi0xLjktLjh6Ii8+PHBhdGggZD0ibTIwLjcgMzQuMy0uNy44djIuNGgtLjl2LTcuMmguOXYzLjdsMy4yLTMuN2gxLjFsLTMgMy40IDMuMiAzLjhoLTEuMXoiLz48cGF0aCBkPSJtMjcuNyAzMC4zaC45djYuNGgzLjR2LjhoLTQuMnYtNy4yeiIvPjwvZz48cGF0aCBkPSJtMTcuNCAyMC4xYzEuMSAxLjYgMi42IDIuNSA0LjggMi41IDIuNSAwIDQuMy0xLjggNC4zLTQuMiAwLTIuNi0xLjgtNC4yLTQuMy00LjItMS42IDAtMi45LjUtNC4yIDEuN2wtMS0uNnYtOWgxMHYxLjNoLTguNXY2LjhjLjktLjggMi4zLTEuNiA0LjEtMS42IDIuOSAwIDUuNSAxLjkgNS41IDUuNSAwIDMuNC0yLjYgNS42LTUuOCA1LjYtMi45IDAtNC42LTEuMS01LjgtMi44eiIvPjwvZz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="9">
                    <button type="button"
                            data-matrix-key="UVQ"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Im0yMC44IDguMy0yLjggMy0uOS0xIDMuOC00aDEuM3YxNy4zaC0xLjV2LTE1LjN6IiBmaWxsPSIjMDAzODgzIi8+PC9zdmc+">
                    </button>
                </li>
                    </ul>

        <script>
            $(function () {
                $("[data-matrix-random-challenge]").val("THIS-STRING_represents0the1random__ElXSl-qJoXCKnqTBiew")
            })
        </script>
    </div>
</div>

<script>
    $(function(){
        $(document).find('[data-matrix]').brsMatrix();
    });
</script>"#;

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

    const VALIDATION_RES: &str = r##"<script src="/build/2109.c9addb486c7311ea3d2b.js"></script>
    <script src="/build/3762.5729c66025e7790f1387.js"></script>
    <script src="/build/head.1b44b6f58c46a5dcdced.js"></script>
    </head>
    
    <body class="is-boursobank     body--modal-full-display body--modal
        " data-is-layout-modal="true">
        <div id="modal-container" class="modal" data-pjax-container="#modal-container" role="dialog" aria-modal="true"
            aria-labelledby="modal-title"><a class="skip-link" href="#modal-main-content" tabindex="0">Aller au contenu
                principal</a>
            <div class="modal__container ">
                <h1 class="logo logo--white hidden-xs hidden-sm">
                    <picture>
                        <source media="(min-width:769px)" srcset="/bundles/boursoramadesign/img/logo-boursobank-blanc.png">
                        <img src="/bundles/boursoramadesign/img/boursorama-banque-white-logo@2x-cache-1458301567.png"
                            alt="BoursoBank">
                    </picture>
                    BoursoBank
                </h1>
                <div data-brs-scroll-notifier="(min-width: 1022px)"></div>
                <div class="modal__header " data-brs-scroll-notifier-taget>
                    <div class="modal__super-header modal__super-header--alone"><a title="Fermer la fenêtre " role="button"
                            href="/" class="modal__back u-force-focus-dark"></a>
                        <h2 class="page-title" tabindex="-1" id="modal-title" data-pjax-modal-title="">Sécurisation</h2>
                    </div>
                </div>
                <div id="modal-main-content" class="modal__content ">
                    <div class="content-block ">
                        <hx:include id="hinclude__57eb968bea74b40579c60bed3496deb4"
                            src="/aide/messages/modal?route=login.enrolment.otp&amp;showza=0"></hx:include>
                        <hx:cached>
                            <div id="za_content_login__enrolment__otp" class="managed  "></div>
                        </hx:cached>
                        <div data-strong-authentication-container>
                            <form name="form" method="post" method="POST" data-strong-authentication-form="true"
                                class="t-uibundle">
                                <div
                                    data-strong-authentication-payload="{&quot;challenges&quot;: [{&quot;type&quot;: &quot;brs-otp-webtoapp&quot;,&quot;realm&quot;: null,&quot;parameters&quot;: {&quot;formScreen&quot;: {&quot;title&quot;: &quot;Saisie du code&quot;,&quot;description&quot;: &quot;Ne fermez pas cette page. Pour valider l&#039;op\u00e9ration, ouvrez votre application BoursoBank sur votre smartphone et authentifiez-vous.&quot;,&quot;actions&quot;: {&quot;check&quot;: {&quot;label&quot;: &quot;Valider&quot;,&quot;featureId&quot;: &quot;customer.http_strong_auth_check&quot;,&quot;web&quot;: &quot;https:\/\/clients.boursobank.com\/feature-redirect?featureId=customer.http_strong_auth_check&amp;params%5BresourceId%5D=10305&quot;,&quot;api&quot;: {&quot;href&quot;: &quot;\/_user_\/_{userHash}_\/session\/challenge\/checkwebtoapp\/{resourceId}&quot;,&quot;method&quot;: &quot;POST&quot;,&quot;params&quot;: {&quot;resourceId&quot;: &quot;10305&quot;,&quot;formState&quot;: &quot;eyJhbGciOiJIUzM4NCIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0.bQTnz6AuMJvmXXQsVPrxeQNvzDkimo7VNXxHeSBfClLufmCVZRUuyTwJF311JHuh&quot;}},&quot;disabled&quot;: false},&quot;cancel&quot;: {&quot;label&quot;: &quot;Annuler mon op\u00e9ration&quot;,&quot;featureId&quot;: &quot;customer.dashboard_home&quot;,&quot;web&quot;: &quot;https:\/\/clients.boursobank.com\/feature-redirect?featureId=customer.dashboard_home&quot;,&quot;api&quot;: null,&quot;disabled&quot;: false}}},&quot;errorWrongOtp&quot;: {&quot;errorTitle&quot;: &quot;Erreur&quot;,&quot;errorDetails&quot;: &quot;Le service est momentan\u00e9ment indisponible ou le d\u00e9lai de l&#039;op\u00e9ration est d\u00e9pass\u00e9.&quot;,&quot;action&quot;: &quot;R\u00e9essayer&quot;,&quot;back&quot;: &quot;Annuler&quot;},&quot;title&quot;: &quot;Validation de votre op\u00e9ration&quot;,&quot;otpSubject&quot;: &quot;10305&quot;,&quot;canFallback&quot;: true,&quot;fallbackTitle&quot;: &quot;Vous n&#039;avez pas acc\u00e8s \u00e0 votre application ?&quot;,&quot;label&quot;: &quot;Utiliser mon application BoursoBank&quot;,&quot;smallDescription&quot;: &quot;Envoyer une notification \u00e0 un autre appareil&quot;,&quot;icon&quot;: null,&quot;presentationScreen&quot;: {&quot;description&quot;: &quot;Pour des raisons de s\u00e9curit\u00e9, nous souhaitons v\u00e9rifier votre identit\u00e9. Pour cela, munissez-vous d&#039;un autre appareil avec lequel vous vous \u00eates d\u00e9j\u00e0 identifi\u00e9 via l&#039;application.&quot;,&quot;userContact&quot;: null,&quot;actions&quot;: {&quot;start&quot;: {&quot;label&quot;: &quot;Envoyer une notification \u00e0 mon appareil&quot;,&quot;featureId&quot;: &quot;customer.http_strong_auth_start&quot;,&quot;web&quot;: &quot;https:\/\/clients.boursobank.com\/feature-redirect?featureId=customer.http_strong_auth_start&amp;params%5BresourceId%5D=10305&quot;,&quot;api&quot;: {&quot;href&quot;: &quot;\/_user_\/_{userHash}_\/session\/challenge\/startwebtoapp\/{resourceId}&quot;,&quot;method&quot;: &quot;POST&quot;,&quot;params&quot;: {&quot;resourceId&quot;: &quot;10305&quot;,&quot;formState&quot;: &quot;eyJhbGciOiJIUzM4NCIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0.bQTnz6AuMJvmXXQsVPrxeQNvzDkimo7VNXxHeSBfClLufmCVZRUuyTwJF311JHuh&quot;}},&quot;disabled&quot;: false},&quot;cancel&quot;: {&quot;label&quot;: &quot;Annuler&quot;,&quot;featureId&quot;: &quot;customer.dashboard_home&quot;,&quot;web&quot;: &quot;https:\/\/clients.boursobank.com\/feature-redirect?featureId=customer.dashboard_home&quot;,&quot;api&quot;: null,&quot;disabled&quot;: false}}},&quot;message&quot;: {&quot;id&quot;: &quot;fraud_without_agreement&quot;,&quot;type&quot;: &quot;INFO&quot;,&quot;detail&quot;: null,&quot;title&quot;: null,&quot;body&quot;: &quot;&lt;div\nstyle=\&quot;\ncolor: #1A2E45;\nfont-size: 1.2em;\ndisplay: flex;\nflex-wrap: wrap;\njustify-content: space-evenly\n\&quot;&gt;\n&lt;img src=\&quot;https:\/\/s.brsimg.com\/static-1686228603\/i\/otp\/introduction\/fraud.png\&quot; srcset=\&quot;https:\/\/s.brsimg.com\/static-1686228603\/i\/otp\/introduction\/fraud2x.png 2x\&quot; alt=\&quot;Attention, risque de fraude !\&quot;&gt;\n&lt;div style=\&quot;margin: auto 0; flex: 0.9; min-width: 300px;\&quot;&gt;\n&lt;div style=\&quot;font-size: 1.3em; color: #003883;\&quot;&gt;Attention, risque de fraude !&lt;\/div&gt;\n&lt;br&gt;\n&lt;strong style=\&quot;font-size: 1em\&quot;&gt;Ne validez pas l&amp;#039;op\u00e9ration &amp;quot;Nouvel appareil&amp;quot; si vous n&amp;#039;en \u00eates pas \u00e0 l&amp;#039;origine&lt;\/strong&gt;\n&lt;br&gt;&lt;br&gt;\n&lt;span&gt;Un collaborateur BoursoBank ne vous appellera &lt;strong&gt;JAMAIS&lt;\/strong&gt; pour le faire.&lt;\/span&gt;\n&lt;\/div&gt;\n&lt;\/div&gt;\n&quot;,&quot;params&quot;: null,&quot;category&quot;: &quot;custom&quot;,&quot;actions&quot;: [{&quot;label&quot;: &quot;Continuer&quot;,&quot;featureId&quot;: &quot;customer.http_strong_auth_start&quot;,&quot;web&quot;: &quot;https:\/\/clients.boursobank.com\/feature-redirect?featureId=customer.http_strong_auth_start&quot;,&quot;api&quot;: null,&quot;disabled&quot;: false}],&quot;flags&quot;: [&quot;ACTIONS_DISPLAY_DELAYED&quot;],&quot;targets&quot;: [],&quot;visualId&quot;: null,&quot;visualTheme&quot;: &quot;secondary&quot;,&quot;medias&quot;: []}}}],&quot;introduction&quot;: {&quot;title&quot;: &quot;Validation de votre op\u00e9ration&quot;,&quot;message&quot;: {&quot;id&quot;: &quot;fraud_without_agreement&quot;,&quot;type&quot;: &quot;INFO&quot;,&quot;detail&quot;: null,&quot;title&quot;: null,&quot;body&quot;: &quot;&lt;div\nstyle=\&quot;\ncolor: #1A2E45;\nfont-size: 1.2em;\ndisplay: flex;\nflex-wrap: wrap;\njustify-content: space-evenly\n\&quot;&gt;\n&lt;img src=\&quot;https:\/\/s.brsimg.com\/static-1686228603\/i\/otp\/introduction\/fraud.png\&quot; srcset=\&quot;https:\/\/s.brsimg.com\/static-1686228603\/i\/otp\/introduction\/fraud2x.png 2x\&quot; alt=\&quot;Attention, risque de fraude !\&quot;&gt;\n&lt;div style=\&quot;margin: auto 0; flex: 0.9; min-width: 300px;\&quot;&gt;\n&lt;div style=\&quot;font-size: 1.3em; color: #003883;\&quot;&gt;Attention, risque de fraude !&lt;\/div&gt;\n&lt;br&gt;\n&lt;strong style=\&quot;font-size: 1em\&quot;&gt;Ne validez pas l&amp;#039;op\u00e9ration &amp;quot;Nouvel appareil&amp;quot; si vous n&amp;#039;en \u00eates pas \u00e0 l&amp;#039;origine&lt;\/strong&gt;\n&lt;br&gt;&lt;br&gt;\n&lt;span&gt;Un collaborateur BoursoBank ne vous appellera &lt;strong&gt;JAMAIS&lt;\/strong&gt; pour le faire.&lt;\/span&gt;\n&lt;\/div&gt;\n&lt;\/div&gt;\n&quot;,&quot;params&quot;: null,&quot;category&quot;: &quot;custom&quot;,&quot;actions&quot;: [{&quot;label&quot;: &quot;Poursuivre l&#039;op\u00e9ration&quot;,&quot;featureId&quot;: &quot;ext.continue&quot;,&quot;web&quot;: null,&quot;api&quot;: null,&quot;disabled&quot;: false},{&quot;label&quot;: &quot;Annuler&quot;,&quot;featureId&quot;: &quot;ext.cancel&quot;,&quot;web&quot;: &quot;https:\/\/clients.boursobank.com\/feature-redirect?featureId=customer.dashboard_home&quot;,&quot;api&quot;: null,&quot;disabled&quot;: false}],&quot;flags&quot;: [&quot;ACTIONS_DISPLAY_DELAYED&quot;],&quot;targets&quot;: [],&quot;visualId&quot;: null,&quot;visualTheme&quot;: &quot;secondary&quot;,&quot;medias&quot;: []}},&quot;showChallengeChoice&quot;: false}">
                                </div><input id="form__token" type="hidden" class="c-field__input"
                                    data-brs-text-input="data-brs-text-input" name="form[_token]"
                                    value="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c">
                            </form>
                        </div>
                    </div>
                </div>
                <script>
                    $(function () {
                        var state = $('#modal-container').data('pushstate:history:closestate');
    
                        if (!state) {
                            return;
                        }
    
                        $('#modal-container > .modal__close')
                            .attr('title', state.title)
                            .attr('href', state.location)
                            .data('pushstate:history:state', state.state);
                    });
                </script>
            </div>
        </div>
        <aside id="drawer" class="c-drawer"></aside>
        <div class="modal-background"><span></span></div>
        <div class="modal-alert-background"><span></span></div>
        <div data-survey-button-container>
            <div data-survey-button-instance data-survey-button-identifier="vuejs-survey-button-component"></div>
        </div>
        <script src="/build/9755.d22f888134aeefc9ad08.js"></script>
        <script src="/build/critical.8f4107da2df04a690e11.js"></script>
        <script src="/build/brs-survey-button.687c78665a884e3ecdae.js"></script>
        <script src="/build/brs-user-satisfaction-review.4cc79c65f2c8b9a27ccc.js"></script>
        <script src="/build/6972.14b67b0f3dfed275658c.js"></script>
        <script src="/build/4988.03f12273d536cc991e0a.js"></script>
        <script src="/build/strong-authentication.213c4c106a8fa1066d20.js"></script>
        <script src="/build/brs-pfm-aggregator.3cb84ea858ac60673afb.js"></script>
        <script src="/build/boomerang.c5edf41ce0801c4b2c6c.js"></script>
    </body>"##;
}
