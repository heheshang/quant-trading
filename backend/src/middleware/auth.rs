use crate::services::auth;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Extract authenticated user from request
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub role: String,
    pub jti: String,
}

/// JWT authentication middleware - validates access token from Authorization header
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    match auth_header {
        Some(token) => {
            match auth::validate_token(&token, &crate::CONFIG.jwt_secret) {
                Ok(claims) => {
                    if claims.token_type != "access" {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            axum::Json(serde_json::json!({
                                "code": 40101,
                                "message": "Invalid token type"
                            })),
                        ));
                    }
                    let user = AuthenticatedUser {
                        user_id: claims.sub.parse().unwrap_or_default(),
                        username: claims.username,
                        role: claims.role,
                        jti: claims.jti,
                    };
                    req.extensions_mut().insert(user);
                    Ok(next.run(req).await)
                }
                Err(e) => {
                    let (code, msg) = match e {
                        crate::utils::error::AppError::TokenExpired => (40102, "Token expired"),
                        _ => (40103, "Invalid token"),
                    };
                    Err((
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({
                            "code": code,
                            "message": msg
                        })),
                    ))
                }
            }
        }
        None => Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "code": 40101,
                "message": "Missing authorization header"
            })),
        )),
    }
}

impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut axum::http::request::Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({
                        "code": 40101,
                        "message": "Not authenticated"
                    })),
                )
            })
    }
}

/// Require a specific role for access
pub async fn require_role(
    req: Request,
    next: Next,
    required_role: &'static str,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    let user = req.extensions().get::<AuthenticatedUser>().cloned();
    match user {
        Some(u) if u.role == required_role || u.role == "admin" => Ok(next.run(req).await),
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "code": 40301,
                "message": "Insufficient permissions"
            })),
        )),
        None => Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "code": 40101,
                "message": "Not authenticated"
            })),
        )),
    }
}
