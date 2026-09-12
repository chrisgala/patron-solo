//! Backend server for Patron, providing API endpoints and authentication services.

/// Handlers module containing API endpoint implementations.
pub mod handlers;
/// `OpenAPI` documentation module for API specification and documentation.
pub mod openapi;

use actix_cors::Cors;
use actix_session::{config::PersistentSession, storage::RedisSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    middleware::Logger,
    web::{self, PayloadConfig},
    App, HttpServer,
};
use openapi::ApiDoc;
use redis::aio::ConnectionManager;
use shared::services::{
    auth::GoogleOAuthService, config::ConfigService, db::DbService, email::EmailService,
    s3::S3Service,
};
use tracing::level_filters::LevelFilter;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};

/// Idempotently promote the `CREATOR_EMAIL` account to the creator role.
///
/// Runs at boot so an account that registered before `CREATOR_EMAIL` was set
/// (or a changed creator email) is reconciled without manual SQL. The partial
/// unique index `one_creator_only` guarantees at most one creator exists.
async fn promote_creator(db_service: &DbService) -> Result<(), Box<dyn std::error::Error>> {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use shared::schema::users::dsl as users_dsl;

    let Ok(creator_email) = std::env::var("CREATOR_EMAIL") else {
        return Ok(());
    };
    if creator_email.trim().is_empty() {
        return Ok(());
    }

    let pool = db_service.pool();
    let mut conn = pool.get().await?;

    let demoted = diesel::update(
        users_dsl::users
            .filter(users_dsl::role.eq("creator"))
            .filter(users_dsl::email.ne(&creator_email)),
    )
    .set(users_dsl::role.eq("fan"))
    .execute(&mut conn)
    .await?;
    if demoted > 0 {
        println!("Demoted {demoted} stale creator account(s).");
    }

    let promoted = diesel::update(
        users_dsl::users
            .filter(users_dsl::email.eq(&creator_email))
            .filter(users_dsl::role.ne("creator")),
    )
    .set(users_dsl::role.eq("creator"))
    .execute(&mut conn)
    .await?;
    if promoted > 0 {
        println!("Promoted {creator_email} to creator.");
    }

    ensure_feed_series(&mut conn).await?;

    Ok(())
}

/// Ensure the creator has the hidden default "Feed" series that holds
/// standalone update posts.
async fn ensure_feed_series(
    conn: &mut diesel_async::AsyncPgConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use shared::schema::series::dsl as series_dsl;
    use shared::schema::users::dsl as users_dsl;

    let Some(creator_id) = users_dsl::users
        .filter(users_dsl::role.eq("creator"))
        .select(users_dsl::id)
        .first::<uuid::Uuid>(conn)
        .await
        .optional()?
    else {
        return Ok(());
    };

    let feed_exists: i64 = series_dsl::series
        .filter(series_dsl::user_id.eq(creator_id))
        .filter(series_dsl::is_feed.eq(true))
        .count()
        .get_result(conn)
        .await?;
    if feed_exists > 0 {
        return Ok(());
    }

    let feed = shared::models::series::Series {
        id: uuid::Uuid::new_v4(),
        user_id: creator_id,
        title: "Feed".to_owned(),
        description: None,
        slug: "feed".to_owned(),
        category: None,
        cover_image_url: None,
        created_at: Some(chrono::Utc::now().naive_utc()),
        updated_at: Some(chrono::Utc::now().naive_utc()),
        deleted_at: None,
        price_cents: None,
        stripe_price_id: None,
        min_tier_level: None,
        is_feed: true,
    };
    let _ = diesel::insert_into(series_dsl::series)
        .values(&feed)
        .on_conflict_do_nothing()
        .execute(conn)
        .await?;
    println!("Created default feed series.");

    Ok(())
}

