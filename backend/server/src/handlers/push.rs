#![allow(clippy::unused_async)]

use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::{
        auth::MaybeUser,
        push::{PushKeyResponse, PushSubscribeRequest, PushSubscription, PushUnsubscribeRequest},
    },
    services::push::{PushOutcome, PushService},
};
use uuid::Uuid;

/// Get the server's VAPID public key for `PushManager.subscribe`
///
/// # Errors
/// Returns 404 when push is not configured
#[utoipa::path(
    get,
    path = "/api/public/push/key",
    tag = "Push",
    responses(
        (status = 200, description = "VAPID public key", body = PushKeyResponse),
        (status = 404, description = "Push notifications not configured", body = ErrorResponse)
    )
)]
pub async fn push_key(
    push: web::Data<Option<PushService>>,
) -> Result<HttpResponse, actix_web::Error> {
    match push.as_ref() {
        Some(service) => Ok(HttpResponse::Ok().json(PushKeyResponse {
            public_key: service.public_key().to_owned(),
        })),
        None => Err(ServiceError::NotFound("Push notifications not configured".to_owned()).into()),
    }
}

/// Register a browser push subscription (anonymous allowed)
///
/// # Errors
/// Returns error if the database write fails
#[utoipa::path(
    post,
    path = "/api/push/subscribe",
    tag = "Push",
    request_body(content = PushSubscribeRequest, description = "Browser push subscription"),
    responses(
        (status = 204, description = "Subscription stored"),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn push_subscribe(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
    body: web::Json<PushSubscribeRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::push_subscriptions::dsl as push_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let record = PushSubscription {
        id: Uuid::new_v4(),
        user_id: user.0.map(|user| user.id),
        endpoint: body.endpoint.clone(),
        p256dh: body.p256dh.clone(),
        auth: body.auth.clone(),
        created_at: Utc::now().naive_utc(),
    };

    let _ = diesel::insert_into(push_dsl::push_subscriptions)
        .values(&record)
        .on_conflict(push_dsl::endpoint)
        .do_update()
        .set((
            push_dsl::p256dh.eq(&record.p256dh),
            push_dsl::auth.eq(&record.auth),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}

/// Remove a browser push subscription by endpoint
///
/// # Errors
/// Returns error if the database write fails
#[utoipa::path(
    delete,
    path = "/api/push/subscribe",
    tag = "Push",
    request_body(content = PushUnsubscribeRequest, description = "Endpoint to remove"),
    responses(
        (status = 204, description = "Subscription removed"),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn push_unsubscribe(
    db_service: web::Data<shared::services::db::DbService>,
    body: web::Json<PushUnsubscribeRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::push_subscriptions::dsl as push_dsl;

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let _ = diesel::delete(
        push_dsl::push_subscriptions.filter(push_dsl::endpoint.eq(&body.endpoint)),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}

/// Fire-and-forget: wake every subscribed browser (called on post publish).
/// Subscriptions rejected as gone are deleted.
pub fn notify_all_subscribers(
    db_service: web::Data<shared::services::db::DbService>,
    push: Option<PushService>,
) {
    let Some(push) = push else { return };

    drop(tokio::spawn(async move {
        use shared::schema::push_subscriptions::dsl as push_dsl;

        let pool = db_service.pool();
        let Ok(mut conn) = pool.get().await else {
            return;
        };

        let subscriptions: Vec<PushSubscription> =
            match push_dsl::push_subscriptions.load(&mut conn).await {
                Ok(subscriptions) => subscriptions,
                Err(error) => {
                    tracing::warn!(%error, "failed to load push subscriptions");
                    return;
                }
            };

        for subscription in subscriptions {
            if push.send_wakeup(&subscription.endpoint).await == PushOutcome::Gone {
                let _ = diesel::delete(
                    push_dsl::push_subscriptions.filter(push_dsl::id.eq(subscription.id)),
                )
                .execute(&mut conn)
                .await;
            }
        }
    }));
}
