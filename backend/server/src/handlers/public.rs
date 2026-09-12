#![allow(clippy::unused_async)]

use actix_web::{web, HttpResponse, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::{
        auth::MaybeUser,
        posts::{Post, PostKind},
        series::{Series, SeriesResponse},
    },
    services::entitlements::{
        can_access_post, load_user_entitlements, AccessReason, UserEntitlements,
    },
};
use utoipa::ToSchema;
use uuid::Uuid;

/// Access state attached to every public post response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "granted": false,
    "reason": null,
    "requiredTierLevel": 2,
    "priceCents": 500,
    "freeAt": "2023-02-01T00:00:00Z"
}))]
#[expect(missing_copy_implementations, reason = "API type, keep non-Copy for evolvability")]
pub struct PostAccess {
    /// Whether the requester may see the full post
    pub granted: bool,
    /// Why access was granted (null when locked)
    pub reason: Option<AccessReason>,
    /// Tier level that unlocks this post, if tier-gated
    #[serde(rename = "requiredTierLevel")]
    pub required_tier_level: Option<i32>,
    /// One-off price that unlocks this post, if purchasable
    #[serde(rename = "priceCents")]
    pub price_cents: Option<i32>,
    /// When the post rolls free, if scheduled
    #[serde(rename = "freeAt")]
    pub free_at: Option<DateTime<Utc>>,
}

/// A post as seen by the public site: full content when entitled, teaser otherwise
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicPostResponse {
    /// Post id
    pub id: Uuid,
    /// Parent series id
    #[serde(rename = "seriesId")]
    pub series_id: Uuid,
    /// Post title (always visible)
    pub title: String,
    /// Content kind
    pub kind: PostKind,
    /// URL slug
    pub slug: String,
    /// Position within the series
    #[serde(rename = "postNumber")]
    pub number: i32,
    /// Thumbnail URL (always visible)
    #[serde(rename = "thumbnailUrl")]
    pub thumbnail_url: Option<String>,
    /// Publication timestamp
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    /// Access decision for this requester
    pub access: PostAccess,
    /// Full content; None when locked
    pub content: Option<String>,
    /// Audio file id; None when locked
    #[serde(rename = "audioFileId")]
    pub audio_file_id: Option<Uuid>,
    /// Video file id; None when locked
    #[serde(rename = "videoFileId")]
    pub video_file_id: Option<Uuid>,
    /// Image gallery file ids; None when locked
    #[serde(rename = "imageFileIds")]
    pub image_file_ids: Option<Vec<Uuid>>,
}

/// Map a post through an entitlement decision into its public shape
fn to_public_post(post: Post, entitlements: &UserEntitlements) -> PublicPostResponse {
    let decision = can_access_post(entitlements, &post);
    let access = PostAccess {
        granted: decision.granted,
        reason: decision.reason,
        required_tier_level: post.min_tier_level,
        price_cents: post.price_cents,
        free_at: post.free_at.map(|dt| dt.and_utc()),
    };

    if decision.granted {
        PublicPostResponse {
            id: post.id,
            series_id: post.series_id,
            title: post.title,
            kind: post.kind.into(),
            slug: post.slug,
            number: post.number,
            thumbnail_url: post.thumbnail_url,
            created_at: post.created_at.map(|dt| dt.and_utc()),
            access,
            content: Some(post.content),
            audio_file_id: post.audio_file_id,
            video_file_id: post.video_file_id,
            image_file_ids: post
                .image_file_ids
                .map(|ids| ids.into_iter().flatten().collect()),
        }
    } else {
        PublicPostResponse {
            id: post.id,
            series_id: post.series_id,
            title: post.title,
            kind: post.kind.into(),
            slug: post.slug,
            number: post.number,
            thumbnail_url: post.thumbnail_url,
            created_at: post.created_at.map(|dt| dt.and_utc()),
            access,
            content: None,
            audio_file_id: None,
            video_file_id: None,
            image_file_ids: None,
        }
    }
}

/// Response type for the public post list
#[derive(Debug, Serialize, ToSchema)]
pub struct PublicPostsResponse(
    /// Posts, newest first
    pub Vec<PublicPostResponse>,
);

/// Query parameters for the public post list
#[derive(Debug, Clone, Copy, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PublicPostsQuery {
    /// Only posts belonging to this series
    #[serde(rename = "seriesId")]
    pub series_id: Option<Uuid>,
    /// Number of posts to skip
    pub offset: Option<i64>,
    /// Maximum number of posts to return (default 20, max 100)
    pub limit: Option<i64>,
}

