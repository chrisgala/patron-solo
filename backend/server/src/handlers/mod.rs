/// Authentication-related handlers
pub mod auth;

/// User file management handlers
pub mod user_files;

/// Series management handlers
pub mod series;

/// Posts management handlers
pub mod posts;

/// API keys management handlers
pub mod api_keys;

/// Outrank SEO integration handlers
pub mod outrank;

/// Public site information handlers
pub mod site;

/// Membership tier handlers
pub mod tiers;

/// Billing handlers: subscriptions, purchases, portal
pub mod billing;

/// Stripe webhook receiver
pub mod stripe_webhook;

/// Public content handlers with entitlement gating
pub mod public;

/// Web Push subscription handlers
pub mod push;