/// Entry point for the Patron backend server, sets up services and starts the HTTP server.
///
/// # Errors
///
/// Returns an error if:
/// - AWS configuration fails
/// - Database connection cannot be established
/// - Redis connection cannot be established
/// - HTTP server fails to bind or start
///
/// # Panics
///
/// Panics if the session TTL days cannot be converted to u32
#[allow(clippy::too_many_lines)]
pub async fn main() -> std::io::Result<()> {
    #[allow(unused_results)]
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let config = ConfigService::from_env();

    let aws_config = match config.aws_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to get AWS config: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get AWS config",
            ));
        }
    };
    let s3_service = match S3Service::new(aws_config).await {
        Ok(service) => service,
        Err(e) => {
            eprintln!("Failed to create S3 service: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create S3 service",
            ));
        }
    };

    let google_oauth_config = match config.google_oauth_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to get Google OAuth config: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get Google OAuth config",
            ));
        }
    };
    let google_oauth_service = GoogleOAuthService::new(google_oauth_config);

    let email_service = EmailService::from_env();

    let db_config = match config.database_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to get database config: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get database config",
            ));
        }
    };

    if let Err(e) = DbService::run_migrations(db_config) {
        eprintln!("Failed to run database migrations: {e}");
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to run database migrations",
        ));
    }
    println!("Database migrations ran successfully.");

    let db_service = match DbService::new(db_config).await {
        Ok(service) => service,
        Err(e) => {
            eprintln!("Failed to create DB service: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create DB service",
            ));
        }
    };

    if let Err(e) = promote_creator(&db_service).await {
        eprintln!("Failed to ensure creator role: {e}");
    }

    let stripe_service = shared::services::stripe::StripeService::from_env();
    if stripe_service.is_none() {
        println!("Stripe not configured; billing endpoints disabled.");
    }
    let push_service = shared::services::push::PushService::from_env();
    if push_service.is_none() {
        println!("VAPID keys not configured; web push disabled.");
    }

    let redis_config = match config.redis_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to get Redis config: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get Redis config",
            ));
        }
    };
    let redis_store = match RedisSessionStore::new(&redis_config.url).await {
        Ok(store) => store,
        Err(e) => {
            eprintln!("Failed to create Redis session store: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create Redis session store",
            ));
        }
    };

    let redis_client = match redis::Client::open(redis_config.url.as_str()) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to create Redis client: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create Redis client",
            ));
        }
    };
    let redis_manager = match ConnectionManager::new(redis_client).await {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to create Redis connection manager: {e}");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create Redis connection manager",
            ));
        }
    };

    let session_key = Key::from(google_oauth_service.auth_secret_key.as_bytes());

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T ms",
            ))
            .wrap(Cors::permissive())
            .app_data(PayloadConfig::new(0x4000_0000))
            .wrap(
                SessionMiddleware::builder(redis_store.clone(), session_key.clone())
                    .cookie_name("session_id".to_owned())
                    .cookie_secure(config.cookie_secure)
                    .cookie_http_only(true)
                    .cookie_same_site(if config.cookie_secure {
                        actix_web::cookie::SameSite::None
                    } else {
                        actix_web::cookie::SameSite::Lax
                    })
                    .session_lifecycle(PersistentSession::default().session_ttl(
                        actix_web::cookie::time::Duration::days(
                            #[allow(clippy::unwrap_used)]
                            (redis_config.session_ttl_days).try_into().unwrap(),
                        ),
                    ))
                    .build(),
            )
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(s3_service.clone()))
            .app_data(web::Data::new(db_service.clone()))
            .app_data(web::Data::new(google_oauth_service.clone()))
            .app_data(web::Data::new(email_service.clone()))
            .app_data(web::Data::new(redis_manager.clone()))
            .app_data(web::Data::new(stripe_service.clone()))
            .app_data(web::Data::new(push_service.clone()))
            .service(Redoc::with_url("/redoc", ApiDoc::openapi()))
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/auth")
                            .service(
                                web::resource("/google")
                                    .route(web::get().to(handlers::auth::google_auth_redirect)),
                            )
                            .service(
                                web::resource("/google/callback")
                                    .route(web::get().to(handlers::auth::google_auth_callback)),
                            )
                            .service(
                                web::resource("/register")
                                    .route(web::post().to(handlers::auth::register)),
                            )
                            .service(
                                web::resource("/login")
                                    .route(web::post().to(handlers::auth::login)),
                            )
                            .service(
                                web::resource("/verify-email")
                                    .route(web::get().to(handlers::auth::verify_email)),
                            )
                            .service(
                                web::resource("/logout")
                                    .route(web::get().to(handlers::auth::logout)),
                            )
                            .service(
                                web::resource("/me")
                                    .route(web::get().to(handlers::auth::get_me))
                                    .route(web::put().to(handlers::auth::update_user_info)),
                            )
                            .service(
                                web::resource("/forgot-password")
                                    .route(web::post().to(handlers::auth::forgot_password)),
                            )
                            .service(
                                web::resource("/reset-password")
                                    .route(web::post().to(handlers::auth::reset_password)),
                            )
                            .service(
                                web::resource("/check-email")
                                    .route(web::post().to(handlers::auth::check_email)),
                            )
                            .service(
                                web::resource("/resend-verification").route(
                                    web::post().to(handlers::auth::resend_verification_email),
                                ),
                            ),
                    )
                    .service(
                        web::scope("/files")
                            .service(
                                web::resource("/actions/upload")
                                    .route(web::post().to(handlers::user_files::upload_file)),
                            )
                            .service(
                                web::resource("")
                                    .route(web::get().to(handlers::user_files::list_files)),
                            )
                            .service(
                                web::resource("/{file_id}")
                                    .route(web::get().to(handlers::user_files::get_file))
                                    .route(web::put().to(handlers::user_files::update_file))
                                    .route(web::delete().to(handlers::user_files::delete_file)),
                            ),
                    )
                    .service(
                        web::scope("/cdn").service(
                            web::resource("/files/{file_id}")
                                .route(web::get().to(handlers::user_files::serve_file_cdn)),
                        ),
                    )
                    .service(
                        web::scope("/series")
                            .service(
                                web::resource("")
                                    .route(web::post().to(handlers::series::create_series))
                                    .route(web::get().to(handlers::series::list_series)),
                            )
                            .service(
                                web::resource("/{series_id}")
                                    .route(web::get().to(handlers::series::get_series))
                                    .route(web::put().to(handlers::series::update_series))
                                    .route(web::delete().to(handlers::series::delete_series)),
                            ),
                    )
                    .service(
                        web::scope("/posts")
                            .service(
                                web::resource("")
                                    .route(web::post().to(handlers::posts::create_post))
                                    .route(web::get().to(handlers::posts::list_posts)),
                            )
                            .service(
                                web::resource("/{post_id}")
                                    .route(web::get().to(handlers::posts::get_post))
                                    .route(web::put().to(handlers::posts::update_post))
                                    .route(web::delete().to(handlers::posts::delete_post)),
                            ),
                    )
                    .service(
                        web::scope("/api-keys")
                            .service(
                                web::resource("")
                                    .route(web::post().to(handlers::api_keys::create_api_key))
                                    .route(web::get().to(handlers::api_keys::list_api_keys)),
                            )
                            .service(
                                web::resource("/{api_key_id}")
                                    .route(web::get().to(handlers::api_keys::get_api_key))
                                    .route(web::put().to(handlers::api_keys::update_api_key))
                                    .route(web::delete().to(handlers::api_keys::delete_api_key)),
                            ),
                    )
                    .service(
                        web::scope("/public")
                            .service(
                                web::resource("/site")
                                    .route(web::get().to(handlers::site::get_site)),
                            )
                            .service(
                                web::resource("/tiers")
                                    .route(web::get().to(handlers::tiers::list_public_tiers)),
                            )
                            .service(
                                web::resource("/posts")
                                    .route(web::get().to(handlers::public::list_public_posts)),
                            )
                            .service(
                                web::resource("/posts/{id_or_slug}")
                                    .route(web::get().to(handlers::public::get_public_post)),
                            )
                            .service(
                                web::resource("/series")
                                    .route(web::get().to(handlers::public::list_public_series)),
                            )
                            .service(
                                web::resource("/series/{id_or_slug}")
                                    .route(web::get().to(handlers::public::get_public_series)),
                            )
                            .service(
                                web::resource("/push/key")
                                    .route(web::get().to(handlers::push::push_key)),
                            ),
                    )
                    .service(
                        web::scope("/tiers")
                            .service(
                                web::resource("")
                                    .route(web::post().to(handlers::tiers::create_tier))
                                    .route(web::get().to(handlers::tiers::list_tiers)),
                            )
                            .service(
                                web::resource("/{tier_id}")
                                    .route(web::put().to(handlers::tiers::update_tier))
                                    .route(web::delete().to(handlers::tiers::delete_tier)),
                            ),
                    )
                    .service(
                        web::scope("/billing")
                            .service(
                                web::resource("/subscribe")
                                    .route(web::post().to(handlers::billing::subscribe)),
                            )
                            .service(
                                web::resource("/purchase")
                                    .route(web::post().to(handlers::billing::purchase)),
                            )
                            .service(
                                web::resource("/portal")
                                    .route(web::post().to(handlers::billing::portal)),
                            )
                            .service(
                                web::resource("/me")
                                    .route(web::get().to(handlers::billing::billing_me)),
                            ),
                    )
                    .service(
                        web::scope("/webhooks").service(
                            web::resource("/stripe")
                                .route(web::post().to(handlers::stripe_webhook::stripe_webhook)),
                        ),
                    )
                    .service(
                        web::scope("/push").service(
                            web::resource("/subscribe")
                                .route(web::post().to(handlers::push::push_subscribe))
                                .route(web::delete().to(handlers::push::push_unsubscribe)),
                        ),
                    )
                    .service(
                        web::scope("/outrank").service(
                            web::resource("/webhook")
                                .route(web::post().to(handlers::outrank::process_webhook)),
                        ),
                    ),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
