use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model mapping a user to their Stripe customer
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::stripe_customers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StripeCustomer {
    /// The user this Stripe customer belongs to
    pub user_id: Uuid,
    /// Stripe customer id
    pub stripe_customer_id: String,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
}

/// Database model mirroring a Stripe subscription
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::subscriptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Subscription {
    /// Unique row identifier
    pub id: Uuid,
    /// Subscribing user
    pub user_id: Uuid,
    /// The tier subscribed to (None if the tier was deleted)
    pub tier_id: Option<Uuid>,
    /// Snapshot of the tier's level used for gating
    pub tier_level: i32,
    /// Stripe subscription id
    pub stripe_subscription_id: String,
    /// Stripe subscription status mirror (active, trialing, past_due, canceled, ...)
    pub status: String,
    /// End of the current billing period
    pub current_period_end: Option<NaiveDateTime>,
    /// Whether the subscription cancels at period end
    pub cancel_at_period_end: bool,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
    /// Last update timestamp
    pub updated_at: NaiveDateTime,
}

impl Subscription {
    /// Whether this subscription currently grants access
    #[inline]
    #[must_use]
    pub fn grants_access(&self) -> bool {
        matches!(self.status.as_str(), "active" | "trialing")
    }
}

/// Database model for one-off purchases of a post or series
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::purchases)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Purchase {
    /// Unique row identifier
    pub id: Uuid,
    /// Purchasing user
    pub user_id: Uuid,
    /// Purchased post (exclusive with `series_id`)
    pub post_id: Option<Uuid>,
    /// Purchased series (exclusive with `post_id`)
    pub series_id: Option<Uuid>,
    /// Stripe Checkout session that produced this purchase
    pub stripe_checkout_session_id: Option<String>,
    /// Stripe payment intent (idempotency key for fulfillment)
    pub stripe_payment_intent_id: Option<String>,
    /// Amount paid in cents
    pub amount_cents: Option<i32>,
    /// Purchase status (paid, refunded)
    pub status: String,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
}

/// Database model recording processed Stripe webhook events
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::stripe_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StripeEvent {
    /// Stripe event id
    pub id: String,
    /// Stripe event type
    pub event_type: String,
    /// When the event was processed
    pub processed_at: NaiveDateTime,
}

/// A fan's current subscription state, for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "tierId": "a1b2c3d4-5e6f-7890-abcd-ef1234567890",
    "tierLevel": 2,
    "status": "active",
    "cancelAtPeriodEnd": false,
    "currentPeriodEnd": "2023-02-01T00:00:00Z"
}))]
pub struct SubscriptionInfo {
    /// The subscribed tier's id (null if that tier was deleted)
    #[serde(rename = "tierId")]
    pub tier_id: Option<Uuid>,
    /// The subscribed tier's level
    #[serde(rename = "tierLevel")]
    pub tier_level: i32,
    /// Stripe status mirror
    #[schema(example = "active")]
    pub status: String,
    /// Whether the subscription ends at period end
    #[serde(rename = "cancelAtPeriodEnd")]
    pub cancel_at_period_end: bool,
    /// End of the current billing period
    #[serde(rename = "currentPeriodEnd")]
    pub current_period_end: Option<DateTime<Utc>>,
}

/// A single owned item in a fan's billing summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PurchaseInfo {
    /// Purchased post id, if a post purchase
    #[serde(rename = "postId")]
    pub post_id: Option<Uuid>,
    /// Purchased series id, if a series purchase
    #[serde(rename = "seriesId")]
    pub series_id: Option<Uuid>,
    /// Amount paid in cents
    #[serde(rename = "amountCents")]
    pub amount_cents: Option<i32>,
    /// When the purchase happened
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
}

/// A fan's complete billing state: subscription + owned purchases
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BillingMeResponse {
    /// Active or most recent subscription, if any
    pub subscription: Option<SubscriptionInfo>,
    /// One-off purchases owned by this fan
    pub purchases: Vec<PurchaseInfo>,
}

/// Request to start a subscription checkout
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"tierId": "a1b2c3d4-5e6f-7890-abcd-ef1234567890"}))]
pub struct SubscribeRequest {
    /// The tier to subscribe to
    #[serde(rename = "tierId")]
    pub tier_id: Uuid,
}

/// Request to start a one-off purchase checkout for a post or series
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"postId": "d290f1ee-6c54-4b01-90e6-d701748f0851"}))]
pub struct PurchaseRequest {
    /// Post to buy (exclusive with `seriesId`)
    #[serde(rename = "postId")]
    pub post_id: Option<Uuid>,
    /// Series to buy (exclusive with `postId`)
    #[serde(rename = "seriesId")]
    pub series_id: Option<Uuid>,
}

/// Response carrying a Stripe-hosted URL to redirect the fan to
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"url": "https://checkout.stripe.com/c/pay/cs_test_..."}))]
pub struct CheckoutUrlResponse {
    /// Stripe-hosted page URL
    pub url: String,
}
