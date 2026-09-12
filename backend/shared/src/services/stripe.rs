//! Thin Stripe REST client covering the calls this app needs:
//! customers, products, prices, checkout sessions, billing-portal sessions,
//! and webhook signature verification.

use crate::errors::ServiceError;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

/// Base URL of the Stripe API
const STRIPE_API: &str = "https://api.stripe.com/v1";

/// Tolerated clock skew for webhook signature timestamps, in seconds
const WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Minimal Stripe API client backed by reqwest
#[derive(Debug, Clone)]
pub struct StripeService {
    /// Secret API key (sk_test_... / sk_live_...)
    secret_key: String,
    /// Webhook signing secret (whsec_...)
    webhook_secret: String,
    /// Shared HTTP client
    client: reqwest::Client,
}

/// A Stripe object id plus the fields we care about, per call
#[derive(Debug, Deserialize)]
pub struct StripeObject {
    /// Object id (cus_..., prod_..., price_..., cs_...)
    pub id: String,
    /// Hosted page URL (checkout / billing portal sessions)
    #[serde(default)]
    pub url: Option<String>,
}

impl StripeService {
    /// Build from `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` env vars.
    /// Returns None when Stripe is not configured (payments disabled).
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let secret_key = std::env::var("STRIPE_SECRET_KEY").ok()?;
        if secret_key.trim().is_empty() {
            return None;
        }
        Some(Self {
            secret_key,
            webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default(),
            client: reqwest::Client::new(),
        })
    }

    /// POST a form-encoded request to a Stripe endpoint and parse the response
    async fn post(
        &self,
        path: &str,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, ServiceError> {
        let response = self
            .client
            .post(format!("{STRIPE_API}{path}"))
            .basic_auth(&self.secret_key, None::<&str>)
            .form(params)
            .send()
            .await?;

        let status = response.status();
        let body: serde_json::Value = response.json().await?;
        if status.is_success() {
            Ok(body)
        } else {
            let message = body
                .pointer("/error/message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Stripe API error");
            Err(ServiceError::Unknown(format!("Stripe: {message}")))
        }
    }

    /// Create a Stripe customer for a user
    ///
    /// # Errors
    /// Returns an error when the Stripe API call fails
    pub async fn create_customer(
        &self,
        email: &str,
        user_id: &str,
    ) -> Result<String, ServiceError> {
        let body = self
            .post(
                "/customers",
                &[
                    ("email".to_owned(), email.to_owned()),
                    ("metadata[user_id]".to_owned(), user_id.to_owned()),
                ],
            )
            .await?;
        let object: StripeObject = serde_json::from_value(body)?;
        Ok(object.id)
    }

    /// Create a product representing a tier or sellable item
    ///
    /// # Errors
    /// Returns an error when the Stripe API call fails
    pub async fn create_product(&self, name: &str) -> Result<String, ServiceError> {
        let body = self
            .post("/products", &[("name".to_owned(), name.to_owned())])
            .await?;
        let object: StripeObject = serde_json::from_value(body)?;
        Ok(object.id)
    }

    /// Create a price. `recurring_monthly` = true for tier prices, false for one-offs.
    ///
    /// # Errors
    /// Returns an error when the Stripe API call fails
    pub async fn create_price(
        &self,
        product_id: &str,
        amount_cents: i32,
        currency: &str,
        recurring_monthly: bool,
    ) -> Result<String, ServiceError> {
        let mut params = vec![
            ("product".to_owned(), product_id.to_owned()),
            ("unit_amount".to_owned(), amount_cents.to_string()),
            ("currency".to_owned(), currency.to_owned()),
        ];
        if recurring_monthly {
            params.push(("recurring[interval]".to_owned(), "month".to_owned()));
        }
        let body = self.post("/prices", &params).await?;
        let object: StripeObject = serde_json::from_value(body)?;
        Ok(object.id)
    }

    /// Archive a price so it can no longer be used for new checkouts
    ///
    /// # Errors
    /// Returns an error when the Stripe API call fails
    pub async fn archive_price(&self, price_id: &str) -> Result<(), ServiceError> {
        let _ = self
            .post(
                &format!("/prices/{price_id}"),
                &[("active".to_owned(), "false".to_owned())],
            )
            .await?;
        Ok(())
    }

    /// Create a checkout session; returns the hosted URL to redirect to.
    /// `mode` is "subscription" or "payment"; metadata is attached for webhook fulfillment.
    ///
    /// # Errors
    /// Returns an error when the Stripe API call fails or no URL is returned
    pub async fn create_checkout_session(
        &self,
        customer_id: &str,
        price_id: &str,
        mode: &str,
        success_url: &str,
        cancel_url: &str,
        metadata: &[(&str, String)],
    ) -> Result<String, ServiceError> {
        let mut params = vec![
            ("customer".to_owned(), customer_id.to_owned()),
            ("mode".to_owned(), mode.to_owned()),
            ("line_items[0][price]".to_owned(), price_id.to_owned()),
            ("line_items[0][quantity]".to_owned(), "1".to_owned()),
            ("success_url".to_owned(), success_url.to_owned()),
            ("cancel_url".to_owned(), cancel_url.to_owned()),
        ];
        for (key, value) in metadata {
            params.push((format!("metadata[{key}]"), value.clone()));
            if mode == "subscription" {
                params.push((format!("subscription_data[metadata][{key}]"), value.clone()));
            }
        }
        let body = self.post("/checkout/sessions", &params).await?;
        let object: StripeObject = serde_json::from_value(body)?;
        object
            .url
            .ok_or_else(|| ServiceError::Unknown("Stripe: checkout session has no URL".to_owned()))
    }

    /// Create a billing-portal session for subscription self-service
    ///
    /// # Errors
    /// Returns an error when the Stripe API call fails or no URL is returned
    pub async fn create_portal_session(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> Result<String, ServiceError> {
        let body = self
            .post(
                "/billing_portal/sessions",
                &[
                    ("customer".to_owned(), customer_id.to_owned()),
                    ("return_url".to_owned(), return_url.to_owned()),
                ],
            )
            .await?;
        let object: StripeObject = serde_json::from_value(body)?;
        object
            .url
            .ok_or_else(|| ServiceError::Unknown("Stripe: portal session has no URL".to_owned()))
    }

    /// Verify a webhook payload against its `Stripe-Signature` header and
    /// return the parsed event JSON.
    ///
    /// # Errors
    /// Returns `ServiceError::Forbidden` when the signature is missing,
    /// malformed, expired, or does not match.
    pub fn verify_webhook(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<serde_json::Value, ServiceError> {
        let mut timestamp: Option<i64> = None;
        let mut signatures: Vec<&str> = Vec::new();
        for part in signature_header.split(',') {
            match part.split_once('=') {
                Some(("t", value)) => timestamp = value.parse().ok(),
                Some(("v1", value)) => signatures.push(value),
                _ => {}
            }
        }
        let timestamp = timestamp
            .ok_or_else(|| ServiceError::Forbidden("Invalid Stripe signature".to_owned()))?;

        let now = chrono::Utc::now().timestamp();
        if (now - timestamp).abs() > WEBHOOK_TOLERANCE_SECS {
            return Err(ServiceError::Forbidden(
                "Stripe signature timestamp out of tolerance".to_owned(),
            ));
        }

        let mut mac = Hmac::<Sha256>::new_from_slice(self.webhook_secret.as_bytes())
            .map_err(|_| ServiceError::Forbidden("Invalid webhook secret".to_owned()))?;
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());

        if !signatures.iter().any(|sig| {
            sig.len() == expected.len()
                && sig
                    .bytes()
                    .zip(expected.bytes())
                    .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
                    == 0
        }) {
            return Err(ServiceError::Forbidden(
                "Stripe signature mismatch".to_owned(),
            ));
        }

        Ok(serde_json::from_slice(payload)?)
    }
}
