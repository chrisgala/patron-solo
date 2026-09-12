#![allow(clippy::unused_async)]

use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use shared::{
    errors::{ErrorResponse, ServiceError},
    models::{
        auth::{MaybeUser, User},
        engagement::{
            Comment, CommentResponse, CommentsListResponse, CreateCommentRequest,
            LikeStateResponse, PostLike,
        },
        posts::Post,
    },
    services::entitlements::{can_access_post, load_user_entitlements},
};
use utoipa::ToSchema;
use uuid::Uuid;

/// Maximum accepted comment length in characters
const MAX_COMMENT_CHARS: usize = 5000;

/// Load a published post and check the requester's entitlement to it.
/// Comments live behind the same gate as the post content.
async fn load_accessible_post(
    conn: &mut AsyncPgConnection,
    user: Option<&User>,
    post_id: Uuid,
) -> Result<Post, ServiceError> {
    use shared::schema::posts::dsl as posts_dsl;

    let post: Post = posts_dsl::posts
        .filter(posts_dsl::id.eq(post_id))
        .filter(posts_dsl::is_published.eq(true))
        .filter(posts_dsl::deleted_at.is_null())
        .first(conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ServiceError::NotFound("Post not found".to_owned()),
            _ => ServiceError::Database(e.to_string()),
        })?;

    let (user_id, is_creator) = match user {
        Some(user) => (Some(user.id), user.is_creator()),
        None => (None, false),
    };
    let entitlements = load_user_entitlements(conn, user_id, is_creator).await?;
    if !can_access_post(&entitlements, &post).granted {
        return Err(ServiceError::Forbidden(
            "This post requires a subscription or purchase".to_owned(),
        ));
    }

    Ok(post)
}

