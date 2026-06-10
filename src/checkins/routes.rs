use crate::{checkins::handlers, config::AppState};
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/check-in", post(handlers::check_in))
        .route("/status", get(handlers::status))
}
