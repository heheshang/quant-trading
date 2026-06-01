use crate::db::{role, user, user_session};
use crate::models::schemas::{
    AuthResponse, JwtClaims, LoginRequest, RegisterRequest, TokenResponse, UserResponse,
};
use crate::utils::error::AppError;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::info;
use uuid::Uuid;

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, hash)
        .map_err(|e| AppError::Internal(format!("Failed to verify password: {}", e)))
}

/// Generate JWT token pair (access + refresh)
pub fn generate_tokens(
    user: &user::Model,
    role_name: &str,
    jwt_secret: &str,
    access_exp_secs: u64,
    refresh_exp_secs: u64,
) -> Result<(String, String, u64), AppError> {
    let now = chrono::Utc::now().timestamp() as usize;

    let access_claims = JwtClaims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: role_name.to_string(),
        exp: now + access_exp_secs as usize,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        token_type: "access".to_string(),
    };

    let refresh_claims = JwtClaims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: role_name.to_string(),
        exp: now + refresh_exp_secs as usize,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        token_type: "refresh".to_string(),
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to generate access token: {}", e)))?;

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to generate refresh token: {}", e)))?;

    Ok((access_token, refresh_token, access_exp_secs))
}

/// Validate a JWT token
pub fn validate_token(token: &str, jwt_secret: &str) -> Result<JwtClaims, AppError> {
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
        _ => AppError::TokenInvalid(e.to_string()),
    })?;

    Ok(token_data.claims)
}

