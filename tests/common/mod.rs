use axum::Router;
use diem_backend::config::{AppConfig, AppState};
use sqlx::PgPool;

/// Builds the full app router backed by the given database pool.
/// This is the test equivalent of what main.rs does — same router,
/// same middleware, just pointed at a throwaway test database.
pub fn build_test_app(db: PgPool) -> Router {
    let config = AppConfig {
        database_url: String::new(), // unused — pool is already connected
        jwt_secret: "test-secret-do-not-use-in-prod".into(),
        host: "127.0.0.1".into(),
        port: 0,
    };

    let state = AppState {
        db,
        config: config.clone(),
    };

    Router::new()
        .merge(diem_backend::auth::routes::routes())
        .merge(diem_backend::users::routes::routes())
        .merge(diem_backend::checkins::routes::routes())
        .with_state(state)
}

/// Registers a test user and returns their JWT token.
/// Most tests need an authenticated user — this avoids repeating
/// the registration boilerplate in every test function.
pub async fn register_user(app: &Router, email: &str, timezone: &str) -> String {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    let body = serde_json::json!({
        "email": email,
        "password": "testpassword123",
        "display_name": "Test User",
        "timezone": timezone
    });

    let req = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["token"].as_str().unwrap().to_string()
}

/// Performs a check-in for the authenticated user.
/// Returns the response body as JSON.
pub async fn do_check_in(
    app: &Router,
    token: &str,
    checked_in_at: Option<&str>,
) -> (http::StatusCode, serde_json::Value) {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    let body = match checked_in_at {
        Some(ts) => serde_json::json!({ "checked_in_at": ts }),
        None => serde_json::json!({}),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/check-in")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

/// Fetches the status for the authenticated user.
pub async fn get_status(app: &Router, token: &str) -> serde_json::Value {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    let req = Request::builder()
        .method("GET")
        .uri("/status")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