/// List published posts for the public site
///
/// Locked posts come back as teasers (no content or media ids).
///
/// # Errors
/// Returns error if database queries fail
#[utoipa::path(
    get,
    path = "/api/public/posts",
    tag = "Public",
    params(PublicPostsQuery),
    responses(
        (status = 200, description = "Published posts, newest first", body = PublicPostsResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn list_public_posts(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
    query: web::Query<PublicPostsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::posts::dsl as posts_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut posts_query = posts_dsl::posts
        .filter(posts_dsl::is_published.eq(true))
        .filter(posts_dsl::deleted_at.is_null())
        .order(posts_dsl::created_at.desc())
        .offset(offset)
        .limit(limit)
        .into_boxed();
    if let Some(series_id) = query.series_id {
        posts_query = posts_query.filter(posts_dsl::series_id.eq(series_id));
    }

    let posts: Vec<Post> = posts_query
        .load(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let (user_id, is_creator) = match &user.0 {
        Some(user) => (Some(user.id), user.is_creator()),
        None => (None, false),
    };
    let entitlements = load_user_entitlements(&mut conn, user_id, is_creator).await?;

    Ok(HttpResponse::Ok().json(PublicPostsResponse(
        posts
            .into_iter()
            .map(|post| to_public_post(post, &entitlements))
            .collect(),
    )))
}

/// Get one published post by id or slug
///
/// Locked posts come back as teasers (no content or media ids).
///
/// # Errors
/// Returns error if the post is not found or database queries fail
#[utoipa::path(
    get,
    path = "/api/public/posts/{id_or_slug}",
    tag = "Public",
    params(("id_or_slug" = String, Path, description = "Post UUID or slug")),
    responses(
        (status = 200, description = "The post, teaser or full", body = PublicPostResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn get_public_post(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::posts::dsl as posts_dsl;

    let id_or_slug = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let mut post_query = posts_dsl::posts
        .filter(posts_dsl::is_published.eq(true))
        .filter(posts_dsl::deleted_at.is_null())
        .into_boxed();
    post_query = if let Ok(id) = Uuid::parse_str(&id_or_slug) {
        post_query.filter(posts_dsl::id.eq(id))
    } else {
        post_query.filter(posts_dsl::slug.eq(id_or_slug))
    };

    let post: Post = post_query
        .first(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ServiceError::NotFound("Post not found".to_owned()),
            _ => ServiceError::Database(e.to_string()),
        })?;

    let (user_id, is_creator) = match &user.0 {
        Some(user) => (Some(user.id), user.is_creator()),
        None => (None, false),
    };
    let entitlements = load_user_entitlements(&mut conn, user_id, is_creator).await?;

    Ok(HttpResponse::Ok().json(to_public_post(post, &entitlements)))
}

/// A series as seen by the public site
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PublicSeriesResponse {
    /// The series
    #[serde(flatten)]
    pub series: SeriesResponse,
    /// Whether the requester owns this series via purchase
    pub owned: bool,
}

/// Response type for the public series list
#[derive(Debug, Serialize, ToSchema)]
pub struct PublicSeriesListResponse(
    /// Visible series
    pub Vec<PublicSeriesResponse>,
);

/// List series for the public site (the hidden feed series is excluded)
///
/// # Errors
/// Returns error if database queries fail
#[utoipa::path(
    get,
    path = "/api/public/series",
    tag = "Public",
    responses(
        (status = 200, description = "Visible series", body = PublicSeriesListResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn list_public_series(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::series::dsl as series_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let series: Vec<Series> = series_dsl::series
        .filter(series_dsl::deleted_at.is_null())
        .filter(series_dsl::is_feed.eq(false))
        .order(series_dsl::created_at.desc())
        .load(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let (user_id, is_creator) = match &user.0 {
        Some(user) => (Some(user.id), user.is_creator()),
        None => (None, false),
    };
    let entitlements = load_user_entitlements(&mut conn, user_id, is_creator).await?;

    Ok(HttpResponse::Ok().json(PublicSeriesListResponse(
        series
            .into_iter()
            .map(|series| {
                let owned = entitlements.owned_series_ids.contains(&series.id);
                PublicSeriesResponse {
                    series: SeriesResponse::from(series),
                    owned,
                }
            })
            .collect(),
    )))
}

/// Get one series by id or slug for the public site
///
/// # Errors
/// Returns error if the series is not found or database queries fail
#[utoipa::path(
    get,
    path = "/api/public/series/{id_or_slug}",
    tag = "Public",
    params(("id_or_slug" = String, Path, description = "Series UUID or slug")),
    responses(
        (status = 200, description = "The series", body = PublicSeriesResponse),
        (status = 404, description = "Series not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn get_public_series(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::series::dsl as series_dsl;

    let id_or_slug = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let mut series_query = series_dsl::series
        .filter(series_dsl::deleted_at.is_null())
        .filter(series_dsl::is_feed.eq(false))
        .into_boxed();
    series_query = if let Ok(id) = Uuid::parse_str(&id_or_slug) {
        series_query.filter(series_dsl::id.eq(id))
    } else {
        series_query.filter(series_dsl::slug.eq(id_or_slug))
    };

    let series: Series = series_query
        .first(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ServiceError::NotFound("Series not found".to_owned())
            }
            _ => ServiceError::Database(e.to_string()),
        })?;

    let (user_id, is_creator) = match &user.0 {
        Some(user) => (Some(user.id), user.is_creator()),
        None => (None, false),
    };
    let entitlements = load_user_entitlements(&mut conn, user_id, is_creator).await?;
    let owned = entitlements.owned_series_ids.contains(&series.id);

    Ok(HttpResponse::Ok().json(PublicSeriesResponse {
        series: SeriesResponse::from(series),
        owned,
    }))
}
