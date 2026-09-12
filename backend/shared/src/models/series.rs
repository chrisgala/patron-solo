use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for series table
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::series)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Series {
    /// Unique series identifier
    pub id: Uuid,
    /// ID of the user who owns this series
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    /// Title of the series
    pub title: String,
    /// Description of the series content and purpose
    pub description: Option<String>,
    /// URL-friendly slug for the series
    pub slug: String,
    /// Category or genre of the series
    pub category: Option<String>,
    /// URL to the series cover image
    #[serde(rename = "coverImageUrl")]
    pub cover_image_url: Option<String>,
    /// Timestamp when the series was created
    #[serde(rename = "createdAt")]
    pub created_at: Option<NaiveDateTime>,
    /// Timestamp when the series was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<NaiveDateTime>,
    /// Timestamp when the series was soft deleted (None if not deleted)
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<NaiveDateTime>,
    /// Bundle price in cents (None = not sellable as a bundle)
    #[serde(rename = "priceCents")]
    pub price_cents: Option<i32>,
    /// Stripe Price id backing the bundle purchase
    #[serde(rename = "stripePriceId")]
    pub stripe_price_id: Option<String>,
    /// Default minimum tier level applied to new posts in this series
    #[serde(rename = "minTierLevel")]
    pub min_tier_level: Option<i32>,
    /// Whether this is the hidden default feed series for standalone updates
    #[serde(rename = "isFeed")]
    pub is_feed: bool,
}

/// API response model for series
#[derive(Debug, Clone, Serialize, ToSchema)]
#[schema(example = json!({
    "id": "e5f6a7b8-9012-3456-ef01-345678901234",
    "userId": "f6a7b8c9-0123-4567-f012-456789012345",
    "title": "My Awesome Podcast",
    "description": "A weekly podcast about technology and innovation",
    "slug": "my-awesome-podcast",
    "category": "Technology",
    "coverImageUrl": "https://example.com/cover.jpg",
    "length": 42,
    "createdAt": "2023-01-01T00:00:00Z",
    "updatedAt": "2023-01-01T12:00:00Z"
}))]
pub struct SeriesResponse {
    /// Series unique identifier
    #[schema(example = "e5f6a7b8-9012-3456-ef01-345678901234")]
    pub id: Uuid,
    /// ID of the user who owns this series
    #[schema(example = "f6a7b8c9-0123-4567-f012-456789012345")]
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    /// Display name of the series
    #[schema(example = "My Awesome Podcast")]
    pub title: String,
    /// Description of the series content and purpose
    #[schema(example = "A weekly podcast about technology and innovation")]
    pub description: Option<String>,
    /// SEO-friendly URL identifier for the series
    #[schema(example = "my-awesome-podcast")]
    pub slug: String,
    /// Category or genre of the series
    #[schema(example = "Technology")]
    pub category: Option<String>,
    /// URL to the series cover image
    #[schema(example = "https://example.com/cover.jpg")]
    #[serde(rename = "coverImageUrl")]
    pub cover_image_url: Option<String>,
    /// Total number of posts in the series
    #[schema(example = 42)]
    pub length: Option<i32>,
    /// Series creation timestamp
    #[schema(example = "2023-01-01T00:00:00Z")]
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    /// Series last update timestamp
    #[schema(example = "2023-01-01T12:00:00Z")]
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Bundle price in cents (null = not sellable as a bundle)
    #[schema(example = 2000)]
    #[serde(rename = "priceCents")]
    pub price_cents: Option<i32>,
    /// Default minimum tier level applied to new posts in this series
    #[schema(example = 1)]
    #[serde(rename = "minTierLevel")]
    pub min_tier_level: Option<i32>,
    /// Whether this is the hidden default feed series
    #[schema(example = false)]
    #[serde(rename = "isFeed")]
    pub is_feed: bool,
}

impl From<Series> for SeriesResponse {
    fn from(series: Series) -> Self {
        Self {
            id: series.id,
            user_id: series.user_id,
            title: series.title,
            description: series.description,
            slug: series.slug,
            category: series.category,
            cover_image_url: series.cover_image_url,
            length: None, // Will be populated in handlers when needed
            created_at: series.created_at.map(|dt| dt.and_utc()),
            updated_at: series.updated_at.map(|dt| dt.and_utc()),
            price_cents: series.price_cents,
            min_tier_level: series.min_tier_level,
            is_feed: series.is_feed,
        }
    }
}

