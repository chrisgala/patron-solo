#![allow(clippy::unused_async)]

use actix_web::{web, HttpResponse, Result};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::auth::User,
};
use utoipa::ToSchema;

/// Public profile of the site's creator
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "displayName": "Jane Author",
    "avatarUrl": "https://example.com/avatar.jpg",
    "banner": "https://example.com/banner.jpg",
    "description": "I write a serialized novel."
}))]
pub struct CreatorProfile {
    /// Creator's display name
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    /// URL to the creator's avatar image
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    /// URL to the creator's banner image
    pub banner: Option<String>,
    /// Creator's bio shown on the public site
    pub description: Option<String>,
}

/// Public site information: the creator's profile
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SiteResponse {
    /// The site creator's public profile, if a creator account exists
    pub creator: Option<CreatorProfile>,
}

/// Get public site information
///
/// Returns the single creator's public profile. Anonymous-accessible.
///
/// # Errors
/// Returns error if the database query fails
#[utoipa::path(
    get,
    path = "/api/public/site",
    tag = "Public",
    responses(
        (status = 200, description = "Public site information", body = SiteResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn get_site(
    db_service: web::Data<shared::services::db::DbService>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::users::dsl as users_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let creator: Option<User> = users_dsl::users
        .filter(users_dsl::role.eq("creator"))
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let profile = creator.map(|user| CreatorProfile {
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        banner: user.banner,
        description: user.description,
    });

    Ok(HttpResponse::Ok().json(SiteResponse { creator: profile }))
}
