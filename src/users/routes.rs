use axum::{Router, routing::get};

use crate::{config::AppState, users::handlers};

pub fn routes() -> Router<AppState> {
    Router::new().route("/me", get(handlers::me))
}
