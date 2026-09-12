//! Web Push notifications via VAPID (RFC 8292), payload-less.
//!
//! Pushes carry no payload — the service worker wakes and fetches the latest
//! post itself — so only VAPID JWT signing is needed, not RFC 8291 payload
//! encryption.

use crate::errors::ServiceError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};

/// Push message TTL in seconds (how long push services queue the wake-up)
const PUSH_TTL_SECS: u32 = 86_400;

/// VAPID JWT lifetime in seconds (max allowed is 24h)
const JWT_LIFETIME_SECS: i64 = 43_200;

/// Web Push sender configured with the site's VAPID key pair
#[derive(Debug, Clone)]
pub struct PushService {
    /// VAPID public key, base64url (sent to browsers and in `k=`)
    public_key: String,
    /// VAPID private key scalar, base64url
    private_key: String,
    /// VAPID subject (mailto: or https: URL identifying the sender)
    subject: String,
    /// Shared HTTP client
    client: reqwest::Client,
}

/// Result of pushing to one endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    /// Accepted by the push service
    Delivered,
    /// Subscription is gone (404/410): delete it
    Gone,
    /// Other failure: keep the subscription, log the error
    Failed,
}

impl PushService {
    /// Build from `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`, `VAPID_SUBJECT` env
    /// vars. Returns None when not configured (push disabled).
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let public_key = std::env::var("VAPID_PUBLIC_KEY").ok()?;
        let private_key = std::env::var("VAPID_PRIVATE_KEY").ok()?;
        if public_key.trim().is_empty() || private_key.trim().is_empty() {
            return None;
        }
        Some(Self {
            public_key,
            private_key,
            subject: std::env::var("VAPID_SUBJECT")
                .unwrap_or_else(|_| "mailto:admin@localhost".to_owned()),
            client: reqwest::Client::new(),
        })
    }

    /// The VAPID public key browsers use for `PushManager.subscribe`
    #[must_use]
    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    /// Sign a VAPID JWT for the push service at `audience` (scheme://host)
    fn vapid_jwt(&self, audience: &str) -> Result<String, ServiceError> {
        let header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let exp = chrono::Utc::now().timestamp() + JWT_LIFETIME_SECS;
        let claims = serde_json::json!({
            "aud": audience,
            "exp": exp,
            "sub": self.subject,
        });
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
        let signing_input = format!("{header}.{claims}");

        let key_bytes = URL_SAFE_NO_PAD
            .decode(self.private_key.trim())
            .map_err(|_| ServiceError::Config("Invalid VAPID_PRIVATE_KEY".to_owned()))?;
        let signing_key = SigningKey::from_slice(&key_bytes)
            .map_err(|_| ServiceError::Config("Invalid VAPID_PRIVATE_KEY".to_owned()))?;
        let signature: Signature = signing_key.sign(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{signing_input}.{signature}"))
    }

    /// Send a payload-less push to one subscription endpoint
    pub async fn send_wakeup(&self, endpoint: &str) -> PushOutcome {
        let audience = match reqwest::Url::parse(endpoint) {
            Ok(url) => match (url.scheme(), url.host_str()) {
                (scheme, Some(host)) => format!("{scheme}://{host}"),
                _ => return PushOutcome::Failed,
            },
            Err(_) => return PushOutcome::Failed,
        };

        let Ok(jwt) = self.vapid_jwt(&audience) else {
            return PushOutcome::Failed;
        };

        let result = self
            .client
            .post(endpoint)
            .header(
                "Authorization",
                format!("vapid t={jwt},k={}", self.public_key),
            )
            .header("TTL", PUSH_TTL_SECS.to_string())
            .body(Vec::new())
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => PushOutcome::Delivered,
            Ok(response)
                if response.status() == reqwest::StatusCode::NOT_FOUND
                    || response.status() == reqwest::StatusCode::GONE =>
            {
                PushOutcome::Gone
            }
            Ok(response) => {
                tracing::warn!(status = %response.status(), "push send failed");
                PushOutcome::Failed
            }
            Err(error) => {
                tracing::warn!(%error, "push send failed");
                PushOutcome::Failed
            }
        }
    }
}
