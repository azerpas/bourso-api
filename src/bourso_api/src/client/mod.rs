pub mod account;
pub mod config;
pub mod error;
pub mod qrcode;
pub mod trade;
pub mod transfer;
pub mod virtual_pad;

use core::fmt;
use std::sync::Arc;

use anyhow::{bail, Result};
use cookie_store::Cookie;
use error::ClientError;
use regex::Regex;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use self::config::{extract_brs_config, Config};

use super::constants::BASE_URL;

lazy_static::lazy_static! {
    static ref OTP_PARAMS_REGEX: Regex = Regex::new(r#"data-strong-authentication-payload="(\{.*?\})">"#)
        .expect("Failed to compile OTP parameters regex");
    static ref BRS_MIT_COOKIE_REGEX: Regex = Regex::new(r"(?m)__brs_mit=(?P<brs_mit_cookie>.*?);")
        .expect("Failed to compile __brs_mit cookie regex");
    static ref TOKEN_REGEX: Regex = Regex::new(r#"(?ms)form\[_token\]"(.*?)value="(?P<token>.*?)"\s*>"#)
        .expect("Failed to compile form token regex");
    static ref USER_CONTACT_REGEX: Regex = Regex::new(r"(?m)userContact&quot;:&quot;(?P<contact_user>.*?)&quot;")
        .expect("Failed to compile user contact regex");
}

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
    pub config: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MfaType {
    Email,
    Sms,
    WebToApp,
}

impl fmt::Display for MfaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MfaType::Email => write!(f, "email"),
            MfaType::Sms => write!(f, "sms"),
            MfaType::WebToApp => write!(f, "web to app"),
        }
    }
}

