use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .expect("PORT must be valid"),
        }
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
}
