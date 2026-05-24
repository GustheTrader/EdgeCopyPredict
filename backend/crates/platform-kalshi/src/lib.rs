use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::{header::HeaderMap, Method};
use rsa::pss::SigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::{pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub mod data_api;
pub use data_api::{KalshiDataApiClient, KalshiDataApiTrade};
pub mod execution;
pub mod rate_limit;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KalshiAuth {
    pub api_key: String,
    /// PEM-encoded RSA private key (PKCS#1 or PKCS#8)
    pub api_secret: String,
}

impl KalshiAuth {
    pub fn from_pem(api_key: String, pem: &str) -> Result<Self> {
        parse_rsa_private_key(pem)
            .map_err(|e| anyhow::anyhow!("failed to parse Kalshi PEM private key: {}", e))?;
        Ok(Self {
            api_key,
            api_secret: pem.to_owned(),
        })
    }

    /// Build Kalshi REST auth headers.
    ///
    /// Kalshi signature spec:
    ///   KALSHI-ACCESS-KEY:       {api_key}
    ///   KALSHI-ACCESS-TIMESTAMP: {unix_ms}
    ///   KALSHI-ACCESS-SIGNATURE: base64( RSA-PSS-SHA256( "{timestamp}{METHOD}{path}" ) )
    ///
    /// `path` should be just the URL path, e.g. "/trade-api/v2/orders".
    /// If a full URL or host-prefixed string is passed, the path portion is extracted.
    pub fn headers(&self, method: &Method, path: &str) -> Result<HeaderMap> {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();

        let sign_path = extract_path(path);
        let message = format!("{}{}{}", ts_ms, method.as_str(), sign_path);

        let private_key = parse_rsa_private_key(&self.api_secret)
            .map_err(|e| anyhow::anyhow!("RSA key parse failed: {}", e))?;

        let signing_key = SigningKey::<Sha256>::new_with_salt_len(private_key, 32);
        let mut rng = rand::thread_rng();
        let sig = signing_key.sign_with_rng(&mut rng, message.as_bytes());
        let sig_b64 = BASE64.encode(sig.to_bytes().as_ref());

        let mut headers = HeaderMap::new();
        headers.insert("KALSHI-ACCESS-KEY", self.api_key.parse()?);
        headers.insert("KALSHI-ACCESS-TIMESTAMP", ts_ms.parse()?);
        headers.insert("KALSHI-ACCESS-SIGNATURE", sig_b64.parse()?);
        Ok(headers)
    }
}

/// Extract the path component from a URL or already-stripped path.
/// "https://host/trade-api/v2/orders" → "/trade-api/v2/orders"
/// "host/trade-api/v2/orders" → "/trade-api/v2/orders"
/// "/trade-api/v2/orders" → "/trade-api/v2/orders"
fn extract_path(input: &str) -> String {
    let without_scheme = input
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let path = if let Some(slash) = without_scheme.find('/') {
        &without_scheme[slash..]
    } else {
        "/"
    };
    // Strip query string for signing
    path.split('?').next().unwrap_or(path).to_owned()
}

fn parse_rsa_private_key(pem: &str) -> Result<RsaPrivateKey> {
    // dotenvy preserves literal \n in quoted env values; expand them to real newlines.
    let expanded;
    let trimmed = if pem.contains("\\n") {
        expanded = pem.replace("\\n", "\n");
        expanded.trim()
    } else {
        pem.trim()
    };
    if trimmed.contains("BEGIN RSA PRIVATE KEY") {
        RsaPrivateKey::from_pkcs1_pem(trimmed)
            .map_err(|e| anyhow::anyhow!("PKCS#1 parse error: {}", e))
    } else if trimmed.contains("BEGIN PRIVATE KEY") {
        RsaPrivateKey::from_pkcs8_pem(trimmed)
            .map_err(|e| anyhow::anyhow!("PKCS#8 parse error: {}", e))
    } else {
        anyhow::bail!("PEM key must be RSA PRIVATE KEY or PRIVATE KEY")
    }
}
