use axum::{Router, routing::post};

use crate::{auth::handlers, config::AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
}
