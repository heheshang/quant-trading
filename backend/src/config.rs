use std::env;
use std::time::Duration;

/// List of known weak/default JWT secrets that must never be used in production.
const WEAK_JWT_SECRETS: &[&str] = &[
    "super-secret-key-change-in-production",
    "change-this-to-a-long-random-string-in-production",
    "changeme",
    "secret",
    "jwt-secret",
    "your-secret-key",
];

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_access_exp: Duration,
    pub jwt_refresh_exp: Duration,
    pub server_host: String,
    pub server_port: u16,
    pub cors_allowed_origins: Vec<String>,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:***@localhost:5432/quant_trading".to_string());

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "super-secret-key-change-in-production".to_string());

        let config = Self {
            database_url,
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into()),
            jwt_secret,
            jwt_access_exp: Duration::from_secs(
                env::var("JWT_ACCESS_EXP_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(900),
            ),
            jwt_refresh_exp: Duration::from_secs(
                env::var("JWT_REFRESH_EXP_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(604800),
            ),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            cors_allowed_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
        };

        // Reject known weak/default JWT secrets at startup (non-test only).
        // This prevents accidentally deploying with placeholder secrets.
        // Unit tests construct Config directly or use their own JWT_SECRET.
        if !cfg!(test) {
            if WEAK_JWT_SECRETS.contains(&config.jwt_secret.as_str()) {
                eprintln!(
                    "FATAL: JWT_SECRET is set to a known weak/default value: {:?}. \
                     Refusing to start. Set a strong, unique secret in .env or environment.",
                    config.jwt_secret
                );
                std::process::exit(1);
            }

            // Minimum length check (32 chars = 256 bits for HMAC-SHA256)
            if config.jwt_secret.len() < 32 {
                eprintln!(
                    "FATAL: JWT_SECRET is too short ({} chars). Minimum 32 characters required for HMAC-SHA256. \
                     Refusing to start. Set a strong, unique secret in .env or environment.",
                    config.jwt_secret.len()
                );
                std::process::exit(1);
            }
        }

        config
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