/// Register a new user
pub async fn register(
    db: &DatabaseConnection,
    req: RegisterRequest,
) -> Result<AuthResponse, AppError> {
    // Validate inputs
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err(AppError::Validation(
            "Username must be between 3 and 32 characters".into(),
        ));
    }

    if req.password.len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters".into(),
        ));
    }

    if !req.email.contains('@') {
        return Err(AppError::Validation("Invalid email format".into()));
    }

    // Check username uniqueness
    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Username already exists".into()));
    }

    // Check email uniqueness
    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Email already registered".into()));
    }

    // Find default "user" role
    let role = role::Entity::find()
        .filter(role::Column::Name.eq("user"))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Internal("Default role not found".into()))?;

    // Hash password
    let password_hash = hash_password(&req.password)?;

    // Create user
    let now = chrono::Utc::now();
    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(req.username.clone()),
        email: Set(req.email.clone()),
        password_hash: Set(password_hash),
        display_name: Set(req.display_name.clone().or(Some(req.username.clone()))),
        avatar_url: Set(None),
        is_active: Set(true),
        role_id: Set(role.id),
        last_login_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved_user = new_user.insert(db).await?;

    // Generate tokens
    let (access_token, refresh_token, expires_in) = generate_tokens(
        &saved_user,
        &role.name,
        &crate::CONFIG.jwt_secret,
        crate::CONFIG.jwt_access_exp.as_secs(),
        crate::CONFIG.jwt_refresh_exp.as_secs(),
    )?;

    // Create session
    let session = user_session::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(saved_user.id),
        refresh_token_hash: Set(bcrypt::hash(&refresh_token, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash refresh token: {}", e)))?),
        user_agent: Set(None),
        ip_address: Set(None),
        is_revoked: Set(false),
        expires_at: Set(
            now + chrono::Duration::seconds(crate::CONFIG.jwt_refresh_exp.as_secs() as i64)
        ),
        created_at: Set(now),
    };
    session.insert(db).await?;

    info!("User registered: {}", saved_user.username);

    Ok(AuthResponse {
        user: UserResponse {
            id: saved_user.id,
            username: saved_user.username,
            email: saved_user.email,
            display_name: saved_user.display_name,
            avatar_url: None,
            is_active: saved_user.is_active,
            role: to_role_response(&role),
            last_login_at: None,
            created_at: saved_user.created_at,
        },
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Login with username/email and password
pub async fn login(db: &DatabaseConnection, req: LoginRequest) -> Result<AuthResponse, AppError> {
    // Find user by username or email
    let user = if let Some(username) = &req.username {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(db)
            .await?
    } else if let Some(email) = &req.email {
        user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(db)
            .await?
    } else {
        return Err(AppError::BadRequest("Username or email is required".into()));
    };

    let user = user.ok_or(AppError::InvalidCredentials)?;

    if !user.is_active {
        return Err(AppError::Forbidden("Account is disabled".into()));
    }

    // Verify password
    let valid = verify_password(&req.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    // Get role
    let role = role::Entity::find()
        .filter(role::Column::Id.eq(user.role_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Internal("User role not found".into()))?;

    // Generate tokens
    let (access_token, refresh_token, expires_in) = generate_tokens(
        &user,
        &role.name,
        &crate::CONFIG.jwt_secret,
        crate::CONFIG.jwt_access_exp.as_secs(),
        crate::CONFIG.jwt_refresh_exp.as_secs(),
    )?;

    // Update last login
    let now = chrono::Utc::now();
    let mut user_active: user::ActiveModel = user.clone().into();
    user_active.last_login_at = Set(Some(now));
    user_active.updated_at = Set(now);
    let saved_user = user_active.update(db).await?;

    // Create session
    let session = user_session::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user.id),
        refresh_token_hash: Set(bcrypt::hash(&refresh_token, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash refresh token: {}", e)))?),
        user_agent: Set(None),
        ip_address: Set(None),
        is_revoked: Set(false),
        expires_at: Set(
            now + chrono::Duration::seconds(crate::CONFIG.jwt_refresh_exp.as_secs() as i64)
        ),
        created_at: Set(now),
    };
    session.insert(db).await?;

    info!("User logged in: {}", saved_user.username);

    Ok(AuthResponse {
        user: UserResponse {
            id: saved_user.id,
            username: saved_user.username,
            email: saved_user.email,
            display_name: saved_user.display_name,
            avatar_url: None,
            is_active: saved_user.is_active,
            role: to_role_response(&role),
            last_login_at: saved_user.last_login_at,
            created_at: saved_user.created_at,
        },
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Refresh access token
pub async fn refresh_token(
    db: &DatabaseConnection,
    refresh_token_str: &str,
    jwt_secret: &str,
) -> Result<TokenResponse, AppError> {
    // Validate the refresh token
    let claims = validate_token(refresh_token_str, jwt_secret)?;

    if claims.token_type != "refresh" {
        return Err(AppError::TokenInvalid(
            "Invalid token type, expected refresh token".into(),
        ));
    }

    // Find user
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::TokenInvalid("Invalid user id in token".into()))?;

    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(AppError::TokenInvalid("User not found".into()))?;

    if !user.is_active {
        return Err(AppError::Forbidden("Account is disabled".into()));
    }

    // Get role
    let role = role::Entity::find()
        .filter(role::Column::Id.eq(user.role_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::Internal("User role not found".into()))?;

    // Generate new tokens
    let (access_token, new_refresh_token, expires_in) = generate_tokens(
        &user,
        &role.name,
        jwt_secret,
        crate::CONFIG.jwt_access_exp.as_secs(),
        crate::CONFIG.jwt_refresh_exp.as_secs(),
    )?;

    // Revoke old session and create new one
    user_session::Entity::delete_many()
        .filter(
            user_session::Column::RefreshTokenHash.eq(&bcrypt::hash(
                refresh_token_str,
                bcrypt::DEFAULT_COST,
            )
            .map_err(|e| AppError::Internal(format!("Failed to hash refresh token: {}", e)))?),
        )
        .exec(db)
        .await?;

    let now = chrono::Utc::now();
    let session = user_session::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user.id),
        refresh_token_hash: Set(bcrypt::hash(&new_refresh_token, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash refresh token: {}", e)))?),
        user_agent: Set(None),
        ip_address: Set(None),
        is_revoked: Set(false),
        expires_at: Set(
            now + chrono::Duration::seconds(crate::CONFIG.jwt_refresh_exp.as_secs() as i64)
        ),
        created_at: Set(now),
    };
    session.insert(db).await?;

    info!("Token refreshed for user: {}", user.username);

    Ok(TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in,
    })
}

/// Logout - revoke all sessions for the user
pub async fn logout(db: &DatabaseConnection, user_id: Uuid) -> Result<(), AppError> {
    user_session::Entity::delete_many()
        .filter(user_session::Column::UserId.eq(user_id))
        .filter(user_session::Column::IsRevoked.eq(false))
        .exec(db)
        .await?;

    info!("User logged out: {}", user_id);
    Ok(())
}

pub fn to_role_response(role: &role::Model) -> crate::models::schemas::RoleResponse {
    use crate::models::schemas::RoleResponse;
    RoleResponse {
        id: role.id,
        name: role.name.clone(),
        display_name: role.display_name.clone(),
        description: role.description.clone(),
        is_system: role.is_system,
        created_at: role.created_at,
    }
}

pub fn to_user_response(user: &user::Model, role: &role::Model) -> UserResponse {
    UserResponse {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        avatar_url: None,
        is_active: user.is_active,
        role: to_role_response(role),
        last_login_at: user.last_login_at,
        created_at: user.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::schemas::JwtClaims;
    use jsonwebtoken::{DecodingKey, Validation, decode};
    use uuid::Uuid;

    const TEST_JWT_SECRET: &str = "test-secret-key-for-unit-tests";

    fn make_test_user() -> user::Model {
        user::Model {
            id: Uuid::new_v4(),
            username: "testuser".into(),
            email: "test@example.com".into(),
            password_hash: String::new(),
            display_name: Some("Test User".into()),
            avatar_url: None,
            is_active: true,
            role_id: Uuid::new_v4(),
            last_login_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_test_role() -> role::Model {
        role::Model {
            id: Uuid::new_v4(),
            name: "user".into(),
            display_name: "User".into(),
            description: Some("Standard user role".into()),
            is_system: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_hash_password_roundtrip() {
        // SAFETY: Test fixture only — never used in production code paths.
        const TEST_PASSWORD: &str = "test-pwd-fixture-001";
        let password = TEST_PASSWORD;
        let hash = hash_password(password).unwrap();
        assert!(hash.starts_with("$2b$") || hash.starts_with("$2a$"));
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_too_long() {
        let long_pwd = "a".repeat(100);
        let result = hash_password(&long_pwd);
        assert!(result.is_ok(), "bcrypt should handle long passwords");
    }

    #[test]
    fn test_generate_tokens_produces_valid_jwt() {
        let user = make_test_user();
        let role_name = "admin";
        let (access_token, refresh_token, expires_in) =
            generate_tokens(&user, role_name, TEST_JWT_SECRET, 900, 604800).unwrap();

        assert_eq!(expires_in, 900);

        // Verify access token
        let access_data = decode::<JwtClaims>(
            &access_token,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(access_data.claims.sub, user.id.to_string());
        assert_eq!(access_data.claims.username, "testuser");
        assert_eq!(access_data.claims.role, "admin");
        assert_eq!(access_data.claims.token_type, "access");

        // Verify refresh token
        let refresh_data = decode::<JwtClaims>(
            &refresh_token,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(refresh_data.claims.token_type, "refresh");
        assert!(refresh_data.claims.exp > access_data.claims.exp);
    }

    #[test]
    fn test_generate_tokens_unique_jti() {
        let user = make_test_user();
        let (token1, _, _) = generate_tokens(&user, "user", TEST_JWT_SECRET, 900, 604800).unwrap();
        let (token2, _, _) = generate_tokens(&user, "user", TEST_JWT_SECRET, 900, 604800).unwrap();

        let claims1 = decode::<JwtClaims>(
            &token1,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        let claims2 = decode::<JwtClaims>(
            &token2,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .unwrap();

        assert_ne!(claims1.claims.jti, claims2.claims.jti);
    }

    #[test]
    fn test_validate_token_valid() {
        let user = make_test_user();
        let (token, _, _) = generate_tokens(&user, "user", TEST_JWT_SECRET, 900, 604800).unwrap();

        let claims = validate_token(&token, TEST_JWT_SECRET).unwrap();
        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let user = make_test_user();
        let (token, _, _) = generate_tokens(&user, "user", TEST_JWT_SECRET, 900, 604800).unwrap();

        let result = validate_token(&token, "different-secret");
        assert!(result.is_err());
        match result {
            Err(AppError::TokenInvalid(_)) => {} // expected
            other => panic!("Expected TokenInvalid, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_token_malformed() {
        let result = validate_token("not.a.token", TEST_JWT_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_token_expired() {
        let user = make_test_user();
        // Generate with 0 expiry to get an already-expired token
        // We need to use the raw jsonwebtoken to create an expired token
        let now = chrono::Utc::now().timestamp() as usize;
        let expired_claims = JwtClaims {
            sub: user.id.to_string(),
            username: "testuser".into(),
            role: "user".into(),
            exp: now - 3600, // expired 1 hour ago
            iat: now - 7200,
            jti: Uuid::new_v4().to_string(),
            token_type: "access".to_string(),
        };
        let token = encode(
            &Header::default(),
            &expired_claims,
            &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
        )
        .unwrap();

        let result = validate_token(&token, TEST_JWT_SECRET);
        assert!(result.is_err());
        match result {
            Err(AppError::TokenExpired) => {} // expected
            other => panic!("Expected TokenExpired, got {:?}", other),
        }
    }

    #[test]
    fn test_to_role_response() {
        let role = make_test_role();
        let resp = to_role_response(&role);
        assert_eq!(resp.name, "user");
        assert_eq!(resp.display_name, "User");
        assert!(resp.is_system);
    }

    #[test]
    fn test_to_user_response() {
        let user = make_test_user();
        let role = make_test_role();
        let resp = to_user_response(&user, &role);
        assert_eq!(resp.username, "testuser");
        assert_eq!(resp.email, "test@example.com");
        assert_eq!(resp.role.name, "user");
        assert!(resp.is_active);
    }
}
