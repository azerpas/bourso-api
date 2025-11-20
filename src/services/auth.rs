use anyhow::Result;
use std::io::{stdout, Write};
use tracing::{info, warn};

use crate::settings::SettingsStore;
use bourso_api::{
    client::{error::ClientError, BoursoWebClient},
    types::{ClientNumber, MfaCode, Password},
};

// TODO: does it make sense to have MFA handling in the CLI?

pub trait CredentialsProvider {
    fn read_password(&self) -> Result<Password>;
    fn read_mfa_code(&self) -> Result<MfaCode>;
}
pub struct StdinCredentialsProvider;
impl CredentialsProvider for StdinCredentialsProvider {
    fn read_password(&self) -> Result<Password> {
        print!("\nEnter your password (hidden): ");
        let _ = stdout().flush();
        let password = Password::new(&rpassword::read_password()?)?;
        println!();
        Ok(password)
    }
    fn read_mfa_code(&self) -> Result<MfaCode> {
        print!("\nEnter your MFA code (hidden): ");
        let _ = stdout().flush();
        let mfa_code = MfaCode::new(&rpassword::read_password()?)?;
        println!();
        Ok(mfa_code)
    }
}

pub trait ClientFactory {
    fn new_client(&self) -> BoursoWebClient;
}
pub struct DefaultClientFactory;
impl ClientFactory for DefaultClientFactory {
    fn new_client(&self) -> BoursoWebClient {
        bourso_api::get_client()
    }
}

pub struct AuthService<'a> {
    settings_store: &'a dyn SettingsStore,
    credentials_provider: Box<dyn CredentialsProvider>,
    client_factory: Box<dyn ClientFactory>,
}

impl<'a> AuthService<'a> {
    pub fn new(
        settings_store: &'a dyn SettingsStore,
        credentials_provider: Box<dyn CredentialsProvider>,
        client_factory: Box<dyn ClientFactory>,
    ) -> Self {
        Self {
            settings_store,
            credentials_provider,
            client_factory,
        }
    }

    pub fn with_defaults(settings_store: &'a dyn SettingsStore) -> Self {
        Self::new(
            settings_store,
            Box::new(StdinCredentialsProvider),
            Box::new(DefaultClientFactory),
        )
    }

    pub async fn login(&self) -> Result<Option<BoursoWebClient>> {
        let settings = self.settings_store.load()?;
        let Some(client_number) = settings.client_number.as_ref() else {
            warn!("No client number found in settings, please run `bourso-cli config` to set it");
            return Ok(None);
        };

        info!(
            "We'll try to log you in with your customer id: {:?}",
            client_number.as_ref()
        );
        info!("If you want to change it, you can run `bourso-cli config` to set it");
        println!();

        let password = match settings.password.as_ref() {
            Some(password) => password,
            None => {
                info!("We'll need your password to log you in. It will not be stored.");
                &self.credentials_provider.read_password()?
            }
        };

        let mut client = self.client_factory.new_client();
        client.init_session().await?;
        match client
            .login(client_number.as_ref(), password.as_ref())
            .await
        {
            Ok(_) => {
                info!("Login successful ✅");
                Ok(Some(client))
            }
            Err(e) => {
                if let Some(ClientError::MfaRequired) = e.downcast_ref::<ClientError>() {
                    self.handle_mfa(client, client_number, password).await
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn handle_mfa(
        &self,
        mut client: BoursoWebClient,
        client_number: &ClientNumber,
        password: &Password,
    ) -> Result<Option<BoursoWebClient>> {
        let mut mfa_count = 0usize;
        loop {
            if mfa_count == 2 {
                warn!("MFA threshold reached. Reinitializing session and logging in again.");
                client.init_session().await?;
                client
                    .login(client_number.as_ref(), password.as_ref())
                    .await?;
                info!("Login successful ✅");
                return Ok(Some(client));
            }

            let (otp_id, token_form, mfa_type) = client.request_mfa().await?;
            let code = &self.credentials_provider.read_mfa_code()?;

            match client
                .submit_mfa(mfa_type, otp_id, code.as_ref().to_string(), token_form)
                .await
            {
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
