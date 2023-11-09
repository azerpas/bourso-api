use regex::Regex;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "API_HOST")]
    pub api_host: String,
    #[serde(rename = "API_PATH")]
    pub api_path: String,
    #[serde(rename = "API_ENV")]
    pub api_env: String,
    #[serde(rename = "API_URL")]
    pub api_url: String,
    #[serde(rename = "API_REFERER_FEATURE_ID")]
    pub api_referer_feature_id: String,
    #[serde(rename = "LOCALE")]
    pub locale: String,
    #[serde(rename = "SUBSCRIPTION_HOST")]
    pub subscription_host: String,
    #[serde(rename = "CUSTOMER_SUBSCRIPTION_HOST")]
    pub customer_subscription_host: String,
    #[serde(rename = "PROSPECT_SUBSCRIPTION_HOST")]
    pub prospect_subscription_host: String,
    #[serde(rename = "DEBUG")]
    pub debug: bool,
    #[serde(rename = "ENABLE_PROFILER")]
    pub enable_profiler: bool,
    #[serde(rename = "AUTHENTICATION_ENDPOINT")]
    pub authentication_endpoint: String,
    #[serde(rename = "app_customer_website_host")]
    pub app_customer_website_host: String,
    #[serde(rename = "app_portal_website_host")]
    pub app_portal_website_host: String,
    #[serde(rename = "pjax_enabled")]
    pub pjax_enabled: bool,
    #[serde(rename = "pjax_timeout")]
    pub pjax_timeout: i64,
    #[serde(rename = "pjax_offset_duration")]
    pub pjax_offset_duration: i64,
    #[serde(rename = "select_bar_autoclose_tooltip_timeout")]
    pub select_bar_autoclose_tooltip_timeout: i64,
    #[serde(rename = "app_release_date")]
    pub app_release_date: String,
    #[serde(rename = "USER_HASH")]
    pub user_hash: Option<String>,
    #[serde(rename = "JWT_TOKEN_ID")]
    pub jwt_token_id: String,
    #[serde(rename = "DEFAULT_API_BEARER")]
    pub default_api_bearer: String,
    #[serde(rename = "JAVASCRIPT_APPS_BEARER")]
    pub javascript_apps_bearer: JavascriptAppsBearer,
    #[serde(rename = "APPLICATION_NAME")]
    pub application_name: String,
    pub webauth: Webauth,
    #[serde(rename = "MARKETING_NAME")]
    pub marketing_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavascriptAppsBearer {
    #[serde(rename = "web_all_feedback_01")]
    pub web_all_feedback_01: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Webauth {
    pub prepare_path: String,
    pub valid_path: String,
}

pub fn extract_brs_config(res: &str) -> Result<Config> {
    let regex = Regex::new(r#"(?ms)window\.BRS_CONFIG\s*=\s*(?P<config>.*?);"#).unwrap();
    let config = regex
        .captures(&res)
        .unwrap()
        .name("config")
        .unwrap();

    let config: Config = serde_json::from_str(&config.as_str().trim())
        .with_context(|| format!("Could not deserialize BRS_CONFIG: {}", config.as_str().trim()))?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    const SCRIPT_CONFIG: &str = r#"<script src="/build/webpack.2f2df8ae5f6dea021fcd.js"></script><script>
    // json config
    window.BRS_CONFIG = {"API_HOST": "api.boursobank.com","API_PATH": "\/services\/api\/v1.7","API_ENV": "prod","API_URL": "https:\/\/api.boursobank.com\/services\/api\/v1.7","API_REFERER_FEATURE_ID": "customer.dashboard_home.web_fr_front_20","LOCALE": "fr-FR","SUBSCRIPTION_HOST": "souscrire.boursobank.com","CUSTOMER_SUBSCRIPTION_HOST": "souscrire.boursobank.com","PROSPECT_SUBSCRIPTION_HOST": "ouvrir-un-compte.boursobank.com","DEBUG": false,"ENABLE_PROFILER": false,"AUTHENTICATION_ENDPOINT": "\/connexion\/","app_customer_website_host": "clients.boursobank.com","app_portal_website_host": "www.boursorama.com","pjax_enabled": true,"pjax_timeout": 20000,"pjax_offset_duration": 350,"select_bar_autoclose_tooltip_timeout": 3000,"app_release_date": "2023-03-01T14:15:36+0100","USER_HASH": "61d55b52615fbdf","JWT_TOKEN_ID": "brsxds_61d55b52615fbdfb898a3731bba89b35","DEFAULT_API_BEARER": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJpc3MiOiJPbmxpbmUgSldUIEJ1aWxkZXIiLCJpYXQiOjE2OTgyNDU5MTksImV4cCI6MTcyOTc4MTkxOSwiYXVkIjoid3d3LmV4YW1wbGUuY29tIiwic3ViIjoianJvY2tldEBleGFtcGxlLmNvbSIsIkdpdmVuTmFtZSI6IkpvaG5ueSIsIlN1cm5hbWUiOiJSb2NrZXQiLCJFbWFpbCI6Impyb2NrZXRAZXhhbXBsZS5jb20iLCJSb2xlIjpbIk1hbmFnZXIiLCJQcm9qZWN0IEFkbWluaXN0cmF0b3IiXX0.bvXls6bqw_xGqA6V8DQMsZK92dMrV8K6hebWpEu5IF8MlEd4qmwmcchJBUT7oeBnSIp5TJHH5112ho548Sw57A","JAVASCRIPT_APPS_BEARER": {"web_all_feedback_01": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJPbmxpbmUgSldUIEJ1aWxkZXIiLCJpYXQiOjE2OTgyNDYwNjQsImV4cCI6MTcyOTc4MjA2NCwiYXVkIjoid3d3LmV4YW1wbGUuY29tIiwic3ViIjoianJvY2tldEBleGFtcGxlLmNvbSIsIkdpdmVuTmFtZSI6IkpvaG5ueSIsIlN1cm5hbWUiOiJSb2NrZXQiLCJFbWFpbCI6Impyb2NrZXRAZXhhbXBsZS5jb20iLCJSb2xlIjpbIk1hbmFnZXIiLCJQcm9qZWN0IEFkbWluaXN0cmF0b3IiXX0.748Vj3kpR8EPUkh2hDUV4dQTR-iFygxUTuQQp5fWwEg"},"APPLICATION_NAME": "web_fr_front_20","webauth": {"preparePath": "\/webauthn\/authentification\/preparation","validPath": "\/webauthn\/authentification\/validation"},"MARKETING_NAME": "BoursoBank"};
    // jquery ready safety belt
    window.$defer = [];
    window.$ = function (fn) { if (typeof fn === 'function') { window.$defer.push(fn); } };
</script>"#;

    #[test]
    fn test_extract_brs_config() {
        let config = super::extract_brs_config(SCRIPT_CONFIG).unwrap();
        assert_eq!(config.jwt_token_id, "brsxds_61d55b52615fbdfb898a3731bba89b35");
        assert_eq!(config.api_path, "/services/api/v1.7");
        assert_eq!(config.api_host, "api.boursobank.com");
        assert_eq!(config.api_url, "https://api.boursobank.com/services/api/v1.7");
        assert_eq!(config.api_env, "prod");
        assert_eq!(config.api_referer_feature_id, "customer.dashboard_home.web_fr_front_20");
        assert_eq!(config.locale, "fr-FR");
        assert_eq!(config.subscription_host, "souscrire.boursobank.com");
        assert_eq!(config.customer_subscription_host, "souscrire.boursobank.com");
        assert_eq!(config.prospect_subscription_host, "ouvrir-un-compte.boursobank.com");
        assert_eq!(config.debug, false);
        assert_eq!(config.enable_profiler, false);
        assert_eq!(config.authentication_endpoint, "/connexion/");
        assert_eq!(config.app_customer_website_host, "clients.boursobank.com");
        assert_eq!(config.app_portal_website_host, "www.boursorama.com");
        assert_eq!(config.pjax_enabled, true);
        assert_eq!(config.pjax_timeout, 20000);
        assert_eq!(config.pjax_offset_duration, 350);
        assert_eq!(config.select_bar_autoclose_tooltip_timeout, 3000);
        assert_eq!(config.app_release_date, "2023-03-01T14:15:36+0100");
        assert_eq!(config.user_hash.unwrap(), "61d55b52615fbdf");
        assert_eq!(config.application_name, "web_fr_front_20");
        assert_eq!(config.marketing_name, "BoursoBank");
        assert_eq!(config.webauth.prepare_path, "/webauthn/authentification/preparation");
        assert_eq!(config.webauth.valid_path, "/webauthn/authentification/validation");
        assert_eq!(config.default_api_bearer, "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJpc3MiOiJPbmxpbmUgSldUIEJ1aWxkZXIiLCJpYXQiOjE2OTgyNDU5MTksImV4cCI6MTcyOTc4MTkxOSwiYXVkIjoid3d3LmV4YW1wbGUuY29tIiwic3ViIjoianJvY2tldEBleGFtcGxlLmNvbSIsIkdpdmVuTmFtZSI6IkpvaG5ueSIsIlN1cm5hbWUiOiJSb2NrZXQiLCJFbWFpbCI6Impyb2NrZXRAZXhhbXBsZS5jb20iLCJSb2xlIjpbIk1hbmFnZXIiLCJQcm9qZWN0IEFkbWluaXN0cmF0b3IiXX0.bvXls6bqw_xGqA6V8DQMsZK92dMrV8K6hebWpEu5IF8MlEd4qmwmcchJBUT7oeBnSIp5TJHH5112ho548Sw57A");
    }
}