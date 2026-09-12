// @generated automatically by Diesel CLI.

#![allow(clippy::pub_use)]
#![allow(clippy::single_char_lifetime_names)]

diesel::table! {
    api_keys (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Text,
        key_hash -> Text,
        key_prefix -> Text,
        permissions -> Nullable<Array<Nullable<Text>>>,
        last_used_at -> Nullable<Timestamp>,
        expires_at -> Nullable<Timestamp>,
        is_active -> Bool,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    email_verification_tokens (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 255]
        token -> Varchar,
        expires_at -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    posts (id) {
        id -> Uuid,
        series_id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        content -> Text,
        #[max_length = 255]
        slug -> Varchar,
        number -> Int4,
        is_published -> Nullable<Bool>,
        thumbnail_url -> Nullable<Text>,
        audio_file_id -> Nullable<Uuid>,
        video_file_id -> Nullable<Uuid>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
        #[max_length = 20]
        kind -> Varchar,
        min_tier_level -> Nullable<Int4>,
        price_cents -> Nullable<Int4>,
        stripe_price_id -> Nullable<Text>,
        free_at -> Nullable<Timestamp>,
        image_file_ids -> Nullable<Array<Nullable<Uuid>>>,
    }
}

diesel::table! {
    series (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        description -> Nullable<Text>,
        #[max_length = 255]
        slug -> Varchar,
        #[max_length = 100]
        category -> Nullable<Varchar>,
        cover_image_url -> Nullable<Text>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
        price_cents -> Nullable<Int4>,
        stripe_price_id -> Nullable<Text>,
        min_tier_level -> Nullable<Int4>,
        is_feed -> Bool,
    }
}

diesel::table! {
    tiers (id) {
        id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        description -> Nullable<Text>,
        level -> Int4,
        price_cents -> Int4,
        #[max_length = 3]
        currency -> Varchar,
        stripe_product_id -> Nullable<Text>,
        stripe_price_id -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    stripe_customers (user_id) {
        user_id -> Uuid,
        stripe_customer_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    subscriptions (id) {
        id -> Uuid,
        user_id -> Uuid,
        tier_id -> Nullable<Uuid>,
        tier_level -> Int4,
        stripe_subscription_id -> Text,
        #[max_length = 30]
        status -> Varchar,
        current_period_end -> Nullable<Timestamp>,
        cancel_at_period_end -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    stripe_events (id) {
        id -> Text,
        event_type -> Text,
        processed_at -> Timestamp,
    }
}

diesel::table! {
    purchases (id) {
        id -> Uuid,
        user_id -> Uuid,
        post_id -> Nullable<Uuid>,
        series_id -> Nullable<Uuid>,
        stripe_checkout_session_id -> Nullable<Text>,
        stripe_payment_intent_id -> Nullable<Text>,
        amount_cents -> Nullable<Int4>,
        #[max_length = 20]
        status -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    push_subscriptions (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        endpoint -> Text,
        p256dh -> Text,
        auth -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    series_length (id) {
        id -> Uuid,
        series_id -> Uuid,
        length -> Int4,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    user_files (id) {
        id -> Uuid,
        user_id -> Uuid,
        filename -> Text,
        original_filename -> Text,
        file_path -> Text,
        file_size -> Int8,
        mime_type -> Text,
        file_hash -> Text,
        status -> Text,
        metadata -> Nullable<Jsonb>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Text,
        password_hash -> Nullable<Text>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 255]
        display_name -> Nullable<Varchar>,
        avatar_url -> Nullable<Text>,
        #[max_length = 50]
        auth_provider -> Varchar,
        email_verified -> Bool,
        last_login -> Nullable<Timestamp>,
        description -> Nullable<Text>,
        banner -> Nullable<Text>,
        #[max_length = 20]
        role -> Varchar,
    }
}

diesel::joinable!(api_keys -> users (user_id));
diesel::joinable!(email_verification_tokens -> users (user_id));
diesel::joinable!(posts -> series (series_id));
diesel::joinable!(series -> users (user_id));
diesel::joinable!(series_length -> series (series_id));
diesel::joinable!(user_files -> users (user_id));
diesel::joinable!(stripe_customers -> users (user_id));
diesel::joinable!(subscriptions -> users (user_id));
diesel::joinable!(subscriptions -> tiers (tier_id));
diesel::joinable!(purchases -> users (user_id));
diesel::joinable!(purchases -> posts (post_id));
diesel::joinable!(purchases -> series (series_id));
diesel::joinable!(push_subscriptions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_keys,
    email_verification_tokens,
    posts,
    series,
    series_length,
    user_files,
    users,
    tiers,
    stripe_customers,
    subscriptions,
    stripe_events,
    purchases,
    push_subscriptions,
);
