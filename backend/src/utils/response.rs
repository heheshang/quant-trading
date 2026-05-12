use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub data: T,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            data,
            message: "success".to_string(),
        }
    }

    pub fn with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: 0,
            data,
            message: message.into(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(code: i32, message: impl Into<String>, details: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let body = Json(self);
        (status_code, body).into_response()
    }
}

impl ApiError {
    fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self.code {
            // 4xx
            40001 | 40002 | 40003 => StatusCode::BAD_REQUEST,
            40101 | 40102 | 40103 => StatusCode::UNAUTHORIZED,
            40301 => StatusCode::FORBIDDEN,
            40401 => StatusCode::NOT_FOUND,
            40901 => StatusCode::CONFLICT,
            42901 => StatusCode::TOO_MANY_REQUESTS,
            // 5xx
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// Error codes
pub const SUCCESS: i32 = 0;
pub const ERR_BAD_REQUEST: i32 = 40001;
pub const ERR_VALIDATION: i32 = 40002;
pub const ERR_INVALID_CREDENTIALS: i32 = 40101;
pub const ERR_TOKEN_EXPIRED: i32 = 40102;
pub const ERR_TOKEN_INVALID: i32 = 40103;
pub const ERR_FORBIDDEN: i32 = 40301;
pub const ERR_NOT_FOUND: i32 = 40401;
pub const ERR_CONFLICT: i32 = 40901;
pub const ERR_RATE_LIMIT: i32 = 42901;
pub const ERR_INTERNAL: i32 = 50001;
pub const ERR_DATABASE: i32 = 50002;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_api_response_success() {
        let resp = ApiResponse::success("hello");
        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "success");
        assert_eq!(resp.data, "hello");
    }

    #[test]
    fn test_api_response_with_message() {
        let resp = ApiResponse::with_message(42, "user count");
        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "user count");
        assert_eq!(resp.data, 42);
    }

    #[test]
    fn test_api_response_serialization() {
        #[derive(Serialize)]
        struct User {
            id: i32,
            name: String,
        }
        let user = User { id: 1, name: "alice".into() };
        let resp = ApiResponse::success(user);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["id"], 1);
        assert_eq!(json["data"]["name"], "alice");
        assert_eq!(json["message"], "success");
    }

    #[test]
    fn test_api_response_into_response_status() {
        let resp = ApiResponse::success(true);
        let axum_resp = resp.into_response();
        assert_eq!(axum_resp.status(), StatusCode::OK);
    }

    #[test]
    fn test_api_error_new() {
        let err = ApiError::new(ERR_NOT_FOUND, "User not found");
        assert_eq!(err.code, ERR_NOT_FOUND);
        assert_eq!(err.message, "User not found");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_api_error_with_details() {
        let details = serde_json::json!({"field": "email"});
        let err = ApiError::with_details(ERR_VALIDATION, "Invalid email", details);
        assert_eq!(err.code, ERR_VALIDATION);
        assert!(err.details.is_some());
        assert_eq!(err.details.unwrap()["field"], "email");
    }

    #[test]
    fn test_api_error_serialization() {
        let err = ApiError::new(ERR_INVALID_CREDENTIALS, "Bad password");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], 40101);
        assert_eq!(json["message"], "Bad password");
        assert!(json.get("details").is_none());
    }

    #[test]
    fn test_api_error_status_codes() {
        let assert_status = |code: i32, expected: u16| {
            let err = ApiError::new(code, "test");
            let resp = err.into_response();
            assert_eq!(resp.status().as_u16(), expected, "code {} should map to status {}", code, expected);
        };

        assert_status(ERR_BAD_REQUEST, 400);
        assert_status(ERR_VALIDATION, 400);
        assert_status(ERR_INVALID_CREDENTIALS, 401);
        assert_status(ERR_TOKEN_EXPIRED, 401);
        assert_status(ERR_TOKEN_INVALID, 401);
        assert_status(ERR_FORBIDDEN, 403);
        assert_status(ERR_NOT_FOUND, 404);
        assert_status(ERR_CONFLICT, 409);
        assert_status(ERR_RATE_LIMIT, 429);
        assert_status(ERR_INTERNAL, 500);
        assert_status(ERR_DATABASE, 500);
    }
}
