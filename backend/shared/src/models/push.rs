use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for a browser's Web Push subscription
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::push_subscriptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PushSubscription {
    /// Unique row identifier
    pub id: Uuid,
    /// Owning user, if the browser was signed in when subscribing
    pub user_id: Option<Uuid>,
    /// Push service endpoint URL
    pub endpoint: String,
    /// Client public key (base64url)
    pub p256dh: String,
    /// Client auth secret (base64url)
    pub auth: String,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
}

/// Request to register a browser's push subscription
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "endpoint": "https://fcm.googleapis.com/fcm/send/...",
    "p256dh": "BASE64URL...",
    "auth": "BASE64URL..."
}))]
pub struct PushSubscribeRequest {
    /// Push service endpoint URL
    pub endpoint: String,
    /// Client public key (base64url)
    pub p256dh: String,
    /// Client auth secret (base64url)
    pub auth: String,
}

/// Request to remove a browser's push subscription
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PushUnsubscribeRequest {
    /// Push service endpoint URL to remove
    pub endpoint: String,
}

/// Response carrying the server's VAPID public key
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PushKeyResponse {
    /// VAPID public key (base64url), for `PushManager.subscribe`
    #[serde(rename = "publicKey")]
    pub public_key: String,
}
