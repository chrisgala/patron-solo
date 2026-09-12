//! Entitlement checks: the single source of truth for who may access a post.
//!
//! Rule: a post is accessible iff it is free (no gate, or rolled free via
//! `free_at`), OR the user holds an active subscription at >= the post's
//! minimum tier level, OR the user purchased the post, OR the user purchased
//! the post's series. The creator always has access.

use crate::errors::ServiceError;
use crate::models::posts::Post;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Why access was granted, surfaced to the client for UI states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AccessReason {
    /// Post has no gate or has rolled free
    Free,
    /// Active subscription at a sufficient tier
    Subscription,
    /// One-off purchase of the post
    Purchase,
    /// One-off purchase of the post's series
    Series,
    /// The requester is the site's creator
    Creator,
}

/// The outcome of an entitlement check for one post
#[derive(Debug, Clone, Copy)]
pub struct AccessDecision {
    /// Whether access is granted
    pub granted: bool,
    /// Why access was granted (None when denied)
    pub reason: Option<AccessReason>,
}

/// A user's entitlement context, loaded once and reused across posts
#[derive(Debug, Clone, Default)]
pub struct UserEntitlements {
    /// Highest active subscription tier level, if any
    pub sub_tier_level: Option<i32>,
    /// Post ids owned via one-off purchase
    pub owned_post_ids: Vec<Uuid>,
    /// Series ids owned via one-off purchase
    pub owned_series_ids: Vec<Uuid>,
    /// Whether the user is the creator
    pub is_creator: bool,
}

/// Load a user's entitlement context (two queries), or an empty context for
/// anonymous visitors.
///
/// # Errors
/// Returns an error when a database query fails
pub async fn load_user_entitlements(
    conn: &mut AsyncPgConnection,
    user_id: Option<Uuid>,
    is_creator: bool,
) -> Result<UserEntitlements, ServiceError> {
    use crate::schema::purchases::dsl as purchases_dsl;
    use crate::schema::subscriptions::dsl as subs_dsl;

    let Some(user_id) = user_id else {
        return Ok(UserEntitlements::default());
    };

    let sub_tier_level: Option<i32> = subs_dsl::subscriptions
        .filter(subs_dsl::user_id.eq(user_id))
        .filter(subs_dsl::status.eq_any(["active", "trialing"]))
        .select(diesel::dsl::max(subs_dsl::tier_level))
        .first(conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let owned: Vec<(Option<Uuid>, Option<Uuid>)> = purchases_dsl::purchases
        .filter(purchases_dsl::user_id.eq(user_id))
        .filter(purchases_dsl::status.eq("paid"))
        .select((purchases_dsl::post_id, purchases_dsl::series_id))
        .load(conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let mut owned_post_ids = Vec::new();
    let mut owned_series_ids = Vec::new();
    for (post_id, series_id) in owned {
        if let Some(id) = post_id {
            owned_post_ids.push(id);
        }
        if let Some(id) = series_id {
            owned_series_ids.push(id);
        }
    }

    Ok(UserEntitlements {
        sub_tier_level,
        owned_post_ids,
        owned_series_ids,
        is_creator,
    })
}

/// Decide access to one post given a loaded entitlement context
#[must_use]
pub fn can_access_post(entitlements: &UserEntitlements, post: &Post) -> AccessDecision {
    if entitlements.is_creator {
        return AccessDecision {
            granted: true,
            reason: Some(AccessReason::Creator),
        };
    }

    let rolled_free = post
        .free_at
        .is_some_and(|t| t <= Utc::now().naive_utc());
    let has_gate = post.min_tier_level.is_some() || post.price_cents.is_some();
    if !has_gate || rolled_free {
        return AccessDecision {
            granted: true,
            reason: Some(AccessReason::Free),
        };
    }

    if let (Some(user_level), Some(required)) =
        (entitlements.sub_tier_level, post.min_tier_level)
    {
        if user_level >= required {
            return AccessDecision {
                granted: true,
                reason: Some(AccessReason::Subscription),
            };
        }
    }

    if entitlements.owned_post_ids.contains(&post.id) {
        return AccessDecision {
            granted: true,
            reason: Some(AccessReason::Purchase),
        };
    }

    if entitlements.owned_series_ids.contains(&post.series_id) {
        return AccessDecision {
            granted: true,
            reason: Some(AccessReason::Series),
        };
    }

    AccessDecision {
        granted: false,
        reason: None,
    }
}
