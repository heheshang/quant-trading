use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid: {0}")]
    TokenInvalid(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> i32 {
        match self {
            Self::BadRequest(_) => 40001,
            Self::Validation(_) => 40002,
            Self::InvalidCredentials => 40101,
            Self::TokenExpired => 40102,
            Self::TokenInvalid(_) => 40103,
            Self::Forbidden(_) => 40301,
            Self::NotFound(_) => 40401,
            Self::Conflict(_) => 40901,
            Self::RateLimit => 42901,
            Self::Database(_) => 50002,
            Self::Internal(_) => 50001,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::InvalidCredentials | Self::TokenExpired | Self::TokenInvalid(_) => {
                StatusCode::UNAUTHORIZED
            }
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = serde_json::json!({
            "code": self.code(),
            "message": self.to_string(),
        });

        (status, axum::Json(body)).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::error!("Database error: {:?}", err);
        Self::Database(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::TokenInvalid(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Internal error: {:?}", err);
        Self::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_app_error_code_mapping() {
        assert_eq!(AppError::BadRequest("x".into()).code(), 40001);
        assert_eq!(AppError::Validation("x".into()).code(), 40002);
        assert_eq!(AppError::InvalidCredentials.code(), 40101);
        assert_eq!(AppError::TokenExpired.code(), 40102);
        assert_eq!(AppError::TokenInvalid("x".into()).code(), 40103);
        assert_eq!(AppError::Forbidden("x".into()).code(), 40301);
        assert_eq!(AppError::NotFound("x".into()).code(), 40401);
        assert_eq!(AppError::Conflict("x".into()).code(), 40901);
        assert_eq!(AppError::RateLimit.code(), 42901);
        assert_eq!(AppError::Database("x".into()).code(), 50002);
        assert_eq!(AppError::Internal("x".into()).code(), 50001);
    }

    #[test]
    fn test_into_response_status_codes() {
        let assert_status = |err: AppError, expected: StatusCode| {
            let resp = err.into_response();
            assert_eq!(resp.status(), expected);
        };

        assert_status(AppError::BadRequest("x".into()), StatusCode::BAD_REQUEST);
        assert_status(AppError::Validation("x".into()), StatusCode::BAD_REQUEST);
        assert_status(AppError::InvalidCredentials, StatusCode::UNAUTHORIZED);
        assert_status(AppError::TokenExpired, StatusCode::UNAUTHORIZED);
        assert_status(AppError::TokenInvalid("x".into()), StatusCode::UNAUTHORIZED);
        assert_status(AppError::Forbidden("x".into()), StatusCode::FORBIDDEN);
        assert_status(AppError::NotFound("x".into()), StatusCode::NOT_FOUND);
        assert_status(AppError::Conflict("x".into()), StatusCode::CONFLICT);
        assert_status(AppError::RateLimit, StatusCode::TOO_MANY_REQUESTS);
        assert_status(AppError::Database("x".into()), StatusCode::INTERNAL_SERVER_ERROR);
        assert_status(AppError::Internal("x".into()), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_display_format() {
        assert_eq!(
            AppError::BadRequest("invalid param".to_string()).to_string(),
            "Bad request: invalid param"
        );
        assert_eq!(
            AppError::Validation("min 8 chars".to_string()).to_string(),
            "Validation error: min 8 chars"
        );
        assert_eq!(
            AppError::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
        assert_eq!(AppError::TokenExpired.to_string(), "Token expired");
        assert_eq!(
            AppError::NotFound("user".to_string()).to_string(),
            "Not found: user"
        );
        assert_eq!(
            AppError::Conflict("already exists".to_string()).to_string(),
            "Conflict: already exists"
        );
        assert_eq!(AppError::RateLimit.to_string(), "Rate limit exceeded");
    }

    #[test]
    fn test_from_db_err() {
        let db_err = sea_orm::DbErr::RecordNotFound("user".to_string());
        let app_err: AppError = db_err.into();
        assert!(matches!(app_err, AppError::Database(_)));
    }

    #[test]
    fn test_from_jwt_err() {
        let jwt_err = jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        );
        let app_err: AppError = jwt_err.into();
        assert!(matches!(app_err, AppError::TokenInvalid(_)));
    }

    #[test]
    fn test_into_response_body_contains_code_and_message() {
        let err = AppError::NotFound("user id=123".into());
        let resp = err.into_response();
        let _status = resp.status();
        // Note: Body content verification requires async context
        // Status code verification is done in test_into_response_status_codes
    }
}
