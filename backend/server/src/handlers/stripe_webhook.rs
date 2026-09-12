#![allow(clippy::unused_async)]

use actix_web::{web, HttpRequest, HttpResponse, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::billing::{Purchase, StripeEvent, Subscription},
    services::stripe::StripeService,
};
use uuid::Uuid;

/// Pull a string field out of a JSON object
fn json_str<'a>(value: &'a serde_json::Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(serde_json::Value::as_str)
}

/// Parse a uuid out of a JSON object field
fn json_uuid(value: &serde_json::Value, pointer: &str) -> Option<Uuid> {
    json_str(value, pointer).and_then(|s| Uuid::parse_str(s).ok())
}

/// Record a subscription state from a Stripe subscription object
async fn upsert_subscription(
    conn: &mut AsyncPgConnection,
    subscription: &serde_json::Value,
) -> Result<(), ServiceError> {
    use shared::schema::subscriptions::dsl as subs_dsl;

    let Some(stripe_subscription_id) = json_str(subscription, "/id") else {
        return Ok(());
    };
    let Some(user_id) = json_uuid(subscription, "/metadata/user_id") else {
        tracing::warn!("subscription event without user_id metadata; skipping");
        return Ok(());
    };
    let tier_id = json_uuid(subscription, "/metadata/tier_id");
    let tier_level = json_str(subscription, "/metadata/tier_level")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let status = json_str(subscription, "/status").unwrap_or("unknown").to_owned();
    let cancel_at_period_end = subscription
        .pointer("/cancel_at_period_end")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let current_period_end = subscription
        .pointer("/current_period_end")
        .and_then(serde_json::Value::as_i64)
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.naive_utc());

    let record = Subscription {
        id: Uuid::new_v4(),
        user_id,
        tier_id,
        tier_level,
        stripe_subscription_id: stripe_subscription_id.to_owned(),
        status: status.clone(),
        current_period_end,
        cancel_at_period_end,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    let _ = diesel::insert_into(subs_dsl::subscriptions)
        .values(&record)
        .on_conflict(subs_dsl::stripe_subscription_id)
        .do_update()
        .set((
            subs_dsl::status.eq(&status),
            subs_dsl::tier_id.eq(tier_id),
            subs_dsl::tier_level.eq(tier_level),
            subs_dsl::current_period_end.eq(current_period_end),
            subs_dsl::cancel_at_period_end.eq(cancel_at_period_end),
            subs_dsl::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(())
}

/// Record a one-off purchase from a completed payment checkout session
async fn record_purchase(
    conn: &mut AsyncPgConnection,
    session: &serde_json::Value,
) -> Result<(), ServiceError> {
    use shared::schema::purchases::dsl as purchases_dsl;

    let Some(user_id) = json_uuid(session, "/metadata/user_id") else {
        tracing::warn!("checkout session without user_id metadata; skipping");
        return Ok(());
    };
    let post_id = json_uuid(session, "/metadata/post_id");
    let series_id = json_uuid(session, "/metadata/series_id");
    if post_id.is_none() && series_id.is_none() {
        return Ok(());
    }

    let amount_cents = session
        .pointer("/amount_total")
        .and_then(serde_json::Value::as_i64)
        .and_then(|amount| i32::try_from(amount).ok());

    let record = Purchase {
        id: Uuid::new_v4(),
        user_id,
        post_id,
        series_id,
        stripe_checkout_session_id: json_str(session, "/id").map(str::to_owned),
        stripe_payment_intent_id: json_str(session, "/payment_intent").map(str::to_owned),
        amount_cents,
        status: "paid".to_owned(),
        created_at: Utc::now().naive_utc(),
    };

    let _ = diesel::insert_into(purchases_dsl::purchases)
        .values(&record)
        .on_conflict_do_nothing()
        .execute(conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(())
}

/// Stripe webhook receiver
///
/// Verifies the `Stripe-Signature` header, records the event id for
/// idempotency, and mirrors subscription/purchase state into the database.
///
/// # Errors
/// Returns 403 for invalid signatures and 500 for database failures
#[utoipa::path(
    post,
    path = "/api/webhooks/stripe",
    tag = "Billing",
    request_body(content = String, description = "Raw Stripe event JSON, signature-verified"),
    responses(
        (status = 200, description = "Event processed or ignored"),
        (status = 403, description = "Invalid signature", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn stripe_webhook(
    request: HttpRequest,
    payload: web::Bytes,
    db_service: web::Data<shared::services::db::DbService>,
    stripe: web::Data<Option<StripeService>>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::stripe_events::dsl as events_dsl;

    let Some(stripe) = stripe.as_ref().as_ref() else {
        return Err(ServiceError::Config("Stripe is not configured".to_owned()).into());
    };

    let signature = request
        .headers()
        .get("Stripe-Signature")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    let event = stripe.verify_webhook(&payload, signature)?;

    let event_id = json_str(&event, "/id").unwrap_or_default().to_owned();
    let event_type = json_str(&event, "/type").unwrap_or_default().to_owned();
    let object = event
        .pointer("/data/object")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    // Idempotency: first insert wins; replays are acknowledged and skipped
    let inserted = diesel::insert_into(events_dsl::stripe_events)
        .values(StripeEvent {
            id: event_id,
            event_type: event_type.clone(),
            processed_at: Utc::now().naive_utc(),
        })
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;
    if inserted == 0 {
        return Ok(HttpResponse::Ok().finish());
    }

    match event_type.as_str() {
        "checkout.session.completed" => {
            let mode = json_str(&object, "/mode").unwrap_or_default();
            if mode == "payment" {
                record_purchase(&mut conn, &object).await?;
            }
            // subscription checkouts are mirrored via customer.subscription.*
        }
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => {
            upsert_subscription(&mut conn, &object).await?;
        }
        _ => {}
    }

    Ok(HttpResponse::Ok().finish())
}
