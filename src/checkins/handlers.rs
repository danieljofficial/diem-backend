use axum::{Json, extract::State, http::StatusCode};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    config::AppState,
    errors::{AppError, AppResult},
};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CheckInRequest {
    pub checked_in_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CheckInResponse {
    pub check_in_date: NaiveDate,
    pub checked_in_at: chrono::DateTime<chrono::Utc>,
    pub already_checked_in: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StatusResponse {
    pub today: NaiveDate,
    pub checked_in_today: bool,
    pub current_streak: i64,
}

#[utoipa::path(
    post,
    path = "/check-in",
    request_body = CheckInRequest,
    responses(
        (status = 201, description = "Check-in recorded", body = CheckInResponse),
        (status = 200, description = "Already checked in", body = CheckInResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn check_in(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CheckInRequest>,
) -> AppResult<(StatusCode, Json<CheckInResponse>)> {
    let now = Utc::now();
    let client_timestamp = body.checked_in_at.unwrap_or(now);

    // Reject timestamps from the future or more than a day in the past.
    let drift = now - client_timestamp;
    if drift.num_hours() < -1 {
        return Err(AppError::Validation(
            "Check in timestamp is in the future".into(),
        ));
    }
    if drift.num_hours() > 24 {
        return Err(AppError::Validation(
            "Check in timestamp is more than 24 hours old".into(),
        ));
    }

    // Look up the user's timezone to determine what calendar day it is for them.
    let user_tz = sqlx::query_scalar!("SELECT timezone FROM users WHERE id = $1", user.user_id)
        .fetch_one(&state.db)
        .await?;

    let tz: chrono_tz::Tz = user_tz
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid stored timezone: {}", user_tz)))?;
    let today_for_user = now.with_timezone(&tz).date_naive();

    let row = sqlx::query!(
        r#"
        INSERT INTO check_ins (id, user_id, check_in_date, checked_in_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, check_in_date)
        DO UPDATE SET checked_in_at = LEAST(check_ins.checked_in_at, EXCLUDED.checked_in_at)
        RETURNING checked_in_at, (xmax = 0) AS "is_new_insert!"
        "#,
        Uuid::new_v4(),
        user.user_id,
        today_for_user,
        client_timestamp,
    )
    .fetch_one(&state.db)
    .await?;

    let status = if row.is_new_insert {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((
        status,
        Json(CheckInResponse {
            check_in_date: today_for_user,
            checked_in_at: row.checked_in_at,
            already_checked_in: !row.is_new_insert,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Current status", body = StatusResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn status(
    user: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<StatusResponse>> {
    let user_tz = sqlx::query_scalar!("SELECT timezone FROM users WHERE id = $1", user.user_id)
        .fetch_one(&state.db)
        .await?;

    let tz: chrono_tz::Tz = user_tz
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid stored timezone: {}", user_tz)))?;
    let today = Utc::now().with_timezone(&tz).date_naive();

    let checked_in_today = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM check_ins
            WHERE user_id = $1 AND check_in_date = $2
        ) AS "exists!"
        "#,
        user.user_id,
        today,
    )
    .fetch_one(&state.db)
    .await?;

    // CalculThis calculates the the current streak.
    let streak = sqlx::query_scalar!(
        r#"
        WITH grouped AS (
            SELECT
                check_in_date,
                check_in_date + CAST(ROW_NUMBER() OVER (ORDER BY check_in_date DESC) AS int) AS grp
            FROM check_ins
            WHERE user_id = $1
        )
        SELECT COUNT(*) AS "count!"
        FROM grouped
        WHERE grp = (
            SELECT grp FROM grouped
            WHERE check_in_date = $2
            LIMIT 1
        )
        "#,
        user.user_id,
        today,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(StatusResponse {
        today,
        checked_in_today,
        current_streak: streak,
    }))
}
