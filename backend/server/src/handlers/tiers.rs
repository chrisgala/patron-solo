#![allow(clippy::unused_async)]

use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::{
        auth::Creator,
        tiers::{CreateTierRequest, Tier, TierResponse, TiersListResponse, UpdateTierRequest},
    },
    services::stripe::StripeService,
};
use uuid::Uuid;

/// List active tiers, ordered by level (public)
///
/// # Errors
/// Returns error if the database query fails
#[utoipa::path(
    get,
    path = "/api/public/tiers",
    tag = "Public",
    responses(
        (status = 200, description = "Active tiers ordered by level", body = TiersListResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn list_public_tiers(
    db_service: web::Data<shared::services::db::DbService>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::tiers::dsl as tiers_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let tiers: Vec<Tier> = tiers_dsl::tiers
        .filter(tiers_dsl::is_active.eq(true))
        .order(tiers_dsl::level.asc())
        .load(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TiersListResponse(
        tiers.into_iter().map(TierResponse::from).collect(),
    )))
}

/// List all tiers including inactive ones (creator)
///
/// # Errors
/// Returns error if the database query fails
#[utoipa::path(
    get,
    path = "/api/tiers",
    tag = "Tiers",
    responses(
        (status = 200, description = "All tiers ordered by level", body = TiersListResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Creator access required", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn list_tiers(
    _user: Creator,
    db_service: web::Data<shared::services::db::DbService>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::tiers::dsl as tiers_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let tiers: Vec<Tier> = tiers_dsl::tiers
        .order(tiers_dsl::level.asc())
        .load(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TiersListResponse(
        tiers.into_iter().map(TierResponse::from).collect(),
    )))
}

/// Create a tier and its backing Stripe product/price (creator)
///
/// # Errors
/// Returns error if Stripe calls or database writes fail
#[utoipa::path(
    post,
    path = "/api/tiers",
    tag = "Tiers",
    request_body(content = CreateTierRequest, description = "Tier definition"),
    responses(
        (status = 201, description = "Tier created", body = TierResponse),
        (status = 400, description = "Invalid tier data", body = ErrorResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Creator access required", body = ErrorResponse),
        (status = 409, description = "A tier with this level already exists", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn create_tier(
    _user: Creator,
    db_service: web::Data<shared::services::db::DbService>,
    stripe: web::Data<Option<StripeService>>,
    body: web::Json<CreateTierRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::tiers::dsl as tiers_dsl;

    if body.price_cents <= 0 {
        return Err(ServiceError::Unknown("Price must be positive".to_owned()).into());
    }

    let (stripe_product_id, stripe_price_id) = match stripe.as_ref() {
        Some(client) => {
            let product_id = client.create_product(&body.name).await?;
            let price_id = client
                .create_price(&product_id, body.price_cents, "usd", true)
                .await?;
            (Some(product_id), Some(price_id))
        }
        None => (None, None),
    };

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let new_tier = Tier {
        id: Uuid::new_v4(),
        name: body.name.clone(),
        description: body.description.clone(),
        level: body.level,
        price_cents: body.price_cents,
        currency: "usd".to_owned(),
        stripe_product_id,
        stripe_price_id,
        is_active: true,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    let inserted: Tier = diesel::insert_into(tiers_dsl::tiers)
        .values(&new_tier)
        .get_result(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => ServiceError::Conflict("A tier with this level already exists".to_owned()),
            _ => ServiceError::Database(e.to_string()),
        })?;

    Ok(HttpResponse::Created().json(TierResponse::from(inserted)))
}

/// Update a tier; a price change creates a new Stripe price (creator)
///
/// Existing subscribers keep their old price; new checkouts use the new one.
///
/// # Errors
/// Returns error if Stripe calls or database writes fail
#[utoipa::path(
    put,
    path = "/api/tiers/{tier_id}",
    tag = "Tiers",
    params(("tier_id" = Uuid, Path, description = "UUID of the tier to update")),
    request_body(content = UpdateTierRequest, description = "Fields to update"),
    responses(
        (status = 200, description = "Tier updated", body = TierResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Creator access required", body = ErrorResponse),
        (status = 404, description = "Tier not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn update_tier(
    _user: Creator,
    db_service: web::Data<shared::services::db::DbService>,
    stripe: web::Data<Option<StripeService>>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTierRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::tiers::dsl as tiers_dsl;

    let tier_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let existing: Tier = tiers_dsl::tiers
        .filter(tiers_dsl::id.eq(tier_id))
        .first(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ServiceError::NotFound("Tier not found".to_owned()),
            _ => ServiceError::Database(e.to_string()),
        })?;

    // A price change mints a new Stripe price and archives the old one
    let mut new_stripe_price: Option<String> = None;
    if let Some(price_cents) = body.price_cents {
        if price_cents != existing.price_cents {
            if let (Some(client), Some(product_id)) =
                (stripe.as_ref(), existing.stripe_product_id.as_ref())
            {
                let price_id = client
                    .create_price(product_id, price_cents, &existing.currency, true)
                    .await?;
                if let Some(old_price) = existing.stripe_price_id.as_ref() {
                    if let Err(error) = client.archive_price(old_price).await {
                        tracing::warn!(%error, "failed to archive old Stripe price");
                    }
                }
                new_stripe_price = Some(price_id);
            }
        }
    }

    let updated: Tier = diesel::update(tiers_dsl::tiers.filter(tiers_dsl::id.eq(tier_id)))
        .set((
            body.name.as_ref().map(|v| tiers_dsl::name.eq(v)),
            body.description
                .as_ref()
                .map(|v| tiers_dsl::description.eq(v)),
            body.level.map(|v| tiers_dsl::level.eq(v)),
            body.price_cents.map(|v| tiers_dsl::price_cents.eq(v)),
            body.is_active.map(|v| tiers_dsl::is_active.eq(v)),
            new_stripe_price
                .as_ref()
                .map(|v| tiers_dsl::stripe_price_id.eq(v)),
            tiers_dsl::updated_at.eq(Utc::now().naive_utc()),
        ))
        .get_result(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TierResponse::from(updated)))
}

/// Deactivate a tier so no new fans can join it (creator)
///
/// Existing subscriptions are untouched; gating still compares levels.
///
/// # Errors
/// Returns error if the database write fails
#[utoipa::path(
    delete,
    path = "/api/tiers/{tier_id}",
    tag = "Tiers",
    params(("tier_id" = Uuid, Path, description = "UUID of the tier to deactivate")),
    responses(
        (status = 204, description = "Tier deactivated"),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Creator access required", body = ErrorResponse),
        (status = 404, description = "Tier not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn delete_tier(
    _user: Creator,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::tiers::dsl as tiers_dsl;

    let tier_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let affected = diesel::update(tiers_dsl::tiers.filter(tiers_dsl::id.eq(tier_id)))
        .set((
            tiers_dsl::is_active.eq(false),
            tiers_dsl::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    if affected == 0 {
        return Err(ServiceError::NotFound("Tier not found".to_owned()).into());
    }

    Ok(HttpResponse::NoContent().finish())
}
