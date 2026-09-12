use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for a comment on a post
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::comments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Comment {
    /// Unique comment identifier
    pub id: Uuid,
    /// The post commented on
    pub post_id: Uuid,
    /// The commenting user
    pub user_id: Uuid,
    /// Parent comment when this is a reply
    pub parent_id: Option<Uuid>,
    /// Comment text
    pub content: String,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
    /// Last update timestamp
    pub updated_at: NaiveDateTime,
    /// Soft-delete timestamp
    pub deleted_at: Option<NaiveDateTime>,
}

/// Database model for a like on a post
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::post_likes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PostLike {
    /// Unique row identifier
    pub id: Uuid,
    /// The liked post
    pub post_id: Uuid,
    /// The liking user
    pub user_id: Uuid,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
}

/// A comment as returned by the API, with author display info
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "a1b2c3d4-5e6f-7890-abcd-ef1234567890",
    "postId": "d290f1ee-6c54-4b01-90e6-d701748f0851",
    "parentId": null,
    "content": "Loved this chapter!",
    "authorName": "A Fan",
    "authorAvatarUrl": "https://example.com/avatar.jpg",
    "isCreator": false,
    "createdAt": "2023-01-01T00:00:00Z"
}))]
pub struct CommentResponse {
    /// Comment id
    pub id: Uuid,
    /// The post commented on
    #[serde(rename = "postId")]
    pub post_id: Uuid,
    /// Parent comment when this is a reply
    #[serde(rename = "parentId")]
    pub parent_id: Option<Uuid>,
    /// Comment text
    pub content: String,
    /// Author's display name
    #[serde(rename = "authorName")]
    pub author_name: Option<String>,
    /// Author's avatar URL
    #[serde(rename = "authorAvatarUrl")]
    pub author_avatar_url: Option<String>,
    /// Whether the author is the site's creator
    #[serde(rename = "isCreator")]
    pub is_creator: bool,
    /// Whether the requester may delete this comment (own comment, or creator)
    #[serde(rename = "canDelete")]
    pub can_delete: bool,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
}

/// Response type for comment list endpoints
#[derive(Debug, Serialize, ToSchema)]
pub struct CommentsListResponse(
    /// Comments, oldest first, replies interleaved after their parents
    pub Vec<CommentResponse>,
);

/// Request to create a comment
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"content": "Loved this chapter!", "parentId": null}))]
pub struct CreateCommentRequest {
    /// Comment text (1..=5000 chars)
    pub content: String,
    /// Parent comment id when replying
    #[serde(rename = "parentId")]
    pub parent_id: Option<Uuid>,
}

/// Like state of a post for the requester
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"likeCount": 12, "likedByMe": true}))]
pub struct LikeStateResponse {
    /// Total number of likes on the post
    #[serde(rename = "likeCount")]
    pub like_count: i64,
    /// Whether the requesting user has liked the post
    #[serde(rename = "likedByMe")]
    pub liked_by_me: bool,
}
