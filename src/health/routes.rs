use axum::{Router, routing::get};

use crate::{config::AppState, health::handlers};

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(handlers::health))
}
