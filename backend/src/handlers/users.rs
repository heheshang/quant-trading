use crate::db::{role, user};
use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    AdminUpdateUserRequest, ChangePasswordRequest, PaginatedResponse, PaginationParams,
    UpdateUserRequest,
};
use crate::services::auth;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

/// Require the authenticated user to have admin role
fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Admin privileges required".into()));
    }
    Ok(())
}

/// List all users (admin only)
pub async fn list_users(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    require_admin(&user)?;
    let page = params.page();
    let size = params.size();
    let offset = params.offset();

    let total = user::Entity::find().count(&*db).await? as u64;

    let users = user::Entity::find()
        .order_by_desc(user::Column::CreatedAt)
        .offset(offset)
        .limit(size)
        .all(&*db)
        .await?;

    let mut user_list = Vec::new();
    for u in users {
        let role = role::Entity::find_by_id(u.role_id)
            .one(&*db)
            .await?
            .ok_or_else(|| AppError::Internal("Role not found".into()))?;

        user_list.push(serde_json::json!({
            "id": u.id,
            "username": u.username,
            "email": u.email,
            "display_name": u.display_name,
            "is_active": u.is_active,
            "role": {
                "id": role.id,
                "name": role.name,
                "display_name": role.display_name,
            },
            "last_login_at": u.last_login_at,
            "created_at": u.created_at,
        }));
    }

    Ok(Json(ApiResponse::success(PaginatedResponse {
        items: user_list,
        total,
        page,
        size,
    })))
}

/// Get current user profile
pub async fn get_me(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user_model = user::Entity::find_by_id(user.user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let role = role::Entity::find_by_id(user_model.role_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::Internal("Role not found".into()))?;

    let resp = serde_json::json!({
        "id": user_model.id,
        "username": user_model.username,
        "email": user_model.email,
        "display_name": user_model.display_name,
        "avatar_url": user_model.avatar_url,
        "is_active": user_model.is_active,
        "role": {
            "id": role.id,
            "name": role.name,
            "display_name": role.display_name,
        },
        "last_login_at": user_model.last_login_at,
        "created_at": user_model.created_at,
    });

    Ok(Json(ApiResponse::success(resp)))
}

/// Update current user profile
pub async fn update_me(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user_model = user::Entity::find_by_id(user.user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let mut active: user::ActiveModel = user_model.clone().into();
    active.updated_at = sea_orm::Set(chrono::Utc::now());

    if let Some(display_name) = body.display_name {
        if !display_name.is_empty() {
            active.display_name = sea_orm::Set(Some(display_name));
        }
    }
    if let Some(email) = body.email {
        if !email.is_empty() {
            // Check email uniqueness
            let existing = user::Entity::find()
                .filter(user::Column::Email.eq(&email))
                .filter(user::Column::Id.ne(user.user_id))
                .one(&*db)
                .await?;
            if existing.is_some() {
                return Err(AppError::Conflict("Email already in use".into()));
            }
            active.email = sea_orm::Set(email);
        }
    }
    if let Some(avatar_url) = body.avatar_url {
        active.avatar_url = sea_orm::Set(Some(avatar_url));
    }

    let updated = active.update(&*db).await?;
    let role = role::Entity::find_by_id(updated.role_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::Internal("Role not found".into()))?;

    let resp = serde_json::json!({
        "id": updated.id,
        "username": updated.username,
        "email": updated.email,
        "display_name": updated.display_name,
        "avatar_url": updated.avatar_url,
        "is_active": updated.is_active,
        "role": {
            "id": role.id,
            "name": role.name,
            "display_name": role.display_name,
        },
        "last_login_at": updated.last_login_at,
        "created_at": updated.created_at,
    });

    Ok(Json(ApiResponse::success(resp)))
}

/// Change current user password
pub async fn change_password(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user_model = user::Entity::find_by_id(user.user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // Verify old password
    let valid = auth::verify_password(&body.old_password, &user_model.password_hash)?;
    if !valid {
        return Err(AppError::BadRequest("Current password is incorrect".into()));
    }

    if body.new_password.len() < 8 {
        return Err(AppError::Validation(
            "New password must be at least 8 characters".into(),
        ));
    }

    let new_hash = auth::hash_password(&body.new_password)?;

    let mut active: user::ActiveModel = user_model.into();
    active.password_hash = sea_orm::Set(new_hash);
    active.updated_at = sea_orm::Set(chrono::Utc::now());
    active.update(&*db).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Password changed successfully"
    }))))
}

/// Admin: update user (including role)
pub async fn admin_update_user(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AdminUpdateUserRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_admin(&user)?;
    let user_model = user::Entity::find_by_id(user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let mut active: user::ActiveModel = user_model.clone().into();
    active.updated_at = sea_orm::Set(chrono::Utc::now());

    if let Some(display_name) = body.display_name {
        if !display_name.is_empty() {
            active.display_name = sea_orm::Set(Some(display_name));
        }
    }
    if let Some(email) = body.email {
        if !email.is_empty() {
            let existing = user::Entity::find()
                .filter(user::Column::Email.eq(&email))
                .filter(user::Column::Id.ne(user_id))
                .one(&*db)
                .await?;
            if existing.is_some() {
                return Err(AppError::Conflict("Email already in use".into()));
            }
            active.email = sea_orm::Set(email);
        }
    }
    if let Some(avatar_url) = body.avatar_url {
        active.avatar_url = sea_orm::Set(Some(avatar_url));
    }
    if let Some(is_active) = body.is_active {
        active.is_active = sea_orm::Set(is_active);
    }
    if let Some(role_id) = body.role_id {
        // Verify role exists
        let role = role::Entity::find_by_id(role_id).one(&*db).await?;
        if role.is_none() {
            return Err(AppError::NotFound("Role not found".into()));
        }
        active.role_id = sea_orm::Set(role_id);
    }

    let updated = active.update(&*db).await?;
    let role = role::Entity::find_by_id(updated.role_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::Internal("Role not found".into()))?;

    let resp = serde_json::json!({
        "id": updated.id,
        "username": updated.username,
        "email": updated.email,
        "display_name": updated.display_name,
        "is_active": updated.is_active,
        "role": {
            "id": role.id,
            "name": role.name,
            "display_name": role.display_name,
        },
    });

    Ok(Json(ApiResponse::success(resp)))
}

/// Admin: delete user
pub async fn admin_delete_user(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_admin(&user)?;

    let user_model = user::Entity::find_by_id(user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // Delete sessions first, then the user
    use sea_orm::EntityTrait as _;
    crate::db::user_session::Entity::delete_many()
        .filter(crate::db::user_session::Column::UserId.eq(user_id))
        .exec(&*db)
        .await?;

    user_model.delete(&*db).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "User deleted successfully"
    }))))
}

/// List all roles
pub async fn list_roles(
    _user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let roles = role::Entity::find()
        .order_by_asc(role::Column::Name)
        .all(&*db)
        .await?;

    let role_list: Vec<serde_json::Value> = roles
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "display_name": r.display_name,
                "description": r.description,
                "is_system": r.is_system,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(role_list)))
}
