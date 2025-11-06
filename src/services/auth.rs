use std::sync::Arc;

use anyhow::{Context, Result};
use bourso_api::client::{error::ClientError, BoursoWebClient};
use tracing::{info, warn};

use crate::settings::SettingsStore;

// TODO: fix naming, too many mismatches with customer_id / username / client_id / client_number
// TODO: does it make sense to have MFA handling in the CLI?

pub trait CredentialsProvider: Send + Sync {
    fn read_password(&self, prompt: &str) -> Result<String>;
}

pub trait ClientFactory: Send + Sync {
    fn new_client(&self) -> BoursoWebClient;
}

pub struct AuthService {
    settings_store: Arc<dyn SettingsStore + Send + Sync>,
    credentials_provider: Arc<dyn CredentialsProvider>,
    client_factory: Arc<dyn ClientFactory>,
}

impl AuthService {
    pub fn new(
        settings_store: Arc<dyn SettingsStore + Send + Sync>,
        credentials_provider: Arc<dyn CredentialsProvider>,
        client_factory: Arc<dyn ClientFactory>,
    ) -> Self {
        Self {
            settings_store,
            credentials_provider,
            client_factory,
        }
    }

    pub fn with_defaults(store: Arc<dyn SettingsStore + Send + Sync>) -> Self {
        Self::new(
            store,
            Arc::new(StdinCredentialsProvider),
            Arc::new(DefaultClientFactory),
        )
    }

    pub async fn login(&self) -> Result<Option<BoursoWebClient>> {
        let settings = self.settings_store.load()?;
        let Some(client_number) = settings.client_number else {
            warn!("No client number found in settings, please run `bourso config` to set it");
            return Ok(None);
        };

        info!("We'll try to log you in with your customer id: {client_number}");
        info!("If you want to change it, you can run `bourso config` to set it");
        println!();

        let password = match settings.password {
            Some(password) => password,
            None => {
                info!("We'll need your password to log you in. It will not be stored.");
                self.credentials_provider
                    .read_password("Enter your password (hidden):")
                    .context("Failed to read password")?
                    .trim()
                    .to_string()
            }
        };

        let mut client = self.client_factory.new_client();
        client.init_session().await?;
        match client.login(&client_number, &password).await {
            Ok(_) => {
                info!("Login successful ✅");
                Ok(Some(client))
            }
            Err(e) => {
                if let Some(ClientError::MfaRequired) = e.downcast_ref::<ClientError>() {
                    self.handle_mfa(client, &client_number, &password).await
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn handle_mfa(
        &self,
        mut client: BoursoWebClient,
        client_number: &str,
        password: &str,
    ) -> Result<Option<BoursoWebClient>> {
        let mut mfa_count = 0usize;
        loop {
            if mfa_count == 2 {
                warn!("MFA threshold reached. Reinitializing session and logging in again.");
                client.init_session().await?;
                client.login(client_number, password).await?;
                info!("Login successful ✅");
                return Ok(Some(client));
            }

            let (otp_id, token_form, mfa_type) = client.request_mfa().await?;
            let code = self
                .credentials_provider
                .read_password("Enter your MFA code (hidden):")
                .context("Failed to read MFA code")?
                .trim()
                .to_string();

            match client.submit_mfa(mfa_type, otp_id, code, token_form).await {
                Ok(_) => {
                    info!("MFA successfully submitted ✅");
                    return Ok(Some(client));
                }
                Err(e) => {
                    if let Some(ClientError::MfaRequired) = e.downcast_ref::<ClientError>() {
                        mfa_count += 1;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
}

pub struct StdinCredentialsProvider;
impl CredentialsProvider for StdinCredentialsProvider {
    fn read_password(&self, prompt: &str) -> Result<String> {
        println!("{prompt}");
        Ok(rpassword::read_password()?)
    }
}

pub struct DefaultClientFactory;
impl ClientFactory for DefaultClientFactory {
    fn new_client(&self) -> BoursoWebClient {
        bourso_api::get_client()
    }
}
