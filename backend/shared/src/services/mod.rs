/// Authentication service for handling user login and registration
pub mod auth;
/// Configuration service for managing application settings
pub mod config;
/// Database service for interacting with the database
pub mod db;
/// Email service for sending verification and password reset emails
pub mod email;
/// Amazon S3 file storage service
pub mod s3;
/// Thin Stripe API client for billing
pub mod stripe;
/// Entitlement checks for gated content access
pub mod entitlements;
/// Web Push notification service
pub mod push;