impl MfaType {
    pub fn start_path(&self) -> &'static str {
        match self {
            MfaType::Email => "startemail",
            MfaType::Sms => "startsms",
            MfaType::WebToApp => "startwebtoapp",
        }
    }

    pub fn check_path(&self) -> &'static str {
        match self {
            MfaType::Email => "checkemail",
            MfaType::Sms => "checksms",
            MfaType::WebToApp => "checkwebtoapp",
        }
    }
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
    #[cfg(not(tarpaulin_include))]
    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36"
                .parse()
                .unwrap(),
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
    #[cfg(not(tarpaulin_include))]
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
    #[cfg(not(tarpaulin_include))]
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
        debug!("Using version from {}", self.config.app_release_date);

        let res = self
            .client
            .get(format!("{BASE_URL}/connexion/clavier-virtuel?_hinclude=1"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        self.challenge_id = virtual_pad::extract_challenge_token(&res)?;

        self.virtual_pad_ids = virtual_pad::extract_data_matrix_keys(&res)?
            .map(|key| key.to_string())
            .to_vec();

        Ok(())
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
    #[cfg(not(tarpaulin_include))]
    pub async fn login(&mut self, customer_id: &str, password: &str) -> Result<()> {
        use error::ClientError;

        self.customer_id = customer_id.to_string();
        self.password =
            virtual_pad::password_to_virtual_pad_keys(self.virtual_pad_ids.clone(), password)?
                .join("|");
        let data = reqwest::multipart::Form::new()
            .text("form[fakePassword]", "••••••••")
            .text("form[ajx]", "1")
            .text("form[password]", self.password.clone())
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
            let status = res.status();
            let text = res.text().await?;
            if text.contains("Identifiant ou mot de passe invalide")
                || text.contains("Erreur d'authentification")
            {
                bail!(ClientError::InvalidCredentials);
            }
            error!("{}", text);
            bail!("Could not login to Bourso website, status code: {}", status);
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
            info!(
                "🔓 You are now logged in with user: {}",
                self.config.user_hash.as_ref().unwrap()
            );
        } else {
            if res.contains("/securisation") {
                bail!(ClientError::MfaRequired)
            }
            debug!("{}", res);

            bail!(ClientError::InvalidCredentials)
        }

        Ok(())
    }

    /// Request the MFA code to be sent to the user.
    ///
    /// # Returns
    /// * `otp_id` - The OTP ID tied to the MFA request.
    /// * `form_state` - The form state to use to check the MFA status.
    /// * `token_form` - The token form to use to validate the MFA process.
    /// * `mfa_type` - The type of MFA requested.
    pub async fn request_mfa(&mut self) -> Result<(String, String, String, MfaType)> {
        let _ = self
            .client
            .get(format!("{BASE_URL}/securisation"))
            .headers(self.get_headers())
            .send()
            .await?;

        let res = self
            .client
            .get(format!("{BASE_URL}/securisation/validation"))
            .headers(self.get_headers())
            .send()
            .await?;

        let res = res.text().await?;

        let mfa_type = if res.contains("brs-otp-webtoapp") {
            // We're only supporting web to app MFA for now
            // cause it seems like Bourso is deprecating SMS and email MFA as of January 2026
            MfaType::WebToApp
        } else {
            debug!("{}", res);
            let regex = Regex::new(r#"brs-otp-(?P<mfa_type>sms|email)"#).unwrap();
            let captures = regex.captures(&res);
            if captures.is_none() {
                error!("{}", res);
                bail!("Could not request MFA, MFA type not found");
            }
            // If one of the other MFA types is found, we bail as they are not supported
            let mfa_type_str = captures.unwrap().name("mfa_type").unwrap().as_str();
            bail!(
                "Could not request MFA, MFA type {} not supported",
                mfa_type_str
            );
        };

        self.config = extract_brs_config(&res)?;
        let (otp_id, form_state) = extract_otp_params(&res)?;

        let contact_number = match mfa_type {
            MfaType::WebToApp => "your phone app".to_string(),
            _ => extract_user_contact(&res)?,
        };
        let token_form = extract_token(&res)?;

        let url = format!(
            "{}/fr-FR/_user_/_{}_/session/challenge/{}/{}",
            self.config.api_url,
            self.config.user_hash.as_ref().unwrap(),
            mfa_type.start_path(),
            otp_id
        );
        debug!(
            "Requesting MFA {} to {} with url {}",
            mfa_type, contact_number, url
        );

        let payload = serde_json::json!({"formState": form_state});

        let res = self
            .client
            .post(url)
            .body(payload.to_string())
            .header("Content-Type", "application/json; charset=utf-8")
            .headers(self.get_headers())
            .send()
            .await?;

        if res.status() != 200 {
            bail!("Could not request MFA, status code: {}", res.status());
        }

        let body = res.text().await?;
        let json_body: serde_json::Value = serde_json::from_str(&body)?;
        if json_body["success"].as_bool().unwrap() {
            info!("{} MFA request sent to {}", mfa_type, contact_number);
        } else {
            error!("{}", json_body);
            bail!("Could not request MFA, response: {}", json_body);
        }

        Ok((otp_id.to_string(), form_state, token_form, mfa_type))
    }

    /// Check the MFA status
    ///
    /// # Arguments
    /// * `mfa_type` - The type of MFA to submit.
    /// * `otp_id` - The OTP ID tied to the MFA request.
    /// * `form_state` - The form state to use to check the MFA status.
    /// * `token_form` - The token form to use to submit the MFA completion.
    ///
    /// # Returns
    /// * `true` if the MFA was successfully submitted, `false` if the MFA is still pending.
    pub async fn check_mfa(
        &mut self,
        mfa_type: MfaType,
        otp_id: String,
        form_state: String,
        token_form: String,
    ) -> Result<bool> {
        let url = format!(
            "{}/_user_/_{}_/session/challenge/{}/{}",
            self.config.api_url,
            self.config.user_hash.as_ref().unwrap(),
            mfa_type.check_path(),
            otp_id
        );
        debug!("Checking MFA status to {}", url);

        let payload = serde_json::json!({
            "formState": form_state
        });
        let res = self
            .client
            .post(url)
            .body(payload.to_string())
            .header("Content-Type", "application/json; charset=utf-8")
            .send()
            .await?;

        let status_code = res.status();

        if status_code != 200 {
            let body = res.text().await?;
            error!("{}", body);
            bail!("Could not submit MFA code, status code: {}", status_code);
        }

        let body = res.text().await?;
        let json_body: serde_json::Value = serde_json::from_str(&body)?;

        if json_body["success"].as_bool().unwrap() {
            debug!("Submitting form with token: {}", token_form);

            let params = [("form[_token]", token_form)];

            let res = self
                .client
                .post(format!("{BASE_URL}/securisation/validation"))
                .form(&params)
                .header("Host", "clients.boursobank.com")
                .header(
                    "accept",
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header("origin", "https://clients.boursobank.com")
                .header("sec-fetch-site", "same-origin")
                .header("sec-fetch-mode", "navigate")
                .headers(self.get_headers())
                .header(
                    "referer",
                    "https://clients.boursobank.com/securisation/validation",
                )
                .header("sec-fetch-dest", "document")
                .header("accept-language", "fr-FR,fr;q=0.9")
                .header("priority", "u=0, i")
                .send()
                .await?;

            if res.status() != 302 {
                // bail!("Could not submit MFA validation, status code: {}", res.status());
                println!(
                    "Could not submit MFA validation, status code: {}",
                    res.status()
                );
            }

            let res = self
                .client
                .get(format!("{BASE_URL}/"))
                .headers(self.get_headers())
                .header("referer", format!("{BASE_URL}/securisation/validation"))
                .header("accept-language", "fr-FR,fr;q=0.9")
                .send()
                .await?
                .text()
                .await?;

            if res.contains(r#"href="/se-deconnecter""#) {
                // Update the config with user hash
                self.config = extract_brs_config(&res)?;
                info!(
                    "🔓 You are now logged in with user: {}",
                    self.config.user_hash.as_ref().unwrap()
                );
            } else {
                if res.contains("/securisation") {
                    bail!(ClientError::MfaRequired);
                }

                bail!("Could not submit MFA, response: {}", res);
            }

            Ok(true)
        } else {
            debug!("⏳ MFA not yet validated");

            if json_body["qrcode"].is_string() {
                match qrcode::generate_qr_code(json_body["qrcode"].as_str().unwrap()) {
                    Ok(qr) => {
                        println!();
                        println!("{}", qrcode::render_to_terminal(&qr));
                        println!();
                    }
                    Err(e) => bail!("Could not render the QR code {}", e),
                }
                info!(
                    "Please scan the latest QR code in your BoursoBank app to validate the login request."
                );
            }

            Ok(false)
        }
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
fn extract_brs_mit_cookie(res: &str) -> Result<String> {
    let brs_mit_cookie = BRS_MIT_COOKIE_REGEX
        .captures(&res)
        .and_then(|c| c.name("brs_mit_cookie"))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| {
            error!("{}", res);
            anyhow::anyhow!("Could not extract brs mit cookie")
        })?;

    Ok(brs_mit_cookie)
}

fn extract_token(res: &str) -> Result<String> {
    let token = TOKEN_REGEX
        .captures(&res)
        .and_then(|c| c.name("token"))
        .map(|m| m.as_str().trim().to_string())
        .ok_or_else(|| {
            error!("{}", res);
            anyhow::anyhow!("Could not extract form token")
        })?;

    Ok(token)
}

/// Extract OTP parameters from the response string.
///
/// # Arguments
/// * `res` - The response string to extract OTP parameters from.
/// # Returns
/// A tuple containing the resource ID and form state as strings.
fn extract_otp_params(res: &str) -> Result<(String, String)> {
    let captures = OTP_PARAMS_REGEX.captures(&res);

    let challenge_json = if let Some(captures) = captures {
        let challenge_str = captures.get(1)
            .map(|m| m.as_str())
            .ok_or_else(|| {
                error!("{}", res);
                anyhow::anyhow!("Could not extract authentication challenge from regex match")
            })?;
        // HTML decode the JSON string (replace &quot; with ")
        let decoded = challenge_str.replace("&quot;", "\"");
        serde_json::from_str::<serde_json::Value>(&decoded)?
    } else {
        error!("{}", res);
        bail!("Could not extract authentication challenge parameters");
    };

    Ok((
        challenge_json["challenges"][0]["parameters"]["formScreen"]["actions"]["check"]["api"]
            ["params"]["resourceId"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                error!("{}", res);
                anyhow::anyhow!("Could not extract resourceId")
            })?,
        challenge_json["challenges"][0]["parameters"]["formScreen"]["actions"]["check"]["api"]
            ["params"]["formState"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                error!("{}", res);
                anyhow::anyhow!("Could not extract formState")
            })?,
    ))
}

fn extract_user_contact(res: &str) -> Result<String> {
    let contact_user = USER_CONTACT_REGEX
        .captures(&res)
        .and_then(|c| c.name("contact_user"))
        .map(|m| m.as_str().trim().to_string())
        .ok_or_else(|| {
            error!("{}", res);
            anyhow::anyhow!("Could not extract user contact")
        })?;

    Ok(contact_user)
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
    fn test_extract_token() {
        let res = r#"data-backspace><i class="form-row-circles-password__backspace-icon / c-icon c-icon--backspace u-block"></i></button></div></div></div><input  id="form_ajx" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[ajx]" value="1" ><input  autocomplete="off" aria-label="Renseignez votre mot de passe en sélectionnant les 8 chiffres sur le clavier virtuel accessible ci-après par votre liseuse." data-matrix-password="1" id="form_password" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[password]" value="" ><input  data-password-ack="1" id="form_passwordAck" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[passwordAck]" value="{&quot;js&quot;:false}" ><input  data-authentication-factor-webauthn-detection="data-authentication-factor-webauthn-detection" id="form_platformAuthenticatorAvailable" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[platformAuthenticatorAvailable]" value="" ><input  data-matrix-random-challenge="1" id="form_matrixRandomChallenge" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[matrixRandomChallenge]" value="" ><input  id="form__token" type="hidden" class="c-field__input" data-brs-text-input="data-brs-text-input" name="form[_token]" value="45ed28b1-76ff-46a2-9202-0ee01928e6bb" ><hx:include id="hinclude__36d8139868f4bef54611a886784a3cbb"  src="/connexion/clavier-virtuel"><div data-matrix-placeholder class="sasmap sasmap--placeholder"><div class="bouncy-loader "><div class="bouncy-loader__balls"><div class="bouncy-loader__ball bouncy-loader__ball--left"></div><div class="bouncy-loader__ball bouncy-loader__ball--center"></div><div class="bouncy-loader__ball bouncy-loader__ball--right"></div></div></div></div></hx:include><div class="narrow-modal-window__input-container"><div class="u-text-center  o-vertical-interval-bottom "><div class="o-grid"><div class="o-grid__item"><button class="c-button--fancy c-button c-button--fancy u-1/1 c-button--primary"        type="submit"        data-login-submit       ><span class="c-button__text">Je me connecte</span></button></div><div class="o-grid__item  u-hidden" data-login-go-to-webauthn-wrapper><button class="c-button--fancy c-button c-button--fancy u-1/1 c-button--secondary"        type="button"        data-login-go-to-webauthn       ><span class="c-button__text">Clé de sécurité</span></button></div></div></div><div class="u-text-center"><a class="c-button--fancy c-button c-button--fancy u-1/1 c-button--tertiary c-button--link"        href="/connexion/mot-de-passe/retrouver"        data-pjax       ><span class="c-button__text">Mot de passe oublié ?</span></a></div></div><div class="narrow-modal-window__back-link"><button class="c-button--nav-back c-button u-1/1@xs-max c-button--text"        type="button"        data-login-back-to-login data-login-change-user-action="/connexion/oublier-identifiant"       ><span class="c-button__text"><div class="o-flex o-flex--align-center"><div class="c-button__icon"><svg xmlns="http://www.w3.org/2000/svg" width="7.8" height="14" viewBox="0 0 2.064 3.704"><path d="M1.712 3.644L.082 2.018a.212.212 0 0 1-.022-.02.206.206 0 0 1-.06-.146.206.206 0 0 1 .06-.147.212.212 0 0 1 .022-.019L1.712.06a.206.206 0 0 1 .291 0 .206.206 0 0 1 0 .291L.5 1.852l1.504 1.501a.206.206 0 0 1 0 .291.205.205 0 0 1-.146.06.205.205 0 0 1-.145-.06z"/></svg></div><div class="c-button__content">Mon identifiant</div></div></span></button></div></div><footer class="narrow-modal-footer narrow-modal-footer--mobile" data-transition-view-footer><div class="narrow-modal-footer__item narrow-modal-footer__item--mobile"><a href="" class="c-link c-link--icon c-link--pull-up c-link--subtle""#;
        let token = extract_token(&res).unwrap();
        assert_eq!(token, "45ed28b1-76ff-46a2-9202-0ee01928e6bb");
    }
}
