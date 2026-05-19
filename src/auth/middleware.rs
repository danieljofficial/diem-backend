use axum::extract::FromRequestParts;
use uuid::Uuid;

use crate::{auth::jwt, config::AppState, errors::AppError};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = jwt::decode_token(token, &state.config.jwt_secret)?;

        Ok(AuthUser {
            user_id: claims.sub,
        })
    }
}
