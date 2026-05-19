use axum::{Json, extract::State};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    config::AppState,
    errors::{AppError, AppResult},
};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub timezone: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
  get,
  path = "/me",
  responses(
    (status = 200, description = "User profile", body = UserProfile),
    (status = 401, description = "Unauthorized"),
  ),
  security(("bearer_auth" = []))
)]
pub async fn me(user: AuthUser, State(state): State<AppState>) -> AppResult<Json<UserProfile>> {
    let row = sqlx::query!(
        r#"
        SELECT id, email, display_name, timezone, created_at
        FROM users
        WHERE id = $1
        "#,
        user.user_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(UserProfile {
        id: row.id,
        email: row.email,
        display_name: row.display_name,
        timezone: row.timezone,
        created_at: row.created_at,
    }))
}
