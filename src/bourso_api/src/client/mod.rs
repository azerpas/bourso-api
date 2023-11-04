pub mod account;
pub mod config;
pub mod trade;
pub mod virtual_pad;

use std::sync::Arc;

use anyhow::{Result, Context, bail};
use regex::Regex;
use cookie_store::Cookie;
use reqwest::Response;
use reqwest_cookie_store::{CookieStoreMutex, CookieStore};


use self::config::{Config, extract_brs_config};

use super::{
    constants::{SAVINGS_PATTERN, ACCOUNT_PATTERN, BASE_URL, BANKING_PATTERN, TRADING_PATTERN, LOANS_PATTERN}, 
    account::{Account, AccountKind},
};

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
                .build().unwrap(),
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
        Ok(
            self.client
                .get(format!("{BASE_URL}/connexion/"))
                .headers(self.get_headers())
                .send()
                .await?
                .text()
                .await?
        )
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
                Cookie::parse( // Necessary cookie to remove the domain migration error
                    "brsDomainMigration=migrated;",
                    &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap()).unwrap(),
                &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap(),
            )?;
            store.insert(
                Cookie::parse( // Necessary cookie to access the virtual pad
                    format!("__brs_mit={};", self.brs_mit_cookie),
                    &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap()).unwrap(),
                &reqwest::Url::parse(&format!("{BASE_URL}/")).unwrap(),
            )?;
        }

        // We call the login page again to a form token
        let res = self.get_login_page().await?;

        self.token = extract_token(&res)?;
        self.config = extract_brs_config(&res)?;
        println!("Using version from {}", self.config.app_release_date);

        let res = self.client
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
    pub async fn login(&mut self, customer_id: &str, password: &str) -> Result<()> {
        self.customer_id = customer_id.to_string();
        self.password = virtual_pad::password_to_virtual_pad_keys(
            self.virtual_pad_ids.clone(), 
            password
        )?.join("|");
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

        let res = self.client
            .post(format!("{BASE_URL}/connexion/saisie-mot-de-passe"))
            .multipart(data)
            .headers(self.get_headers())
            .send()
            .await?;

        if res.status() != 302 {
            bail!("Could not login to Bourso website, status code: {}", res.status());
        }

        let res = self.client
            .get(format!("{BASE_URL}/"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        if res.contains(r#"href="/se-deconnecter""#) {
            // Update the config with user hash
            self.config = extract_brs_config(&res)?;
            println!("You are now logged in with user: {}", self.config.user_hash.as_ref().unwrap());
        } else {
            bail!("Could not login to Bourso website");
        }

        Ok(())
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
    let regex = Regex::new(r"(?m)__brs_mit=(?P<brs_mit_cookie>.*?);").unwrap();
    let brs_mit_cookie = regex
        .captures(&res)
        .unwrap()
        .name("brs_mit_cookie")
        .unwrap();

    Ok(brs_mit_cookie.as_str().to_string())
}

fn extract_token(res: &str) -> Result<String> {
    let regex = Regex::new(r#"(?ms)form\[_token\]"(.*?)value="(?P<token>.*?)"\s*>"#).unwrap();
    let token = regex
        .captures(&res)
        .unwrap()
        .name("token")
        .unwrap();

    Ok(token.as_str().trim().to_string())
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
