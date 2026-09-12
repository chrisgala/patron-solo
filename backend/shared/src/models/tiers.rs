use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for the tiers table
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::tiers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tier {
    /// Unique tier identifier
    pub id: Uuid,
    /// Display name of the tier
    pub name: String,
    /// Description of the tier's perks
    pub description: Option<String>,
    /// Ordering level: higher levels include everything below
    pub level: i32,
    /// Monthly price in cents
    pub price_cents: i32,
    /// ISO currency code
    pub currency: String,
    /// Stripe Product id backing this tier
    pub stripe_product_id: Option<String>,
    /// Stripe Price id backing this tier's recurring price
    pub stripe_price_id: Option<String>,
    /// Whether the tier is currently joinable
    pub is_active: bool,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
    /// Last update timestamp
    pub updated_at: NaiveDateTime,
}

/// API response model for a tier
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "a1b2c3d4-5e6f-7890-abcd-ef1234567890",
    "name": "Silver",
    "description": "Early access to all episodes",
    "level": 2,
    "priceCents": 1500,
    "currency": "usd",
    "isActive": true
}))]
pub struct TierResponse {
    /// Tier's unique identifier
    pub id: Uuid,
    /// Display name of the tier
    #[schema(example = "Silver")]
    pub name: String,
    /// Description of the tier's perks
    #[schema(example = "Early access to all episodes")]
    pub description: Option<String>,
    /// Ordering level: higher levels include everything below
    #[schema(example = 2)]
    pub level: i32,
    /// Monthly price in cents
    #[schema(example = 1500)]
    #[serde(rename = "priceCents")]
    pub price_cents: i32,
    /// ISO currency code
    #[schema(example = "usd")]
    pub currency: String,
    /// Whether the tier is currently joinable
    #[schema(example = true)]
    #[serde(rename = "isActive")]
    pub is_active: bool,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
}

impl From<Tier> for TierResponse {
    #[inline]
    fn from(tier: Tier) -> Self {
        Self {
            id: tier.id,
            name: tier.name,
            description: tier.description,
            level: tier.level,
            price_cents: tier.price_cents,
            currency: tier.currency,
            is_active: tier.is_active,
            created_at: Some(tier.created_at.and_utc()),
        }
    }
}

/// Response type for tier list endpoints
#[derive(Debug, Serialize, ToSchema)]
pub struct TiersListResponse(
    /// List of tiers ordered by level
    pub Vec<TierResponse>,
);

/// Request model for creating a tier
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "Silver",
    "description": "Early access to all episodes",
    "level": 2,
    "priceCents": 1500
}))]
pub struct CreateTierRequest {
    /// Display name for the tier
    #[schema(example = "Silver")]
    pub name: String,
    /// Description of the tier's perks
    #[schema(example = "Early access to all episodes")]
    pub description: Option<String>,
    /// Ordering level; must be unique among tiers
    #[schema(example = 2)]
    pub level: i32,
    /// Monthly price in cents
    #[schema(example = 1500)]
    #[serde(rename = "priceCents")]
    pub price_cents: i32,
}

/// Request model for updating a tier
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateTierRequest {
    /// New display name
    #[schema(example = "Silver")]
    pub name: Option<String>,
    /// New description
    pub description: Option<String>,
    /// New ordering level
    #[schema(example = 2)]
    pub level: Option<i32>,
    /// New monthly price in cents (creates a new Stripe Price)
    #[schema(example = 1500)]
    #[serde(rename = "priceCents")]
    pub price_cents: Option<i32>,
    /// Whether the tier is joinable
    #[serde(rename = "isActive")]
    pub is_active: Option<bool>,
}
