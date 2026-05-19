use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
 use uuid::Uuid;

use crate::{config::AppState, errors::{AppError, AppResult}};

use super::jwt;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
  pub email: String,
  pub password: String,
  pub display_name: String,
  pub timezone: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
  pub email: String,
  pub password: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AuthResponse {
  pub token: String,
  pub user: UserDto,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserDto {
  pub id: Uuid,
  pub email: String,
  pub display_name: String,
  pub timezone: String
}

#[utoipa::path(
  post, 
  path = "/auth/register",
  request_body = RegisterRequest,
  responses(
    (status = 201, description = "Account Created", body = AuthResponse),
    (status = 409, description = "Email already taken"),
    (status = 400, description = "Validation error"),
  )
)]
pub async fn register(
  State(state): State<AppState>, 
  Json(body): Json<RegisterRequest>
) -> AppResult<(StatusCode, Json<AuthResponse>)> {

  if body.email.is_empty() || body.password.is_empty() || body.display_name.is_empty() {
    return Err(AppError::Validation("All fields are required".into()));
  };

  if body.password.len() < 8 {
    return Err(AppError::Validation("Password must be at least 8 characters".into()));
  }

  if body.timezone.parse::<chrono_tz::Tz>().is_err() {
    return Err(AppError::Validation(format!(
      "Invalid timezone: {}", 
      body.timezone
    )))


  }

  let exists = sqlx::query_scalar!(
    "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1) as \"exists!\"",
    body.email
  ).fetch_one(&state.db).await?;

  if exists {
    return Err(AppError::EmailTaken);
  }

  let salt  = SaltString::generate(&mut OsRng);
  let password_hash = Argon2::default()
  .hash_password(body.password.as_bytes(), &salt)
  .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed : {}", e)))?
  .to_string();

  let user_id = Uuid::new_v4();
  sqlx::query!(
    r#"
    INSERT INTO users (id, email, password_hash, display_name, timezone)
    VALUES ($1, $2, $3, $4, $5)
    "#,
    user_id,
    body.email,
    password_hash,
    body.display_name,
    body.timezone,
  )
  .execute(&state.db).await?;

  let token = jwt::encode_token(user_id, &state.config.jwt_secret)?;

 
  Ok((
    StatusCode::CREATED,
    Json(AuthResponse {
      token,
      user: UserDto { 
        id: user_id, 
        email: body.email, 
        display_name: body.display_name, 
        timezone: body.timezone }
    })
  ))
}

#[utoipa::path(
  post,
  path = "/auth/login",
  request_body = LoginRequest,
  responses(
    (status = 200, description = "Login successful", body = AuthResponse),
    (status = 401, description = "Invalid credentials"),
  )
)]
pub async fn login(
  State(state): State<AppState>,
  Json(body): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
  let user = sqlx::query!(
    r#"
    SELECT id, email, password_hash, display_name, timezone
    FROM users
    WHERE email = $1
    "#,
    body.email
  ).fetch_optional(&state.db).await?.ok_or(AppError::InvalidCredentials)?;

  let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|e| AppError::Internal(anyhow::anyhow!("Stored hash is corrupt: {}", e)))?;
  Argon2::default().verify_password(body.password.as_bytes(), &parsed_hash).map_err(|_| AppError::InvalidCredentials)?;

  let token = jwt::encode_token(user.id, &state.config.jwt_secret)?;

  Ok(Json(AuthResponse { 
    token, 
    user: UserDto { 
      id: user.id, 
      email: user.email, 
      display_name: user.display_name, 
      timezone: user.timezone 
    } 
  }))
}