impl SeriesResponse {
    /// Create a `SeriesResponse` with length populated
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_length(mut self, length: Option<i32>) -> Self {
        self.length = length;
        self
    }
}

/// Request model for creating a new series
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(
    description = "Request payload for creating a new series with content and metadata",
    example = json!({
        "title": "My Awesome Podcast",
        "description": "A weekly podcast about technology and innovation",
        "slug": "my-awesome-podcast",
        "category": "Technology",
        "coverImageUrl": "https://example.com/cover.jpg"
    })
)]
pub struct CreateSeriesRequest {
    /// Name for the new series being created
    #[schema(example = "My Awesome Podcast")]
    pub title: String,
    /// Description of the series content and purpose (optional)
    #[schema(example = "A weekly podcast about technology and innovation")]
    pub description: Option<String>,
    /// Unique URL path segment for the new series
    #[schema(example = "my-awesome-podcast")]
    pub slug: String,
    /// Category or genre of the series (optional)
    #[schema(example = "Technology")]
    pub category: Option<String>,
    /// Banner image URL for the series homepage
    #[schema(example = "https://example.com/cover.jpg")]
    #[serde(rename = "coverImageUrl")]
    pub cover_image_url: Option<String>,
    /// Bundle price in cents (optional)
    #[schema(example = 2000)]
    #[serde(rename = "priceCents")]
    pub price_cents: Option<i32>,
    /// Default minimum tier level for new posts in this series (optional)
    #[schema(example = 1)]
    #[serde(rename = "minTierLevel")]
    pub min_tier_level: Option<i32>,
}

/// Request model for updating an existing series
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(
    description = "Request payload for updating series content, metadata, or status",
    example = json!({
        "title": "My Updated Podcast",
        "description": "An updated description with more details",
        "slug": "my-updated-podcast",
        "category": "Technology",
        "coverImageUrl": "https://example.com/new-cover.jpg"
    })
)]
pub struct UpdateSeriesRequest {
    /// New title for the series (optional)
    #[schema(example = "My Updated Podcast")]
    pub title: Option<String>,
    /// Updated description of the series (optional)
    #[schema(example = "An updated description with more details")]
    pub description: Option<String>,
    /// New URL-friendly slug for the series (optional)
    #[schema(example = "my-updated-podcast")]
    pub slug: Option<String>,
    /// Updated category or genre of the series (optional)
    #[schema(example = "Technology")]
    pub category: Option<String>,
    /// Update the series banner image with a new URL
    #[schema(example = "https://example.com/new-cover.jpg")]
    #[serde(rename = "coverImageUrl")]
    pub cover_image_url: Option<String>,
    /// Updated bundle price in cents; null leaves unchanged (use `clearPrice` to remove)
    #[schema(example = 2000)]
    #[serde(rename = "priceCents")]
    pub price_cents: Option<i32>,
    /// Remove the bundle price
    #[schema(example = false)]
    #[serde(rename = "clearPrice")]
    pub clear_price: Option<bool>,
    /// Updated default minimum tier level for new posts
    #[schema(example = 1)]
    #[serde(rename = "minTierLevel")]
    pub min_tier_level: Option<i32>,
    /// Remove the default tier gate
    #[schema(example = false)]
    #[serde(rename = "clearMinTier")]
    pub clear_min_tier: Option<bool>,
}

/// Response type for series list endpoints
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "List of series with pagination support", example = json!([{
    "id": "e5f6a7b8-9012-3456-ef01-345678901234",
    "userId": "f6a7b8c9-0123-4567-f012-456789012345",
    "title": "My Awesome Podcast",
    "description": "A weekly podcast about technology and innovation",
    "slug": "my-awesome-podcast",
    "category": "Technology",
    "coverImageUrl": "https://example.com/cover.jpg",
    "length": 42,
    "createdAt": "2023-01-01T00:00:00Z",
    "updatedAt": "2023-01-01T12:00:00Z"
}]))]
pub struct SeriesListResponse(
    /// List of series
    pub Vec<SeriesResponse>,
);

impl From<Vec<SeriesResponse>> for SeriesListResponse {
    fn from(series: Vec<SeriesResponse>) -> Self {
        Self(series)
    }
}