/// List comments on a post (requires access to the post)
///
/// # Errors
/// Returns 404 for unknown posts, 403 when the requester lacks access
#[utoipa::path(
    get,
    path = "/api/public/posts/{post_id}/comments",
    tag = "Engagement",
    params(("post_id" = Uuid, Path, description = "UUID of the post")),
    responses(
        (status = 200, description = "Comments, oldest first", body = CommentsListResponse),
        (status = 403, description = "Post requires a subscription or purchase", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn list_comments(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::comments::dsl as comments_dsl;
    use shared::schema::users::dsl as users_dsl;

    let post_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let _post = load_accessible_post(&mut conn, user.0.as_ref(), post_id).await?;

    let rows: Vec<(Comment, Option<String>, Option<String>, String)> = comments_dsl::comments
        .inner_join(users_dsl::users.on(comments_dsl::user_id.eq(users_dsl::id)))
        .filter(comments_dsl::post_id.eq(post_id))
        .filter(comments_dsl::deleted_at.is_null())
        .order(comments_dsl::created_at.asc())
        .select((
            Comment::as_select(),
            users_dsl::display_name,
            users_dsl::avatar_url,
            users_dsl::role,
        ))
        .load(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let (requester_id, requester_is_creator) = match &user.0 {
        Some(user) => (Some(user.id), user.is_creator()),
        None => (None, false),
    };

    Ok(HttpResponse::Ok().json(CommentsListResponse(
        rows.into_iter()
            .map(|(comment, display_name, avatar_url, role)| CommentResponse {
                id: comment.id,
                post_id: comment.post_id,
                parent_id: comment.parent_id,
                content: comment.content,
                author_name: display_name,
                author_avatar_url: avatar_url,
                is_creator: role == "creator",
                can_delete: requester_is_creator || requester_id == Some(comment.user_id),
                created_at: Some(comment.created_at.and_utc()),
            })
            .collect(),
    )))
}

/// Create a comment on a post (requires access to the post)
///
/// # Errors
/// Returns 401 unauthenticated, 403 without access, 404 for unknown posts
#[utoipa::path(
    post,
    path = "/api/posts/{post_id}/comments",
    tag = "Engagement",
    params(("post_id" = Uuid, Path, description = "UUID of the post")),
    request_body(content = CreateCommentRequest, description = "Comment text and optional parent"),
    responses(
        (status = 201, description = "Comment created", body = CommentResponse),
        (status = 400, description = "Empty or too-long comment", body = ErrorResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Post requires a subscription or purchase", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn create_comment(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
    body: web::Json<CreateCommentRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::comments::dsl as comments_dsl;

    let post_id = path.into_inner();
    let content = body.content.trim();
    if content.is_empty() || content.chars().count() > MAX_COMMENT_CHARS {
        return Err(ServiceError::BadRequest(
            "Comment must be between 1 and 5000 characters".to_owned(),
        )
        .into());
    }

    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let _post = load_accessible_post(&mut conn, Some(&user), post_id).await?;

    if let Some(parent_id) = body.parent_id {
        let parent_ok: i64 = comments_dsl::comments
            .filter(comments_dsl::id.eq(parent_id))
            .filter(comments_dsl::post_id.eq(post_id))
            .filter(comments_dsl::deleted_at.is_null())
            .count()
            .get_result(&mut conn)
            .await
            .map_err(|e| ServiceError::Database(e.to_string()))?;
        if parent_ok == 0 {
            return Err(ServiceError::NotFound("Parent comment not found".to_owned()).into());
        }
    }

    let record = Comment {
        id: Uuid::new_v4(),
        post_id,
        user_id: user.id,
        parent_id: body.parent_id,
        content: content.to_owned(),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        deleted_at: None,
    };
    let inserted: Comment = diesel::insert_into(comments_dsl::comments)
        .values(&record)
        .get_result(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::Created().json(CommentResponse {
        id: inserted.id,
        post_id: inserted.post_id,
        parent_id: inserted.parent_id,
        content: inserted.content,
        author_name: user.display_name.clone(),
        author_avatar_url: user.avatar_url.clone(),
        is_creator: user.is_creator(),
        can_delete: true,
        created_at: Some(inserted.created_at.and_utc()),
    }))
}

/// Delete a comment (author or the creator)
///
/// # Errors
/// Returns 401 unauthenticated, 403 when not the author or creator, 404 unknown
#[utoipa::path(
    delete,
    path = "/api/comments/{comment_id}",
    tag = "Engagement",
    params(("comment_id" = Uuid, Path, description = "UUID of the comment")),
    responses(
        (status = 204, description = "Comment deleted"),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Only the author or the creator may delete", body = ErrorResponse),
        (status = 404, description = "Comment not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn delete_comment(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::comments::dsl as comments_dsl;

    let comment_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let comment: Comment = comments_dsl::comments
        .filter(comments_dsl::id.eq(comment_id))
        .filter(comments_dsl::deleted_at.is_null())
        .first(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ServiceError::NotFound("Comment not found".to_owned())
            }
            _ => ServiceError::Database(e.to_string()),
        })?;

    if comment.user_id != user.id && !user.is_creator() {
        return Err(ServiceError::Forbidden(
            "Only the author or the creator may delete a comment".to_owned(),
        )
        .into());
    }

    let _ = diesel::update(comments_dsl::comments.filter(comments_dsl::id.eq(comment_id)))
        .set(comments_dsl::deleted_at.eq(Utc::now().naive_utc()))
        .execute(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}

/// Response wrapper naming for like toggling
#[derive(Debug, Clone, Copy, serde::Serialize, ToSchema)]
pub struct LikeToggleResponse {
    /// New like state after the operation
    #[serde(flatten)]
    pub state: LikeStateResponse,
}

/// Get the like state of a post for the requester
async fn like_state(
    conn: &mut AsyncPgConnection,
    post_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<LikeStateResponse, ServiceError> {
    use shared::schema::post_likes::dsl as likes_dsl;

    let like_count: i64 = likes_dsl::post_likes
        .filter(likes_dsl::post_id.eq(post_id))
        .count()
        .get_result(conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let liked_by_me = match user_id {
        Some(user_id) => {
            let count: i64 = likes_dsl::post_likes
                .filter(likes_dsl::post_id.eq(post_id))
                .filter(likes_dsl::user_id.eq(user_id))
                .count()
                .get_result(conn)
                .await
                .map_err(|e| ServiceError::Database(e.to_string()))?;
            count > 0
        }
        None => false,
    };

    Ok(LikeStateResponse {
        like_count,
        liked_by_me,
    })
}

/// Get like count and whether the requester liked the post
///
/// # Errors
/// Returns 404 for unknown posts
#[utoipa::path(
    get,
    path = "/api/public/posts/{post_id}/likes",
    tag = "Engagement",
    params(("post_id" = Uuid, Path, description = "UUID of the post")),
    responses(
        (status = 200, description = "Like state", body = LikeStateResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn get_likes(
    user: MaybeUser,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    let post_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let state = like_state(&mut conn, post_id, user.0.map(|user| user.id)).await?;
    Ok(HttpResponse::Ok().json(state))
}

/// Like a post (idempotent; requires access to the post)
///
/// # Errors
/// Returns 401 unauthenticated, 403 without access, 404 unknown post
#[utoipa::path(
    post,
    path = "/api/posts/{post_id}/like",
    tag = "Engagement",
    params(("post_id" = Uuid, Path, description = "UUID of the post")),
    responses(
        (status = 200, description = "New like state", body = LikeStateResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 403, description = "Post requires a subscription or purchase", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn like_post(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::post_likes::dsl as likes_dsl;

    let post_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let _post = load_accessible_post(&mut conn, Some(&user), post_id).await?;

    let record = PostLike {
        id: Uuid::new_v4(),
        post_id,
        user_id: user.id,
        created_at: Utc::now().naive_utc(),
    };
    let _ = diesel::insert_into(likes_dsl::post_likes)
        .values(&record)
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    let state = like_state(&mut conn, post_id, Some(user.id)).await?;
    Ok(HttpResponse::Ok().json(state))
}

/// Remove a like from a post (idempotent)
///
/// # Errors
/// Returns 401 unauthenticated
#[utoipa::path(
    delete,
    path = "/api/posts/{post_id}/like",
    tag = "Engagement",
    params(("post_id" = Uuid, Path, description = "UUID of the post")),
    responses(
        (status = 200, description = "New like state", body = LikeStateResponse),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    ),
    security(("cookieAuth" = [], "bearerAuth" = []))
)]
pub async fn unlike_post(
    user: User,
    db_service: web::Data<shared::services::db::DbService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    use shared::schema::post_likes::dsl as likes_dsl;

    let post_id = path.into_inner();
    let pool = db_service.pool();
    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let _ = diesel::delete(
        likes_dsl::post_likes
            .filter(likes_dsl::post_id.eq(post_id))
            .filter(likes_dsl::user_id.eq(user.id)),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| ServiceError::Database(e.to_string()))?;

    let state = like_state(&mut conn, post_id, Some(user.id)).await?;
    Ok(HttpResponse::Ok().json(state))
}
