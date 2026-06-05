use axum::{Json, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{errors::AppResult, users::handlers::UserProfile};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
    success: bool,
    timestamp: DateTime<Utc>,
}

#[utoipa::path(
  get,
  path = "/health",
  responses(
    (status = 200, description = "Health", body = UserProfile),
    (status = 400, description = "Bad request"),
  ),
)]
pub async fn health() -> AppResult<(StatusCode, Json<HealthResponse>)> {
    tracing::info!("All systems go!");
    Ok((
        StatusCode::OK,
        Json(HealthResponse {
            success: true,
            timestamp: Utc::now(),
        }),
    ))
}
