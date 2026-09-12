#![allow(clippy::unused_async)]

use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::{
        auth::User,
        billing::{
            BillingMeResponse, CheckoutUrlResponse, PurchaseInfo, PurchaseRequest,
            StripeCustomer, SubscribeRequest, Subscription, SubscriptionInfo,
        },
        posts::Post,
        series::Series,
        tiers::Tier,
    },
    services::stripe::StripeService,
};

/// The public frontend base URL for Stripe redirect targets
fn frontend_url() -> String {
    std::env::var("APPLICATION_FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:5173".to_owned())
}

/// Get the Stripe client or fail with a clear configuration error
fn require_stripe(stripe: &web::Data<Option<StripeService>>) -> Result<&StripeService, ServiceError> {
    stripe
        .as_ref()
        .as_ref()
        .ok_or_else(|| ServiceError::Config("Stripe is not configured".to_owned()))
}

/// Find or create the Stripe customer for a user
async fn ensure_customer(
    conn: &mut AsyncPgConnection,
    stripe: &StripeService,
    user: &User,
) -> Result<String, ServiceError> {
    use shared::schema::stripe_customers::dsl as customers_dsl;

    let existing: Option<StripeCustomer> = customers_dsl::stripe_customers
        .filter(customers_dsl::user_id.eq(user.id))
        .first(conn)
        .await
        .optional()
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    if let Some(customer) = existing {
        return Ok(customer.stripe_customer_id);
    }

    let customer_id = stripe
        .create_customer(&user.email, &user.id.to_string())
        .await?;

    let record = StripeCustomer {
        user_id: user.id,
        stripe_customer_id: customer_id.clone(),
        created_at: Utc::now().naive_utc(),
    };
    let _ = diesel::insert_into(customers_dsl::stripe_customers)
        .values(&record)
        .on_conflict(customers_dsl::user_id)
        .do_nothing()
        .execute(conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(customer_id)
}

/// Start a subscription checkout for a tier
///
/// Returns a Stripe-hosted checkout URL to redirect the fan to.
///
/// # Errors
/// Returns error if the tier is unknown/inactive, Stripe is unconfigured, or calls fail
#[utoipa::path(
    post,
    path = "/api/billing/subscribe",
    tag = "Billing",
    request_body(content = SubscribeRequest, description = "Tier to subscribe to"),
    responses(
        (status = 200, description = "Checkout session created", body = CheckoutUrlResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 404, description = "Tier not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn subscribe(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    stripe: web::Data<Option<StripeService>>,
    body: web::Json<SubscribeRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::tiers::dsl as tiers_dsl;

    let stripe = require_stripe(&stripe)?;
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let tier: Tier = tiers_dsl::tiers
        .filter(tiers_dsl::id.eq(body.tier_id))
        .filter(tiers_dsl::is_active.eq(true))
        .first(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ServiceError::NotFound("Tier not found".to_owned()),
            _ => ServiceError::Database(e.to_string()),
        })?;

    let price_id = tier
        .stripe_price_id
        .ok_or_else(|| ServiceError::Config("Tier has no Stripe price".to_owned()))?;

    let customer_id = ensure_customer(&mut conn, stripe, &user).await?;

    let frontend = frontend_url();
    let url = stripe
        .create_checkout_session(
            &customer_id,
            &price_id,
            "subscription",
            &format!("{frontend}/billing/success"),
            &format!("{frontend}/billing/cancel"),
            &[
                ("user_id", user.id.to_string()),
                ("tier_id", tier.id.to_string()),
                ("tier_level", tier.level.to_string()),
            ],
        )
        .await?;

    Ok(HttpResponse::Ok().json(CheckoutUrlResponse { url }))
}

/// Start a one-off purchase checkout for a post or a series
///
/// # Errors
/// Returns error if the item is unknown or not purchasable, or Stripe calls fail
#[utoipa::path(
    post,
    path = "/api/billing/purchase",
    tag = "Billing",
    request_body(content = PurchaseRequest, description = "Post or series to buy"),
    responses(
        (status = 200, description = "Checkout session created", body = CheckoutUrlResponse),
        (status = 400, description = "Item is not purchasable", body = ErrorResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 404, description = "Item not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn purchase(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    stripe: web::Data<Option<StripeService>>,
    body: web::Json<PurchaseRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::posts::dsl as posts_dsl;
    use shared::schema::series::dsl as series_dsl;

    let stripe = require_stripe(&stripe)?;
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    // Resolve the sellable item: (name, amount, existing stripe price, metadata)
    let (name, amount_cents, stripe_price_id, meta_key, meta_value) =
        match (body.post_id, body.series_id) {
            (Some(post_id), None) => {
                let post: Post = posts_dsl::posts
                    .filter(posts_dsl::id.eq(post_id))
                    .filter(posts_dsl::deleted_at.is_null())
                    .first(&mut conn)
                    .await
                    .map_err(|e| match e {
                        diesel::result::Error::NotFound => {
                            ServiceError::NotFound("Post not found".to_owned())
                        }
                        _ => ServiceError::Database(e.to_string()),
                    })?;
                let amount = post.price_cents.ok_or_else(|| {
                    ServiceError::BadRequest("This post is not purchasable".to_owned())
                })?;
                (
                    post.title,
                    amount,
                    post.stripe_price_id,
                    "post_id",
                    post_id.to_string(),
                )
            }
            (None, Some(series_id)) => {
                let series: Series = series_dsl::series
                    .filter(series_dsl::id.eq(series_id))
                    .filter(series_dsl::deleted_at.is_null())
                    .first(&mut conn)
                    .await
                    .map_err(|e| match e {
                        diesel::result::Error::NotFound => {
                            ServiceError::NotFound("Series not found".to_owned())
                        }
                        _ => ServiceError::Database(e.to_string()),
                    })?;
                let amount = series.price_cents.ok_or_else(|| {
                    ServiceError::BadRequest("This series is not purchasable".to_owned())
                })?;
                (
                    series.title,
                    amount,
                    series.stripe_price_id,
                    "series_id",
                    series_id.to_string(),
                )
            }
            _ => {
                return Err(ServiceError::BadRequest(
                    "Provide exactly one of postId or seriesId".to_owned(),
                )
                .into())
            }
        };

    // Lazily mint (and persist) a Stripe price for the item on first purchase
    let price_id = match stripe_price_id {
        Some(price_id) => price_id,
        None => {
            let product_id = stripe.create_product(&name).await?;
            let price_id = stripe
                .create_price(&product_id, amount_cents, "usd", false)
                .await?;
            if meta_key == "post_id" {
                let _ = diesel::update(
                    posts_dsl::posts.filter(posts_dsl::id.eq(body.post_id.unwrap_or_default())),
                )
                .set(posts_dsl::stripe_price_id.eq(&price_id))
                .execute(&mut conn)
                .await
                .map_err(|e| ServiceError::Database(e.to_string()))?;
            } else {
                let _ = diesel::update(
                    series_dsl::series
                        .filter(series_dsl::id.eq(body.series_id.unwrap_or_default())),
                )
                .set(series_dsl::stripe_price_id.eq(&price_id))
                .execute(&mut conn)
                .await
                .map_err(|e| ServiceError::Database(e.to_string()))?;
            }
            price_id
        }
    };

    let customer_id = ensure_customer(&mut conn, stripe, &user).await?;

    let frontend = frontend_url();
    let url = stripe
        .create_checkout_session(
            &customer_id,
            &price_id,
            "payment",
            &format!("{frontend}/billing/success"),
            &format!("{frontend}/billing/cancel"),
            &[
                ("user_id", user.id.to_string()),
                (meta_key, meta_value),
                ("amount_cents", amount_cents.to_string()),
            ],
        )
        .await?;

    Ok(HttpResponse::Ok().json(CheckoutUrlResponse { url }))
}

/// Open a Stripe billing-portal session for subscription self-service
///
/// # Errors
/// Returns error if the user has no Stripe customer or Stripe calls fail
#[utoipa::path(
    post,
    path = "/api/billing/portal",
    tag = "Billing",
    responses(
        (status = 200, description = "Portal session created", body = CheckoutUrlResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 404, description = "No billing account for this user", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn portal(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    stripe: web::Data<Option<StripeService>>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::stripe_customers::dsl as customers_dsl;

    let stripe = require_stripe(&stripe)?;
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let customer: StripeCustomer = customers_dsl::stripe_customers
        .filter(customers_dsl::user_id.eq(user.id))
        .first(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ServiceError::NotFound("No billing account for this user".to_owned())
            }
            _ => ServiceError::Database(e.to_string()),
        })?;

    let frontend = frontend_url();
    let url = stripe
        .create_portal_session(&customer.stripe_customer_id, &format!("{frontend}/settings"))
        .await?;

    Ok(HttpResponse::Ok().json(CheckoutUrlResponse { url }))
}

/// Get the current fan's billing state: subscription + purchases
///
/// # Errors
/// Returns error if database queries fail
#[utoipa::path(
    get,
    path = "/api/billing/me",
    tag = "Billing",
    responses(
        (status = 200, description = "Billing state", body = BillingMeResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn billing_me(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::purchases::dsl as purchases_dsl;
    use shared::schema::subscriptions::dsl as subs_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let subscription: Option<Subscription> = subs_dsl::subscriptions
        .filter(subs_dsl::user_id.eq(user.id))
        .order(subs_dsl::updated_at.desc())
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let purchases: Vec<shared::models::billing::Purchase> = purchases_dsl::purchases
        .filter(purchases_dsl::user_id.eq(user.id))
        .filter(purchases_dsl::status.eq("paid"))
        .load(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(BillingMeResponse {
        subscription: subscription.map(|sub| SubscriptionInfo {
            tier_id: sub.tier_id,
            tier_level: sub.tier_level,
            status: sub.status,
            cancel_at_period_end: sub.cancel_at_period_end,
            current_period_end: sub.current_period_end.map(|dt| dt.and_utc()),
        }),
        purchases: purchases
            .into_iter()
            .map(|purchase| PurchaseInfo {
                post_id: purchase.post_id,
                series_id: purchase.series_id,
                amount_cents: purchase.amount_cents,
                created_at: Some(purchase.created_at.and_utc()),
            })
            .collect(),
    }))
}
